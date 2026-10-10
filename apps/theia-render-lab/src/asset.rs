//! One immutable cooked mesh buffer and a bounded, one-time staging upload.
use crate::Result;
use ash::vk;
use std::{fs, io::Read, path::Path};
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
pub struct Buffer {
    device: ash::Device,
    pub handle: vk::Buffer,
    memory: vk::DeviceMemory,
    pub bytes: u64,
}
impl Buffer {
    fn allocate(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        bytes: u64,
        usage: vk::BufferUsageFlags,
        flags: vk::MemoryPropertyFlags,
    ) -> Result<Self> {
        let mut buffer = Self {
            device: device.clone(),
            handle: vk::Buffer::null(),
            memory: vk::DeviceMemory::null(),
            bytes,
        };
        // SAFETY: Device/create-info storage is live. Every created object immediately gains RAII ownership.
        unsafe {
            buffer.handle = device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(bytes)
                    .usage(usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )?;
            let requirements = device.get_buffer_memory_requirements(buffer.handle);
            let index = (0..memory.memory_type_count)
                .find(|&i| {
                    requirements.memory_type_bits & (1 << i) != 0
                        && memory.memory_types[i as usize]
                            .property_flags
                            .contains(flags)
                })
                .ok_or("No memory type for cooked asset buffer")?;
            buffer.memory = device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(index),
                None,
            )?;
            device.bind_buffer_memory(buffer.handle, buffer.memory, 0)?;
        }
        Ok(buffer)
    }
    pub fn upload(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        bytes: &[u8],
        family: u32,
        queue: vk::Queue,
    ) -> Result<Self> {
        let destination = Self::allocate(
            device,
            memory,
            bytes.len() as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        let staging = Self::allocate(
            device,
            memory,
            bytes.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let mut transfer = Transfer {
            device: device.clone(),
            pool: vk::CommandPool::null(),
        };
        // SAFETY: Mapped staging allocation covers exactly bytes; one startup submission owns all
        // resources. Transfer guard retires submitted work before command/staging destruction on errors.
        unsafe {
            let pointer = device.map_memory(
                staging.memory,
                0,
                staging.bytes,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len());
            device.unmap_memory(staging.memory);
            transfer.pool = device.create_command_pool(
                &vk::CommandPoolCreateInfo::default().queue_family_index(family),
                None,
            )?;
            let command = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(transfer.pool)
                    .command_buffer_count(1),
            )?[0];
            device.begin_command_buffer(
                command,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            device.cmd_copy_buffer(
                command,
                staging.handle,
                destination.handle,
                &[vk::BufferCopy::default().size(destination.bytes)],
            );
            let publish = [vk::BufferMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(
                    vk::PipelineStageFlags2::COMPUTE_SHADER
                        | vk::PipelineStageFlags2::MESH_SHADER_EXT,
                )
                .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(destination.handle)
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
                vk::Fence::null(),
            )?;
            device.queue_wait_idle(queue)?;
        }
        println!(
            "Immutable GPU mesh uploaded once: {} bytes; no runtime native-library dependency",
            destination.bytes
        );
        Ok(destination)
    }
}
struct Transfer {
    device: ash::Device,
    pool: vk::CommandPool,
}
impl Drop for Transfer {
    fn drop(&mut self) {
        // SAFETY: Guard outlives submitted command/staging uses; queue failure still needs ordered cleanup.
        unsafe {
            let _ = self.device.device_wait_idle();
            if self.pool != vk::CommandPool::null() {
                self.device.destroy_command_pool(self.pool, None);
            }
        }
    }
}
impl Drop for Buffer {
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
