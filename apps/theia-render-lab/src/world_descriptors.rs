//! Isolated GPU gate for nonuniform storage descriptor arrays, not a world renderer.
use crate::{
    Result,
    asset::{Buffer, Transfer},
    pipeline::ShaderModule,
};
use ash::vk;
use std::path::Path;

struct Probe {
    // Retire commands before deleting referenced layouts/pipelines/descriptors.
    transfer: Option<Transfer>,
    device: ash::Device,
    descriptor_layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}
impl Drop for Probe {
    fn drop(&mut self) {
        drop(self.transfer.take());
        // SAFETY: Transfer retirement completed; caller retains the parent device.
        unsafe {
            if self.pipeline != vk::Pipeline::null() {
                self.device.destroy_pipeline(self.pipeline, None);
            }
            if self.layout != vk::PipelineLayout::null() {
                self.device.destroy_pipeline_layout(self.layout, None);
            }
            if self.pool != vk::DescriptorPool::null() {
                self.device.destroy_descriptor_pool(self.pool, None);
            }
            if self.descriptor_layout != vk::DescriptorSetLayout::null() {
                self.device
                    .destroy_descriptor_set_layout(self.descriptor_layout, None);
            }
        }
    }
}
pub fn qualify(
    device: &ash::Device,
    memory: &vk::PhysicalDeviceMemoryProperties,
    family: u32,
    queue: vk::Queue,
    shaders: &Path,
    limits: vk::PhysicalDeviceLimits,
) -> Result<()> {
    const SOURCES: [[u32; 4]; 4] = [
        [13, 29, 47, 71],
        [101, 131, 167, 199],
        [239, 281, 337, 383],
        [431, 487, 541, 601],
    ];
    const EXPECTED: [u32; 16] = [
        13, 101, 239, 431, 29, 131, 281, 487, 47, 167, 337, 541, 71, 199, 383, 601,
    ];
    let mut sources = Vec::new();
    for words in SOURCES {
        let bytes: Vec<_> = words.into_iter().flat_map(u32::to_le_bytes).collect();
        sources.push(Buffer::upload(
            device, memory, &bytes, family, queue, 1024, None,
        )?);
    }
    let output = Buffer::allocate(
        device,
        memory,
        256,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        1024,
    )?;
    // This guard drops before sources/output on every construction/error path.
    let mut probe = Probe {
        transfer: None,
        device: device.clone(),
        descriptor_layout: vk::DescriptorSetLayout::null(),
        pool: vk::DescriptorPool::null(),
        layout: vk::PipelineLayout::null(),
        pipeline: vk::Pipeline::null(),
    };
    let shader = ShaderModule::load(device, &shaders.join("world.descriptors.compute.spv"))?;
    // SAFETY: All input arrays and buffer owners outlive calls/submission. Every
    // successful object immediately gains ownership; Probe retires commands first.
    unsafe {
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(4)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
        ];
        probe.descriptor_layout = device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
            None,
        )?;
        let sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(5)];
        probe.pool = device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::default()
                .max_sets(1)
                .pool_sizes(&sizes),
            None,
        )?;
        let layouts = [probe.descriptor_layout];
        let set = device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(probe.pool)
                .set_layouts(&layouts),
        )?[0];
        let source_info: Vec<_> = sources
            .iter()
            .map(|b| {
                vk::DescriptorBufferInfo::default()
                    .buffer(b.handle)
                    .offset(b.offset)
                    .range(b.bytes)
            })
            .collect();
        let out_info = [vk::DescriptorBufferInfo::default()
            .buffer(output.handle)
            .range(256)];
        device.update_descriptor_sets(
            &[
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&source_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&out_info),
            ],
            &[],
        );
        probe.layout = device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::default().set_layouts(&layouts),
            None,
        )?;
        let stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader.handle)
            .name(c"main");
        probe.pipeline = match device.create_compute_pipelines(
            vk::PipelineCache::null(),
            &[vk::ComputePipelineCreateInfo::default()
                .stage(stage)
                .layout(probe.layout)],
            None,
        ) {
            Ok(pipelines) => pipelines[0],
            Err((pipelines, error)) => {
                for pipeline in pipelines {
                    if pipeline != vk::Pipeline::null() {
                        device.destroy_pipeline(pipeline, None);
                    }
                }
                return Err(error.into());
            }
        };
        probe.transfer = Some(Transfer {
            device: device.clone(),
            pool: vk::CommandPool::null(),
            fence: vk::Fence::null(),
            submitted: false,
        });
        let transfer = probe.transfer.as_mut().unwrap();
        transfer.pool = device.create_command_pool(
            &vk::CommandPoolCreateInfo::default().queue_family_index(family),
            None,
        )?;
        transfer.fence = device.create_fence(&vk::FenceCreateInfo::default(), None)?;
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
        device.cmd_fill_buffer(command, output.handle, 0, 256, 0xdeadbeef);
        let init = [vk::BufferMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(
                vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
            )
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .buffer(output.handle)
            .size(256)];
        device.cmd_pipeline_barrier2(
            command,
            &vk::DependencyInfo::default().buffer_memory_barriers(&init),
        );
        device.cmd_bind_pipeline(command, vk::PipelineBindPoint::COMPUTE, probe.pipeline);
        device.cmd_bind_descriptor_sets(
            command,
            vk::PipelineBindPoint::COMPUTE,
            probe.layout,
            0,
            &[set],
            &[],
        );
        device.cmd_dispatch(command, 1, 1, 1);
        let host = [init[0]
            .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::HOST)
            .dst_access_mask(vk::AccessFlags2::HOST_READ)];
        device.cmd_pipeline_barrier2(
            command,
            &vk::DependencyInfo::default().buffer_memory_barriers(&host),
        );
        device.end_command_buffer(command)?;
        let commands = [vk::CommandBufferSubmitInfo::default().command_buffer(command)];
        device.queue_submit2(
            queue,
            &[vk::SubmitInfo2::default().command_buffer_infos(&commands)],
            transfer.fence,
        )?;
        transfer.submitted = true;
        device.wait_for_fences(&[transfer.fence], true, u64::MAX)?;
        transfer.submitted = false;
        let pointer = device.map_memory(output.memory, 0, 256, vk::MemoryMapFlags::empty())?;
        let mut actual = [0u32; 64];
        std::ptr::copy_nonoverlapping(pointer.cast::<u32>(), actual.as_mut_ptr(), 64);
        device.unmap_memory(output.memory);
        if actual
            .iter()
            .enumerate()
            .any(|(i, v)| *v != EXPECTED[i % 16])
        {
            return Err("Nonuniform storage descriptor GPU outputs differ".into());
        }
    }
    println!(
        "World storage descriptor probe verified: {}",
        serde_json::json!({
        "source_buffers":4,"verified_gpu_values":64,"expected_first16":EXPECTED,
        "max_per_stage_storage_buffers":limits.max_per_stage_descriptor_storage_buffers,
        "max_set_storage_buffers":limits.max_descriptor_set_storage_buffers,
        "physical_buffer_bytes":sources.iter().map(|b|b.allocated_bytes).sum::<u64>()+output.allocated_bytes,
        "nonuniform_within_workgroup":true,"rendered_world_chunks":false})
    );
    Ok(())
}
