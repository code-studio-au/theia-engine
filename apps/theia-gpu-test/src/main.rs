use ash::{Entry, util::read_spv, vk};
use std::{ffi::CStr, fs::File, io::BufReader};
const COUNT: usize = 256;
const BUFFER_SIZE: u64 = (COUNT * size_of::<u32>()) as u64;

const AMD_VENDOR: u32 = 0x1002;
const RX_7900_XTX: u32 = 0x744c;

unsafe extern "system" fn vulkan_debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    if !callback_data.is_null() {
        let message = unsafe { CStr::from_ptr((*callback_data).p_message) };

        if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
            eprintln!("[Vulkan ERROR] {}", message.to_string_lossy());
        } else {
            eprintln!("[Vulkan WARNING] {}", message.to_string_lossy());
        }
    }

    vk::FALSE
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_gpu_test()
}

fn run_gpu_test() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // 1. Initialize Vulkan.
        let entry = Entry::load()?;

        let app_info = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);

        let debug_extensions = [vk::EXT_DEBUG_UTILS_NAME.as_ptr()];

        let instance = entry.create_instance(
            &vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_extension_names(&debug_extensions),
            None,
        )?;

        let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);

        let debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_messenger = debug_utils.create_debug_utils_messenger(&debug_create_info, None)?;
        println!("Vulkan debug messenger initialized.");
        // 2. Select the AMD Radeon RX 7900 XTX.
        let physical = instance
            .enumerate_physical_devices()?
            .into_iter()
            .find(|&gpu| {
                let props = instance.get_physical_device_properties(gpu);
                props.vendor_id == AMD_VENDOR && props.device_id == RX_7900_XTX
            })
            .ok_or("RX 7900 XTX Vulkan device not found")?;

        let props = instance.get_physical_device_properties(physical);
        let name = CStr::from_ptr(props.device_name.as_ptr());

        println!("GPU: {}", name.to_string_lossy());

        // 3. Find a compute-capable queue.
        let queue_family = instance
            .get_physical_device_queue_family_properties(physical)
            .iter()
            .position(|q| q.queue_flags.contains(vk::QueueFlags::COMPUTE))
            .ok_or("No compute queue available")? as u32;

        let priorities = [1.0f32];

        let queue_infos = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&priorities)];

        let device = instance.create_device(
            physical,
            &vk::DeviceCreateInfo::default().queue_create_infos(&queue_infos),
            None,
        )?;

        let queue = device.get_device_queue(queue_family, 0);

        // 4. Create a 1 KiB GPU storage buffer.
        let buffer = device.create_buffer(
            &vk::BufferCreateInfo::default()
                .size(BUFFER_SIZE)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
            None,
        )?;

        let requirements = device.get_buffer_memory_requirements(buffer);

        let memory_props = instance.get_physical_device_memory_properties(physical);

        // Use host-visible coherent memory for this simple test.
        let memory_type = (0..memory_props.memory_type_count)
            .find(|&i| {
                let allowed = requirements.memory_type_bits & (1u32 << i) != 0;

                let flags = memory_props.memory_types[i as usize].property_flags;

                allowed
                    && flags.contains(
                        vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT,
                    )
            })
            .ok_or("No host-visible coherent memory type")?;

        let memory = device.allocate_memory(
            &vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(memory_type),
            None,
        )?;

        device.bind_buffer_memory(buffer, memory, 0)?;

        // 5. Create descriptor layout: set 0, binding 0.
        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)];

        let set_layout = device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
            None,
        )?;

        let pool_sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 1,
        }];

        let descriptor_pool = device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::default()
                .max_sets(1)
                .pool_sizes(&pool_sizes),
            None,
        )?;

        let set_layouts = [set_layout];

        let descriptor_set = device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&set_layouts),
        )?[0];

        let buffer_infos = [vk::DescriptorBufferInfo::default()
            .buffer(buffer)
            .offset(0)
            .range(BUFFER_SIZE)];

        let writes = [vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&buffer_infos)];

        device.update_descriptor_sets(&writes, &[]);

        // 6. Load Slang-generated SPIR-V.
        let shader_path = "build/shaders/buffer_test.spv";

        let spirv = read_spv(&mut BufReader::new(File::open(shader_path)?))?;

        let shader = device
            .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&spirv), None)?;

        // 7. Create compute pipeline.
        let pipeline_layout = device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts),
            None,
        )?;

        // Slang generated the SPIR-V entry point named "main".
        let shader_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader)
            .name(c"main");

        let pipeline_infos = [vk::ComputePipelineCreateInfo::default()
            .stage(shader_stage)
            .layout(pipeline_layout)];

        let pipelines = device
            .create_compute_pipelines(vk::PipelineCache::null(), &pipeline_infos, None)
            .map_err(|(_, error)| error)?;

        let pipeline = pipelines[0];

        // 8. Create command pool and command buffer.
        let command_pool = device.create_command_pool(
            &vk::CommandPoolCreateInfo::default().queue_family_index(queue_family),
            None,
        )?;

        let commands = device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )?;

        let cmd = commands[0];

        device.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default())?;

        // 9. Record GPU compute execution.
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, pipeline);

        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );

        // 4 workgroups * 64 threads = 256 threads.
        device.cmd_dispatch(cmd, 4, 1, 1);

        device.end_command_buffer(cmd)?;

        // 10. Submit GPU work.
        let command_buffers = [cmd];

        let submits = [vk::SubmitInfo::default().command_buffers(&command_buffers)];

        device.queue_submit(queue, &submits, vk::Fence::null())?;

        // Simple synchronization for this one-shot test.
        device.queue_wait_idle(queue)?;

        // 11. Read GPU-computed values.
        let mapped =
            device.map_memory(memory, 0, requirements.size, vk::MemoryMapFlags::empty())?;

        let values = std::slice::from_raw_parts(mapped as *const u32, COUNT);

        for (index, &value) in values.iter().enumerate() {
            let expected = index as u32 * 2 + 1;

            assert_eq!(value, expected, "GPU result mismatch at index {index}");
        }

        println!("All {COUNT} GPU values verified!");
        println!("First 8: {:?}", &values[..8]);
        println!("Last 8: {:?}", &values[COUNT - 8..]);

        device.unmap_memory(memory);

        // 12. Cleanup.
        device.destroy_command_pool(command_pool, None);
        device.destroy_pipeline(pipeline, None);
        device.destroy_pipeline_layout(pipeline_layout, None);
        device.destroy_shader_module(shader, None);
        device.destroy_descriptor_pool(descriptor_pool, None);
        device.destroy_descriptor_set_layout(set_layout, None);
        device.destroy_buffer(buffer, None);
        device.free_memory(memory, None);

        device.destroy_device(None);

        debug_utils.destroy_debug_utils_messenger(debug_messenger, None);

        instance.destroy_instance(None);

        Ok(())
    }
}
