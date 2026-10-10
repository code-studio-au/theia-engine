//! One immutable cooked mesh buffer and a bounded, one-time staging upload.
use crate::Result;
use ash::vk;
use std::{fs, io::Read, path::Path, sync::Arc};
use theia_assets::{DEFAULT_BUDGET, Mesh};
pub fn select(requested: &Path, fallback: Option<&Path>, budget: usize) -> Result<Vec<u8>> {
    let primary = load(requested)?;
    if primary.len() <= budget {
        return Ok(primary);
    }
    let fallback = fallback
        .ok_or("Requested cooked mesh exceeds encoded-byte budget and no fallback was supplied")?;
    let coarse = load(fallback)?;
    if coarse.len() > budget {
        return Err("Fallback cooked mesh also exceeds encoded-byte budget".into());
    }
    let primary_mesh = Mesh::decode(&primary, None, DEFAULT_BUDGET)?;
    let coarse_mesh = Mesh::decode(&coarse, None, DEFAULT_BUDGET)?;
    let producer: serde_json::Value = serde_json::from_str(&primary_mesh.provenance)?;
    let fallback_producer: serde_json::Value = serde_json::from_str(&coarse_mesh.provenance)?;
    let source = producer
        .get("source_sha256")
        .and_then(|value| value.as_str())
        .ok_or("Requested asset has no source digest")?;
    if source.len() != 64 || !source.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Requested asset source digest is not SHA-256 hex".into());
    }
    if fallback_producer
        .get("fallback_for_source_sha256")
        .and_then(|value| value.as_str())
        != Some(source)
    {
        return Err("Fallback source link does not match requested asset".into());
    }
    let bounds = |mesh: &Mesh| {
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        for vertex in &mesh.positions {
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(vertex[axis]);
                maximum[axis] = maximum[axis].max(vertex[axis]);
            }
        }
        (minimum, maximum)
    };
    if bounds(&primary_mesh) != bounds(&coarse_mesh) {
        return Err("Fallback spatial bounds differ from requested asset".into());
    }
    println!(
        "Explicit capacity fallback: requested {} encoded bytes exceeds budget {budget}; linked coarse source {} bytes selected",
        primary.len(),
        coarse.len()
    );
    Ok(coarse)
}
pub fn load(path: &Path) -> Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > DEFAULT_BUDGET as u64 {
        return Err("Cooked mesh exceeds encoded-byte qualification budget".into());
    }
    let mut bytes = Vec::new();
    file.take(DEFAULT_BUDGET as u64 + 1)
        .read_to_end(&mut bytes)?;
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or("Asset needs a UTF-8 content-address filename")?;
    if name.len() != 64 || !name.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Cooked asset filename must be its 64-digit SHA-256 content address".into());
    }
    let mut digest = [0u8; 32];
    for (i, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&name[i * 2..i * 2 + 2], 16)?;
    }
    let mesh = Mesh::decode(&bytes, Some(digest), DEFAULT_BUDGET)?;
    if mesh.meshlets.len() > 32 {
        return Err("Meshlet viewer qualification supports at most 32 meshlets".into());
    }
    println!(
        "Cooked asset verified: {} bytes, {} vertices, {} meshlets, {} triangles; content address {name}",
        bytes.len(),
        mesh.positions.len(),
        mesh.meshlets.len(),
        mesh.triangle_count()
    );
    Ok(bytes)
}
struct Allocation {
    device: ash::Device,
    handle: vk::Buffer,
    memory: vk::DeviceMemory,
}
pub fn load_material(path: &Path, mesh: &Mesh) -> Result<Vec<u8>> {
    let budget = theia_assets::material::DEFAULT_BUDGET;
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(budget as u64 + 1)
        .read_to_end(&mut bytes)?;
    let name = path
        .file_stem()
        .and_then(|v| v.to_str())
        .ok_or("Material needs a content address filename")?;
    let actual = theia_assets::content_id(&bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect::<String>();
    if name != actual {
        return Err("Material content address filename mismatch".into());
    }
    theia_assets::material::MaterialPage::decode(&bytes, None, mesh, budget)?;
    Ok(bytes)
}
pub struct Buffer {
    _allocation: Arc<Allocation>,
    _lease: Option<theia_assets::arena::Lease>,
    pub offset: u64,
    pub handle: vk::Buffer,
    memory: vk::DeviceMemory,
    pub bytes: u64,
    pub allocated_bytes: u64,
    pub reserved_bytes: u64,
}
#[derive(Debug)]
pub struct AllocationBudgetExceeded;
impl std::fmt::Display for AllocationBudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Cooked geometry allocation or range budget exhausted")
    }
}
impl std::error::Error for AllocationBudgetExceeded {}
/// One fixed device-local allocation. Page ranges are leased until safe retirement.
pub struct GeometryArena {
    backing: Buffer,
    ranges: theia_assets::arena::Arena,
}
impl GeometryArena {
    pub fn new(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        budget: u64,
        alignment: u64,
    ) -> Result<Self> {
        let backing = Buffer::allocate(
            device,
            memory,
            budget,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            budget,
        )?;
        let ranges = theia_assets::arena::Arena::new(budget, alignment)?;
        println!(
            "Fixed geometry arena: {} allocation bytes, {} logical bytes, descriptor offset alignment {alignment}",
            backing.allocated_bytes, backing.bytes
        );
        Ok(Self { backing, ranges })
    }
    pub fn live_bytes(&self) -> Result<u64> {
        Ok(self.ranges.live_bytes()?)
    }
    pub fn allocated_bytes(&self) -> u64 {
        self.backing.allocated_bytes
    }
    fn page(&self, bytes: u64) -> Result<Buffer> {
        let lease = self
            .ranges
            .reserve(bytes)
            .map_err(|error| -> Box<dyn std::error::Error> {
                if error.0 == "arena has no contiguous aligned range" {
                    AllocationBudgetExceeded.into()
                } else {
                    error.into()
                }
            })?;
        Ok(Buffer {
            _allocation: self.backing._allocation.clone(),
            handle: self.backing.handle,
            memory: self.backing.memory,
            bytes,
            offset: lease.offset(),
            allocated_bytes: self.backing.allocated_bytes,
            reserved_bytes: lease.reserved(),
            _lease: Some(lease),
        })
    }
}
impl Buffer {
    fn allocate(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        bytes: u64,
        usage: vk::BufferUsageFlags,
        flags: vk::MemoryPropertyFlags,
        budget: u64,
    ) -> Result<Self> {
        let mut allocation = Allocation {
            device: device.clone(),
            handle: vk::Buffer::null(),
            memory: vk::DeviceMemory::null(),
        };
        let allocated_bytes;
        // SAFETY: Live device/create-info storage. Allocation immediately owns each created handle;
        // failed construction drops this guard, releasing buffer before its backing memory.
        unsafe {
            allocation.handle = device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(bytes)
                    .usage(usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )?;
            let requirements = device.get_buffer_memory_requirements(allocation.handle);
            if requirements.size > budget {
                return Err(AllocationBudgetExceeded.into());
            }
            allocated_bytes = requirements.size;
            let index = (0..memory.memory_type_count)
                .find(|&i| {
                    requirements.memory_type_bits & (1 << i) != 0
                        && memory.memory_types[i as usize]
                            .property_flags
                            .contains(flags)
                })
                .ok_or("No memory type for cooked asset buffer")?;
            allocation.memory = device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(index),
                None,
            )?;
            device.bind_buffer_memory(allocation.handle, allocation.memory, 0)?;
        }
        Ok(Self {
            handle: allocation.handle,
            memory: allocation.memory,
            _allocation: Arc::new(allocation),
            _lease: None,
            offset: 0,
            bytes,
            allocated_bytes,
            reserved_bytes: allocated_bytes,
        })
    }
    pub fn upload(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        bytes: &[u8],
        family: u32,
        queue: vk::Queue,
        resident_budget: u64,
        arena: Option<&GeometryArena>,
    ) -> Result<Self> {
        Upload::begin(device, memory, bytes, family, queue, resident_budget, arena)?.finish()
    }
}

/// Owns pending copy work and both buffers until its dedicated upload fence retires.
/// No queue/device idle wait is used on the successful path.
pub struct Upload {
    // Fields drop in this order: submitted work must retire before its buffers.
    transfer: Transfer,
    staging: Buffer,
    destination: Option<Buffer>,
}
impl Upload {
    pub fn begin(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        bytes: &[u8],
        family: u32,
        queue: vk::Queue,
        resident_budget: u64,
        arena: Option<&GeometryArena>,
    ) -> Result<Self> {
        let destination = if let Some(arena) = arena {
            arena.page(bytes.len() as u64)?
        } else {
            Buffer::allocate(
                device,
                memory,
                bytes.len() as u64,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                resident_budget,
            )?
        };
        let staging = Buffer::allocate(
            device,
            memory,
            bytes.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            DEFAULT_BUDGET as u64,
        )?;
        let mut upload = Self {
            transfer: Transfer {
                device: device.clone(),
                pool: vk::CommandPool::null(),
                fence: vk::Fence::null(),
                submitted: false,
            },
            staging,
            destination: Some(destination),
        };
        let destination = upload.destination.as_ref().unwrap();
        // SAFETY: Upload owns all referenced objects. Host mapping is coherent and bounded.
        // The transfer guard waits the dedicated fence before destroying commands or buffers.
        unsafe {
            let pointer = device.map_memory(
                upload.staging.memory,
                0,
                upload.staging.bytes,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len());
            device.unmap_memory(upload.staging.memory);
            upload.transfer.pool = device.create_command_pool(
                &vk::CommandPoolCreateInfo::default().queue_family_index(family),
                None,
            )?;
            upload.transfer.fence = device.create_fence(&vk::FenceCreateInfo::default(), None)?;
            let command = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(upload.transfer.pool)
                    .command_buffer_count(1),
            )?[0];
            device.begin_command_buffer(
                command,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            device.cmd_copy_buffer(
                command,
                upload.staging.handle,
                destination.handle,
                &[vk::BufferCopy::default()
                    .dst_offset(destination.offset)
                    .size(destination.bytes)],
            );
            let publish = [vk::BufferMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(
                    vk::PipelineStageFlags2::COMPUTE_SHADER
                        | vk::PipelineStageFlags2::MESH_SHADER_EXT
                        | vk::PipelineStageFlags2::FRAGMENT_SHADER,
                )
                .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(destination.handle)
                .offset(destination.offset)
                .size(destination.bytes)];
            device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().buffer_memory_barriers(&publish),
            );
            device.end_command_buffer(command)?;
            let commands = [vk::CommandBufferSubmitInfo::default().command_buffer(command)];
            device.queue_submit2(
                queue,
                &[vk::SubmitInfo2::default().command_buffer_infos(&commands)],
                upload.transfer.fence,
            )?;
            upload.transfer.submitted = true;
        }
        Ok(upload)
    }
    pub fn ready(&self) -> Result<bool> {
        // SAFETY: Fence is live and belongs to this upload's retained device.
        Ok(unsafe { self.transfer.device.get_fence_status(self.transfer.fence)? })
    }
    pub fn resident_bytes(&self) -> u64 {
        self.destination.as_ref().unwrap().reserved_bytes
    }
    pub fn staging_bytes(&self) -> u64 {
        self.staging.allocated_bytes
    }
    pub fn finish(mut self) -> Result<Buffer> {
        // SAFETY: Fence tracks this upload only and owns all submitted resources.
        unsafe {
            self.transfer
                .device
                .wait_for_fences(&[self.transfer.fence], true, u64::MAX)?;
        }
        self.transfer.submitted = false;
        let destination = self.destination.take().unwrap();
        println!(
            "Immutable GPU mesh upload complete: {} encoded bytes, offset {}, {} charged range bytes in {} backing allocation bytes, {} staging allocation bytes; dedicated fence retired",
            destination.bytes,
            destination.offset,
            destination.reserved_bytes,
            destination.allocated_bytes,
            self.staging.allocated_bytes
        );
        Ok(destination)
    }
}
struct Transfer {
    device: ash::Device,
    pool: vk::CommandPool,
    fence: vk::Fence,
    submitted: bool,
}
impl Drop for Transfer {
    fn drop(&mut self) {
        // SAFETY: This guard drops before staging/destination buffers. After device loss pending
        // work cannot continue; other unexpected fence errors get a final device-retirement attempt.
        unsafe {
            if self.submitted
                && let Err(error) = self.device.wait_for_fences(&[self.fence], true, u64::MAX)
            {
                eprintln!("Upload fence retirement failed: {error}");
                let _ = self.device.device_wait_idle();
            }
            if self.fence != vk::Fence::null() {
                self.device.destroy_fence(self.fence, None);
            }
            if self.pool != vk::CommandPool::null() {
                self.device.destroy_command_pool(self.pool, None);
            }
        }
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        // SAFETY: Transfer guard/work owner has retired uses; parent device remains live.
        unsafe {
            if self.handle != vk::Buffer::null() {
                self.device.destroy_buffer(self.handle, None);
            }
            if self.memory != vk::DeviceMemory::null() {
                self.device.free_memory(self.memory, None);
            }
        }
    }
}
