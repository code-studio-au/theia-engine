//! Isolated batched selected-terrain GPU readback gate; no crowd rendering.
use crate::{
    Result,
    asset::{Buffer, Transfer},
    pipeline::{ComputeSubmission, ShaderModule},
    world_ground,
};
use ash::vk;
use std::path::Path;
pub fn qualify(
    device: &ash::Device,
    memory: &vk::PhysicalDeviceMemoryProperties,
    family: u32,
    queue: vk::Queue,
    shaders: &Path,
    paths: (&Path, &Path, &Path),
    limits: vk::PhysicalDeviceLimits,
) -> Result<()> {
    let input = world_ground::gpu_input(paths.0, paths.1)?;
    let output_bytes = input.queries.len() as u64;
    if output_bytes > u64::from(limits.max_storage_buffer_range)
        || (input.expected.len() as u32).div_ceil(64) > limits.max_compute_work_group_count[0]
    {
        return Err("Ground probe exceeds device storage/dispatch limits".into());
    }
    let table = Buffer::upload(
        device,
        memory,
        &input.table,
        family,
        queue,
        16 * 1024 * 1024,
        None,
    )?;
    let queries = Buffer::upload(
        device,
        memory,
        &input.queries,
        family,
        queue,
        16 * 1024 * 1024,
        None,
    )?;
    let output = Buffer::allocate(
        device,
        memory,
        output_bytes,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        16 * 1024 * 1024,
    )?;
    // Drops before buffers, including every pipeline creation/submission error.
    let mut probe = ComputeSubmission::new(device);
    let shader = ShaderModule::load(device, &shaders.join("world.ground.compute.spv"))?;
    // SAFETY: All input arrays and buffer owners outlive calls/submission. Every
    // successful object immediately gains ownership; Probe retires commands first.
    unsafe {
        let bindings: Vec<_> = (0..3)
            .map(|binding| {
                vk::DescriptorSetLayoutBinding::default()
                    .binding(binding)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
            })
            .collect();
        probe.descriptor_layout = device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
            None,
        )?;
        let sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(3)];
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
        let buffers = [&table, &queries, &output];
        let info: Vec<_> = buffers
            .iter()
            .map(|b| {
                vk::DescriptorBufferInfo::default()
                    .buffer(b.handle)
                    .offset(b.offset)
                    .range(b.bytes)
            })
            .collect();
        let writes: Vec<_> = info
            .iter()
            .enumerate()
            .map(|(binding, item)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(binding as u32)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(item))
            })
            .collect();
        device.update_descriptor_sets(&writes, &[]);
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
        device.cmd_fill_buffer(command, output.handle, 0, output_bytes, 0xdeadbeef);
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
            .size(output_bytes)];
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
        device.cmd_dispatch(command, (input.expected.len() as u32).div_ceil(64), 1, 1);
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
        let pointer =
            device.map_memory(output.memory, 0, output_bytes, vk::MemoryMapFlags::empty())?;
        let mut actual = vec![0u32; input.expected.len() * 4];
        std::ptr::copy_nonoverlapping(
            pointer.cast::<u32>(),
            actual.as_mut_ptr(),
            input.expected.len() * 4,
        );
        device.unmap_memory(output.memory);

        let mut heights = Vec::with_capacity(input.expected.len());
        let mut max_error = 0f32;
        for (words, &(expected, region)) in actual.as_chunks::<4>().0.iter().zip(&input.expected) {
            let value = f32::from_bits(words[0]);
            let error = (value - expected).abs();
            if !value.is_finite()
                || error > 0.000003
                || words[1] != region
                || words[2] != 0
                || !(1..=8).contains(&words[3])
            {
                return Err(format!("Ground GPU query differs: actual {words:?}, expected {expected} in region {region}").into());
            }
            max_error = max_error.max(error);
            heights.push(value);
        }
        if let Some(parent) = paths.2.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let report = serde_json::json!({"passed":true,"gpu_execution":true,"presentation_only":true,"points":input.points,"mixed_selections":16,"verified_heights":heights.len(),"heights":heights,"max_cpu_gpu_error":max_error,"table_bytes":input.table.len(),"physical_buffer_bytes":table.allocated_bytes+queries.allocated_bytes+output.allocated_bytes,"rendered_proxies":false});
        std::fs::write(paths.2, serde_json::to_vec(&report)?)?;
        println!(
            "Selected terrain GPU query verified: {}",
            serde_json::json!({"heights":heights.len(),"max_error":max_error,"table_bytes":input.table.len(),"physical_buffer_bytes":table.allocated_bytes+queries.allocated_bytes+output.allocated_bytes})
        );
    }
    Ok(())
}
