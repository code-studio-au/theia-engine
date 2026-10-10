//! One device-local work buffer, generated and consumed without host readback.
use crate::{Result, pipeline::ShaderModule};
use ash::vk;
use std::path::Path;
const BUFFER_BYTES: vk::DeviceSize = 64;
const COUNT_OFFSET: vk::DeviceSize = 12;
const COMMAND_STRIDE: u32 = 12;

pub struct IndirectWork {
    device: ash::Device,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    pub descriptor_layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    set: vk::DescriptorSet,
    compute_layout: vk::PipelineLayout,
    compute: vk::Pipeline,
}
impl IndirectWork {
    pub fn new(
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        shader_dir: &Path,
    ) -> Result<Self> {
        let shader = ShaderModule::load(device, &shader_dir.join("work.compute.spv"))?;
        let mut work = Self {
            device: device.clone(),
            buffer: vk::Buffer::null(),
            memory: vk::DeviceMemory::null(),
            descriptor_layout: vk::DescriptorSetLayout::null(),
            pool: vk::DescriptorPool::null(),
            set: vk::DescriptorSet::null(),
            compute_layout: vk::PipelineLayout::null(),
            compute: vk::Pipeline::null(),
        };
        // SAFETY: All create-info backing arrays remain live through calls. Every successful handle
        // immediately gains ownership in work; Drop handles partial initialization in dependency order.
        unsafe {
            work.buffer = device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(BUFFER_BYTES)
                    .usage(
                        vk::BufferUsageFlags::STORAGE_BUFFER
                            | vk::BufferUsageFlags::INDIRECT_BUFFER,
                    )
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )?;
            let requirements = device.get_buffer_memory_requirements(work.buffer);
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
            let bindings = [vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE | vk::ShaderStageFlags::TASK_EXT)];
            work.descriptor_layout = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                None,
            )?;
            let sizes = [vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)];
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
                .range(BUFFER_BYTES)];
            device.update_descriptor_sets(
                &[vk::WriteDescriptorSet::default()
                    .dst_set(work.set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&buffers)],
                &[],
            );
            let push = [vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .offset(0)
                .size(4)];
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
        println!(
            "GPU indirect work initialized: 64-byte device-local buffer; command at 0, count at 12, transforms/mesh count at 16..40; no mapping/readback"
        );
        Ok(work)
    }
    pub fn generate(&self, command: vk::CommandBuffer, requested_items: u32) {
        // SAFETY: Caller has begun recording a graphics+compute command buffer and waited its frame
        // fence. Work handles/descriptor set remain live; explicit barriers order reuse, compute writes,
        // indirect command/count reads and task descriptor reads. Push constants are copied by Vulkan.
        unsafe {
            let reuse = [vk::BufferMemoryBarrier2::default()
                .src_stage_mask(
                    vk::PipelineStageFlags2::DRAW_INDIRECT
                        | vk::PipelineStageFlags2::TASK_SHADER_EXT
                        | vk::PipelineStageFlags2::COMPUTE_SHADER,
                )
                .src_access_mask(
                    vk::AccessFlags2::INDIRECT_COMMAND_READ
                        | vk::AccessFlags2::SHADER_STORAGE_READ
                        | vk::AccessFlags2::SHADER_STORAGE_WRITE,
                )
                .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(self.buffer)
                .offset(0)
                .size(BUFFER_BYTES)];
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
            self.device.cmd_push_constants(
                command,
                self.compute_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                &requested_items.to_ne_bytes(),
            );
            self.device.cmd_dispatch(command, 1, 1, 1);
            let publish = [vk::BufferMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
                .dst_stage_mask(
                    vk::PipelineStageFlags2::DRAW_INDIRECT
                        | vk::PipelineStageFlags2::TASK_SHADER_EXT,
                )
                .dst_access_mask(
                    vk::AccessFlags2::INDIRECT_COMMAND_READ | vk::AccessFlags2::SHADER_STORAGE_READ,
                )
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(self.buffer)
                .offset(0)
                .size(BUFFER_BYTES)];
            self.device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().buffer_memory_barriers(&publish),
            );
        }
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
