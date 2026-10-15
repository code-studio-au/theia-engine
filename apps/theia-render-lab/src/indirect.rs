//! One device-local work buffer, generated and consumed without host readback.
use crate::{RenderMode, Result, pipeline::ShaderModule};
use ash::vk;
use std::path::Path;
const BUFFER_BYTES: vk::DeviceSize = 64;
const COUNT_OFFSET: vk::DeviceSize = 12;
const COMMAND_STRIDE: u32 = 12;

#[derive(Clone, Copy)]
pub struct Lod {
    pub bounds: [[f32; 3]; 2],
    pub error: f32,
    pub pixels: f32,
    pub extent: [u32; 2],
    pub entry_fraction: f32,
    pub reset: bool,
    pub test_trajectory: bool,
}
pub struct IndirectWork {
    device: ash::Device,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    pub descriptor_layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    set: vk::DescriptorSet,
    compute_layout: vk::PipelineLayout,
    compute: vk::Pipeline,
    bytes: vk::DeviceSize,
    pub allocated_bytes: u64,
    asset: Option<crate::asset::Buffer>,
    coarse: Option<crate::asset::Buffer>,
    lod: bool,
    cull: bool,
}
enum GeometryInput<'a> {
    Encoded(&'a [u8], Option<&'a crate::asset::GeometryArena>),
    Resident(crate::asset::Buffer),
}
impl IndirectWork {
    pub fn new(
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        shader_dir: &Path,
        mode: RenderMode,
        asset: Option<(&[u8], Option<&crate::asset::GeometryArena>)>,
        queue: (u32, vk::Queue, u64),
        coarse: Option<&[u8]>,
    ) -> Result<Self> {
        Self::configured(
            device,
            memory_properties,
            shader_dir,
            mode,
            asset.map(|(bytes, arena)| GeometryInput::Encoded(bytes, arena)),
            queue,
            coarse,
        )
    }
    pub fn resident(
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        shader_dir: &Path,
        geometry: crate::asset::Buffer,
    ) -> Result<Self> {
        Self::configured(
            device,
            memory_properties,
            shader_dir,
            RenderMode::AssetResolve,
            Some(GeometryInput::Resident(geometry)),
            (0, vk::Queue::null(), 0),
            None,
        )
    }
    fn configured(
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        shader_dir: &Path,
        mode: RenderMode,
        asset: Option<GeometryInput<'_>>,
        queue: (u32, vk::Queue, u64),
        coarse: Option<&[u8]>,
    ) -> Result<Self> {
        let has_asset = asset.is_some();
        let spatial = mode == RenderMode::Spatial;
        let lod = mode == RenderMode::AssetLod;
        if lod && (coarse.is_none() || !has_asset) {
            return Err("GPU LOD requires both geometry buffers".into());
        }
        let cull = matches!(
            mode,
            RenderMode::AssetCull | RenderMode::AssetLod | RenderMode::AssetResolve
        );
        let bytes = if mode == RenderMode::Benchmark {
            crate::benchmark::WORK_BYTES
        } else if spatial {
            9728
        } else if cull {
            256
        } else {
            BUFFER_BYTES
        };
        let shader = ShaderModule::load(
            device,
            &shader_dir.join(if mode == RenderMode::Benchmark {
                "benchmark.compute.spv"
            } else if lod {
                "asset.lod.compute.spv"
            } else if cull {
                "asset.cull.compute.spv"
            } else if has_asset {
                "asset.compute.spv"
            } else if spatial {
                "spatial.compute.spv"
            } else {
                "work.compute.spv"
            }),
        )?;
        let mut work = Self {
            device: device.clone(),
            buffer: vk::Buffer::null(),
            memory: vk::DeviceMemory::null(),
            descriptor_layout: vk::DescriptorSetLayout::null(),
            pool: vk::DescriptorPool::null(),
            set: vk::DescriptorSet::null(),
            compute_layout: vk::PipelineLayout::null(),
            compute: vk::Pipeline::null(),
            bytes,
            allocated_bytes: 0,
            asset: None,
            coarse: None,
            lod,
            cull,
        };
        // SAFETY: All create-info backing arrays remain live through calls. Every successful handle
        // immediately gains ownership in work; Drop handles partial initialization in dependency order.
        unsafe {
            work.buffer = device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(bytes)
                    .usage(
                        vk::BufferUsageFlags::STORAGE_BUFFER
                            | vk::BufferUsageFlags::INDIRECT_BUFFER
                            | vk::BufferUsageFlags::TRANSFER_DST,
                    )
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )?;
            let requirements = device.get_buffer_memory_requirements(work.buffer);
            work.allocated_bytes = requirements.size;
            let memory_type = (0..memory_properties.memory_type_count)
                .find(|&index| {
                    requirements.memory_type_bits & (1 << index) != 0
                        && memory_properties.memory_types[index as usize]
                            .property_flags
                            .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
                })
                .ok_or("No device-local memory type for work buffer")?;
            work.memory = device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(memory_type),
                None,
            )?;
            device.bind_buffer_memory(work.buffer, work.memory, 0)?;
            let mut bindings = vec![
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(
                        vk::ShaderStageFlags::COMPUTE
                            | vk::ShaderStageFlags::TASK_EXT
                            | if spatial || cull || mode == RenderMode::Benchmark {
                                vk::ShaderStageFlags::MESH_EXT
                            } else {
                                vk::ShaderStageFlags::empty()
                            },
                    ),
            ];
            if let Some(input) = asset {
                work.asset = Some(match input {
                    GeometryInput::Resident(buffer) => buffer,
                    GeometryInput::Encoded(bytes, arena) => crate::asset::Buffer::upload(
                        device,
                        memory_properties,
                        bytes,
                        queue.0,
                        queue.1,
                        queue.2,
                        arena,
                    )?,
                });
                bindings.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(1)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(
                            vk::ShaderStageFlags::COMPUTE | vk::ShaderStageFlags::MESH_EXT,
                        ),
                );
            }
            if let Some(bytes) = coarse {
                let remaining = queue
                    .2
                    .checked_sub(work.asset.as_ref().map_or(0, |v| v.reserved_bytes))
                    .ok_or(crate::asset::AllocationBudgetExceeded)?;
                work.coarse = Some(crate::asset::Buffer::upload(
                    device,
                    memory_properties,
                    bytes,
                    queue.0,
                    queue.1,
                    remaining,
                    None,
                )?);
                bindings.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(2)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(
                            vk::ShaderStageFlags::COMPUTE | vk::ShaderStageFlags::MESH_EXT,
                        ),
                );
            }
            work.descriptor_layout = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                None,
            )?;
            let sizes = [vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(if coarse.is_some() {
                    3
                } else if has_asset {
                    2
                } else {
                    1
                })];
            work.pool = device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .max_sets(1)
                    .pool_sizes(&sizes),
                None,
            )?;
            let layouts = [work.descriptor_layout];
            work.set = device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(work.pool)
                    .set_layouts(&layouts),
            )?[0];
            let buffers = [vk::DescriptorBufferInfo::default()
                .buffer(work.buffer)
                .offset(0)
                .range(bytes)];
            device.update_descriptor_sets(
                &[vk::WriteDescriptorSet::default()
                    .dst_set(work.set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&buffers)],
                &[],
            );
            if let Some(asset) = &work.asset {
                let info = [vk::DescriptorBufferInfo::default()
                    .buffer(asset.handle)
                    .offset(asset.offset)
                    .range(asset.bytes)];
                device.update_descriptor_sets(
                    &[vk::WriteDescriptorSet::default()
                        .dst_set(work.set)
                        .dst_binding(1)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(&info)],
                    &[],
                );
            }
            if let Some(coarse) = &work.coarse {
                let info = [vk::DescriptorBufferInfo::default()
                    .buffer(coarse.handle)
                    .offset(coarse.offset)
                    .range(coarse.bytes)];
                device.update_descriptor_sets(
                    &[vk::WriteDescriptorSet::default()
                        .dst_set(work.set)
                        .dst_binding(2)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(&info)],
                    &[],
                );
            }
            let push = [vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .offset(0)
                .size(if mode == RenderMode::Benchmark {
                    96
                } else if lod {
                    112
                } else if cull {
                    64
                } else {
                    4
                })];
            work.compute_layout = device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default()
                    .set_layouts(&layouts)
                    .push_constant_ranges(&push),
                None,
            )?;
            let stage = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(shader.handle)
                .name(c"main");
            match device.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[vk::ComputePipelineCreateInfo::default()
                    .stage(stage)
                    .layout(work.compute_layout)],
                None,
            ) {
                Ok(handles) => work.compute = handles[0],
                Err((handles, error)) => {
                    for handle in handles {
                        if handle != vk::Pipeline::null() {
                            device.destroy_pipeline(handle, None);
                        }
                    }
                    return Err(format!("Work-generation compute pipeline failed: {error}").into());
                }
            }
        }
        if mode == RenderMode::Benchmark {
            println!(
                "Benchmark dense GPU work initialized: {bytes} requested bytes; GPU motion/culling/detail and indirect counts; no per-object host draws"
            );
        } else if lod {
            println!(
                "GPU LOD work initialized: two fixed resident geometry bindings; selected page at 44, projected pixel bounds at 48, reason at 56; geometry physical allocation bytes {}; no selection/count readback",
                work.resident_bytes()
            );
        } else if cull {
            println!(
                "GPU meshlet frustum work initialized: 256-byte device-local command/count; visible count at 36, mask at 40, IDs at 64; no readback"
            );
        } else if has_asset {
            println!(
                "GPU cooked-mesh work initialized: 64-byte device-local command/count; meshlet count at 36; immutable geometry at binding 1; no count readback"
            );
        } else if spatial {
            println!(
                "GPU spatial work initialized: 9728-byte device-local CSR/visibility/work arena; no mapping/readback"
            );
        } else {
            println!(
                "GPU indirect work initialized: 64-byte device-local buffer; command at 0, count at 12, transforms/mesh count at 16..40; no mapping/readback"
            );
        }
        Ok(work)
    }
    pub fn generate(
        &self,
        command: vk::CommandBuffer,
        requested_items: u32,
        rows: Option<[[f32; 4]; 4]>,
        lod: Option<Lod>,
    ) {
        // SAFETY: Caller has begun recording a graphics+compute command buffer and waited its frame
        // fence. Work handles/descriptor set remain live; explicit barriers order reuse, compute writes,
        // indirect/task/mesh reads and the optional fragment HUD. Push constants are copied by Vulkan.
        unsafe {
            let reuse = [vk::BufferMemoryBarrier2::default()
                .src_stage_mask(
                    vk::PipelineStageFlags2::DRAW_INDIRECT
                        | vk::PipelineStageFlags2::TASK_SHADER_EXT
                        | vk::PipelineStageFlags2::MESH_SHADER_EXT
                        | vk::PipelineStageFlags2::FRAGMENT_SHADER
                        | vk::PipelineStageFlags2::COMPUTE_SHADER,
                )
                .src_access_mask(
                    vk::AccessFlags2::INDIRECT_COMMAND_READ
                        | vk::AccessFlags2::SHADER_STORAGE_READ
                        | vk::AccessFlags2::SHADER_STORAGE_WRITE,
                )
                .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .dst_access_mask(
                    vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
                )
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(self.buffer)
                .offset(0)
                .size(self.bytes)];
            self.device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().buffer_memory_barriers(&reuse),
            );
            self.device
                .cmd_bind_pipeline(command, vk::PipelineBindPoint::COMPUTE, self.compute);
            self.device.cmd_bind_descriptor_sets(
                command,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_layout,
                0,
                &[self.set],
                &[],
            );
            let mut camera_bytes = [0u8; 112];
            if self.cull {
                for (i, value) in rows
                    .expect("Culling requires camera rows")
                    .iter()
                    .flatten()
                    .enumerate()
                {
                    camera_bytes[i * 4..i * 4 + 4].copy_from_slice(&value.to_ne_bytes());
                }
            }
            if self.lod {
                let lod = lod.expect("LOD generation requires its bounded config");
                let values = [
                    lod.bounds[0][0],
                    lod.bounds[0][1],
                    lod.bounds[0][2],
                    lod.error,
                    lod.bounds[1][0],
                    lod.bounds[1][1],
                    lod.bounds[1][2],
                    lod.pixels,
                    lod.extent[0] as f32,
                    lod.extent[1] as f32,
                    lod.entry_fraction,
                    u32::from(lod.reset) as f32,
                ];
                for (i, value) in values.iter().enumerate() {
                    camera_bytes[64 + i * 4..68 + i * 4].copy_from_slice(&value.to_ne_bytes());
                }
            }
            let scenario_bytes = requested_items.to_ne_bytes();
            self.device.cmd_push_constants(
                command,
                self.compute_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                if self.lod {
                    &camera_bytes
                } else if self.cull {
                    &camera_bytes[..64]
                } else {
                    &scenario_bytes
                },
            );
            self.device.cmd_dispatch(command, 1, 1, 1);
            let publish = [vk::BufferMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
                .dst_stage_mask(
                    vk::PipelineStageFlags2::DRAW_INDIRECT
                        | vk::PipelineStageFlags2::TASK_SHADER_EXT
                        | vk::PipelineStageFlags2::MESH_SHADER_EXT
                        | vk::PipelineStageFlags2::FRAGMENT_SHADER,
                )
                .dst_access_mask(
                    vk::AccessFlags2::INDIRECT_COMMAND_READ | vk::AccessFlags2::SHADER_STORAGE_READ,
                )
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(self.buffer)
                .offset(0)
                .size(self.bytes)];
            self.device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().buffer_memory_barriers(&publish),
            );
        }
    }
    pub fn resident_bytes(&self) -> u64 {
        self.asset.as_ref().map_or(0, |asset| asset.reserved_bytes)
            + self.coarse.as_ref().map_or(0, |v| v.reserved_bytes)
    }
    pub fn benchmark_descriptor(&self) -> vk::DescriptorBufferInfo {
        vk::DescriptorBufferInfo::default()
            .buffer(self.buffer)
            .range(self.bytes)
    }
    pub fn generate_benchmark(&self, command: vk::CommandBuffer, push: &[u8; 96], objects: u32) {
        // SAFETY: Sole frame fence retired all readers; handles are owned through submission.
        // Header transfer precedes atomic compaction; publication covers indirect/task/mesh/resolve reads.
        unsafe {
            let barrier = vk::BufferMemoryBarrier2::default()
                .buffer(self.buffer)
                .size(self.bytes)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .src_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE);
            self.device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().buffer_memory_barriers(&[barrier]),
            );
            let header = [0u32, 1, 1, 1, objects, 0, 0, 0];
            let mut bytes = [0u8; 32];
            for (i, v) in header.iter().enumerate() {
                bytes[i * 4..i * 4 + 4].copy_from_slice(&v.to_ne_bytes());
            }
            self.device
                .cmd_update_buffer(command, self.buffer, 0, &bytes);
            let init = barrier
                .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .dst_access_mask(
                    vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
                );
            self.device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().buffer_memory_barriers(&[init]),
            );
            self.device
                .cmd_bind_pipeline(command, vk::PipelineBindPoint::COMPUTE, self.compute);
            self.device.cmd_bind_descriptor_sets(
                command,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_layout,
                0,
                &[self.set],
                &[],
            );
            self.device.cmd_push_constants(
                command,
                self.compute_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                push,
            );
            self.device
                .cmd_dispatch(command, objects.div_ceil(64), 1, 1);
            let publish = init
                .src_stage_mask(
                    vk::PipelineStageFlags2::COMPUTE_SHADER | vk::PipelineStageFlags2::TRANSFER,
                )
                .src_access_mask(
                    vk::AccessFlags2::SHADER_STORAGE_WRITE | vk::AccessFlags2::TRANSFER_WRITE,
                )
                .dst_stage_mask(
                    vk::PipelineStageFlags2::ALL_GRAPHICS | vk::PipelineStageFlags2::DRAW_INDIRECT,
                )
                .dst_access_mask(
                    vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::INDIRECT_COMMAND_READ,
                );
            self.device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().buffer_memory_barriers(&[publish]),
            );
        }
    }
    pub fn geometry_descriptor(&self) -> vk::DescriptorBufferInfo {
        let asset = self
            .asset
            .as_ref()
            .expect("Cooked resolve requires geometry");
        vk::DescriptorBufferInfo::default()
            .buffer(asset.handle)
            .offset(asset.offset)
            .range(asset.bytes)
    }
    /// Caller must have retired every command using this descriptor set and old buffer.
    pub fn replace_asset(&mut self, asset: crate::asset::Buffer) -> crate::asset::Buffer {
        let info = [vk::DescriptorBufferInfo::default()
            .buffer(asset.handle)
            .offset(asset.offset)
            .range(asset.bytes)];
        // SAFETY: Renderer waits its sole frame fence before calling. Binding 1 is present in
        // Asset mode; its layout does not change and the new upload has completed its fence.
        unsafe {
            self.device.update_descriptor_sets(
                &[vk::WriteDescriptorSet::default()
                    .dst_set(self.set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&info)],
                &[],
            );
        }
        self.asset
            .replace(asset)
            .expect("asset replacement requires Asset mode")
    }
    pub fn draw(
        &self,
        command: vk::CommandBuffer,
        graphics_layout: vk::PipelineLayout,
        mesh_commands: &ash::ext::mesh_shader::Device,
    ) {
        // SAFETY: Caller binds a compatible indirect task/mesh graphics pipeline in dynamic rendering.
        // generate() published this buffer, its descriptor layout matches graphics_layout, and the
        // command/count ranges are aligned and bounded. maxDrawCount=1 fits the single command slot.
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                command,
                vk::PipelineBindPoint::GRAPHICS,
                graphics_layout,
                0,
                &[self.set],
                &[],
            );
            mesh_commands.cmd_draw_mesh_tasks_indirect_count(
                command,
                self.buffer,
                0,
                self.buffer,
                COUNT_OFFSET,
                1,
                COMMAND_STRIDE,
            );
        }
    }
}
impl Drop for IndirectWork {
    fn drop(&mut self) {
        // SAFETY: Device outlives work. Complete submissions before destroying pipelines, descriptor
        // pool/sets, layout, buffer and memory. Partial construction has no submitted work.
        unsafe {
            if let Err(error) = self.device.device_wait_idle() {
                eprintln!("Indirect work retirement failed: {error}");
            }
            if self.compute != vk::Pipeline::null() {
                self.device.destroy_pipeline(self.compute, None);
            }
            if self.compute_layout != vk::PipelineLayout::null() {
                self.device
                    .destroy_pipeline_layout(self.compute_layout, None);
            }
            if self.pool != vk::DescriptorPool::null() {
                self.device.destroy_descriptor_pool(self.pool, None);
            }
            if self.descriptor_layout != vk::DescriptorSetLayout::null() {
                self.device
                    .destroy_descriptor_set_layout(self.descriptor_layout, None);
            }
            if self.buffer != vk::Buffer::null() {
                self.device.destroy_buffer(self.buffer, None);
            }
            if self.memory != vk::DeviceMemory::null() {
                self.device.free_memory(self.memory, None);
            }
        }
    }
}
