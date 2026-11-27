use crate::{Result, WORDS};
use ash::{Entry, util::read_spv, vk};
use std::{
    ffi::CStr,
    fs::File,
    io::BufReader,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Default)]
pub struct Validation {
    pub warnings: AtomicUsize,
    pub errors: AtomicUsize,
    checks: AtomicUsize,
}
unsafe extern "system" fn callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    user: *mut std::ffi::c_void,
) -> vk::Bool32 {
    if data.is_null() || user.is_null() {
        return vk::FALSE;
    }
    // SAFETY: Vulkan supplies the callback data; Context retains this Arc through messenger destruction.
    let (message, state) = unsafe {
        (
            CStr::from_ptr((*data).p_message),
            &*user.cast::<Validation>(),
        )
    };
    // SAFETY: Callback data and its optional NUL-terminated message ID are live for this call.
    if unsafe {
        !(*data).p_message_id_name.is_null()
            && CStr::from_ptr((*data).p_message_id_name) == c"THEIA_ECS_CALLBACK_CHECK"
    } {
        state.checks.fetch_add(1, Ordering::Relaxed);
        println!(
            "ECS debug callback delivery confirmed (intentional diagnostic excluded from totals)"
        );
        return vk::FALSE;
    }
    if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        state.errors.fetch_add(1, Ordering::Relaxed);
    } else {
        state.warnings.fetch_add(1, Ordering::Relaxed);
    }
    eprintln!("Vulkan {severity:?}: {}", message.to_string_lossy());
    vk::FALSE
}
struct Context {
    device: Option<ash::Device>,
    debug: Option<ash::ext::debug_utils::Instance>,
    messenger: vk::DebugUtilsMessengerEXT,
    instance: Option<ash::Instance>,
    _entry: Entry,
    _validation: Arc<Validation>,
    physical: vk::PhysicalDevice,
    family: u32,
    queue: vk::Queue,
}
impl Context {
    fn new(filter: &str, validation: Arc<Validation>) -> Result<Self> {
        // SAFETY: Entry is retained until every instance/device handle is destroyed.
        let entry = unsafe { Entry::load()? };
        let mut ctx = Self {
            device: None,
            debug: None,
            messenger: vk::DebugUtilsMessengerEXT::null(),
            instance: None,
            _entry: entry,
            _validation: validation,
            physical: vk::PhysicalDevice::null(),
            family: 0,
            queue: vk::Queue::null(),
        };
        // SAFETY: Feature chains/arrays remain live through calls and each created object is retained immediately.
        unsafe {
            let layers = ctx._entry.enumerate_instance_layer_properties()?;
            if !layers
                .iter()
                .any(|p| CStr::from_ptr(p.layer_name.as_ptr()) == c"VK_LAYER_KHRONOS_validation")
            {
                return Err("Khronos validation layer required for ECS qualification".into());
            }
            let layer_names = [c"VK_LAYER_KHRONOS_validation".as_ptr()];
            let extensions = [
                ash::ext::debug_utils::NAME.as_ptr(),
                ash::ext::validation_features::NAME.as_ptr(),
            ];
            let enabled = [vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION];
            let mut features =
                vk::ValidationFeaturesEXT::default().enabled_validation_features(&enabled);
            let mut debug = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(callback))
                .user_data(Arc::as_ptr(&ctx._validation).cast_mut().cast());
            let app = vk::ApplicationInfo::default()
                .application_name(c"Theia ECS Lab")
                .api_version(vk::API_VERSION_1_3);
            ctx.instance = Some(
                ctx._entry.create_instance(
                    &vk::InstanceCreateInfo::default()
                        .application_info(&app)
                        .enabled_layer_names(&layer_names)
                        .enabled_extension_names(&extensions)
                        .push_next(&mut features)
                        .push_next(&mut debug),
                    None,
                )?,
            );
            let instance = ctx.instance.as_ref().unwrap();
            // push_next linked the instance validation features through this struct. Messenger
            // creation has a separate contract: no instance-only pNext chain may be reused.
            debug.p_next = std::ptr::null();
            ctx.debug = Some(ash::ext::debug_utils::Instance::new(&ctx._entry, instance));
            ctx.messenger = ctx
                .debug
                .as_ref()
                .unwrap()
                .create_debug_utils_messenger(&debug, None)?;
            ctx.debug.as_ref().unwrap().submit_debug_utils_message(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL,
                &vk::DebugUtilsMessengerCallbackDataEXT::default()
                    .message_id_name(c"THEIA_ECS_CALLBACK_CHECK")
                    .message(c"Theia ECS callback-delivery check"),
            );
            if ctx._validation.checks.load(Ordering::Relaxed) == 0 {
                return Err("ECS debug callback did not receive diagnostic".into());
            }
            println!("Khronos layer and synchronization validation enabled for ECS qualification");
            for physical in instance.enumerate_physical_devices()? {
                let props = instance.get_physical_device_properties(physical);
                let name = CStr::from_ptr(props.device_name.as_ptr()).to_string_lossy();
                let mut v13 = vk::PhysicalDeviceVulkan13Features::default();
                let mut mesh = vk::PhysicalDeviceMeshShaderFeaturesEXT::default();
                let mut query = vk::PhysicalDeviceFeatures2::default()
                    .push_next(&mut v13)
                    .push_next(&mut mesh);
                instance.get_physical_device_features2(physical, &mut query);
                let ext = instance.enumerate_device_extension_properties(physical)?;
                let has_mesh = ext.iter().any(|p| {
                    CStr::from_ptr(p.extension_name.as_ptr()) == ash::ext::mesh_shader::NAME
                });
                if !name.to_lowercase().contains(&filter.to_lowercase())
                    || props.api_version < vk::API_VERSION_1_3
                    || v13.synchronization2 == 0
                    || !has_mesh
                    || mesh.mesh_shader == 0
                    || mesh.task_shader == 0
                    || props.limits.max_per_stage_descriptor_storage_buffers < 5
                    || props.limits.max_descriptor_set_storage_buffers < 5
                    || props.limits.max_storage_buffer_range < WORDS as u32 * 4
                {
                    continue;
                }
                if let Some(family) = instance
                    .get_physical_device_queue_family_properties(physical)
                    .iter()
                    .position(|q| q.queue_flags.contains(vk::QueueFlags::COMPUTE))
                {
                    ctx.physical = physical;
                    ctx.family = family as u32;
                    println!(
                        "ECS GPU: {name}; Vulkan 1.3, Sync2, mesh/task capabilities checked; compute family {family}"
                    );
                    break;
                }
            }
            if ctx.physical == vk::PhysicalDevice::null() {
                return Err(
                    "No qualified Vulkan 1.3 mesh/task-capable compute device matches --gpu".into(),
                );
            }
            let priorities = [1.0];
            let queues = [vk::DeviceQueueCreateInfo::default()
                .queue_family_index(ctx.family)
                .queue_priorities(&priorities)];
            let mut v13 = vk::PhysicalDeviceVulkan13Features::default().synchronization2(true);
            ctx.device = Some(
                instance.create_device(
                    ctx.physical,
                    &vk::DeviceCreateInfo::default()
                        .queue_create_infos(&queues)
                        .push_next(&mut v13),
                    None,
                )?,
            );
            ctx.queue = ctx.device.as_ref().unwrap().get_device_queue(ctx.family, 0);
        }
        Ok(ctx)
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: Compute children have been dropped; initialized parents are released in dependency order.
        unsafe {
            if let Some(device) = &self.device {
                device.destroy_device(None);
            }
            if let Some(debug) = &self.debug
                && self.messenger != vk::DebugUtilsMessengerEXT::null()
            {
                debug.destroy_debug_utils_messenger(self.messenger, None);
            }
            if let Some(instance) = &self.instance {
                instance.destroy_instance(None);
            }
        }
    }
}
struct Buffer {
    device: ash::Device,
    handle: vk::Buffer,
    memory: vk::DeviceMemory,
    size: u64,
    mapped: *mut u8,
}
impl Buffer {
    fn new(ctx: &Context, size: u64, usage: vk::BufferUsageFlags, host: bool) -> Result<Self> {
        let device = ctx.device.as_ref().unwrap();
        let mut buffer = Self {
            device: device.clone(),
            handle: vk::Buffer::null(),
            memory: vk::DeviceMemory::null(),
            size,
            mapped: std::ptr::null_mut(),
        };
        // SAFETY: Device/physical handles are live; resource recorded before subsequent fallible operations.
        unsafe {
            buffer.handle = device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(size)
                    .usage(usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )?;
            let requirements = device.get_buffer_memory_requirements(buffer.handle);
            let props = ctx
                .instance
                .as_ref()
                .unwrap()
                .get_physical_device_memory_properties(ctx.physical);
            let flags = if host {
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT
            } else {
                vk::MemoryPropertyFlags::DEVICE_LOCAL
            };
            let memory_type = (0..props.memory_type_count)
                .find(|&i| {
                    requirements.memory_type_bits & (1 << i) != 0
                        && props.memory_types[i as usize]
                            .property_flags
                            .contains(flags)
                })
                .ok_or("No required buffer memory type")?;
            buffer.memory = device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(memory_type),
                None,
            )?;
            device.bind_buffer_memory(buffer.handle, buffer.memory, 0)?;
            if host {
                buffer.mapped = device
                    .map_memory(buffer.memory, 0, size, vk::MemoryMapFlags::empty())?
                    .cast();
            }
        }
        Ok(buffer)
    }
    fn write(&mut self, words: &[u32]) {
        assert!(!self.mapped.is_null() && size_of_val(words) as u64 <= self.size);
        // SAFETY: Mapping is live/coherent and size-checked. Caller has completed the previous submission.
        unsafe {
            std::ptr::copy_nonoverlapping(
                words.as_ptr().cast::<u8>(),
                self.mapped,
                size_of_val(words),
            );
        }
    }
    fn read(&self) -> Vec<u32> {
        assert!(!self.mapped.is_null());
        // SAFETY: Coherent mapping is valid/aligned; caller waited on completion and a GPU->host barrier.
        unsafe {
            std::slice::from_raw_parts(self.mapped.cast::<u32>(), (self.size / 4) as usize).to_vec()
        }
    }
}
impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: Compute waits for submitted uses and keeps the device live until all buffers drop.
        unsafe {
            if !self.mapped.is_null() {
                self.device.unmap_memory(self.memory);
            }
            if self.handle != vk::Buffer::null() {
                self.device.destroy_buffer(self.handle, None);
            }
            if self.memory != vk::DeviceMemory::null() {
                self.device.free_memory(self.memory, None);
            }
        }
    }
}
pub struct Compute {
    buffers: Vec<Buffer>,
    pipelines: Vec<vk::Pipeline>,
    layout: vk::PipelineLayout,
    descriptors: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    set: vk::DescriptorSet,
    pool: vk::CommandPool,
    command: vk::CommandBuffer,
    fence: vk::Fence,
    ctx: Context,
}
impl Compute {
    pub fn new(filter: &str, shader_dir: &Path, validation: Arc<Validation>) -> Result<Self> {
        let ctx = Context::new(filter, validation)?;
        let mut gpu = Self {
            buffers: Vec::new(),
            pipelines: Vec::new(),
            layout: vk::PipelineLayout::null(),
            descriptors: vk::DescriptorSetLayout::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            set: vk::DescriptorSet::null(),
            pool: vk::CommandPool::null(),
            command: vk::CommandBuffer::null(),
            fence: vk::Fence::null(),
            ctx,
        };
        let storage = vk::BufferUsageFlags::STORAGE_BUFFER;
        let transfer = vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST;
        // Arena, commands, statuses, retained reader arena, GPU reader output, arena readback.
        for (size, usage, host) in [
            (WORDS as u64 * 4, storage | transfer, false),
            (64 * 40, storage, true),
            (64 * 12, storage, true),
            (WORDS as u64 * 4, storage | transfer, false),
            (WORDS as u64 * 4, storage, true),
            (WORDS as u64 * 4, vk::BufferUsageFlags::TRANSFER_DST, true),
        ] {
            gpu.buffers.push(Buffer::new(&gpu.ctx, size, usage, host)?);
        }
        let device = gpu.ctx.device.as_ref().unwrap();
        // SAFETY: Create-info arrays live through calls; all objects are saved immediately for RAII cleanup.
        unsafe {
            let bindings: Vec<_> = (0..5)
                .map(|binding| {
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(binding)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::COMPUTE)
                })
                .collect();
            gpu.descriptors = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                None,
            )?;
            let layouts = [gpu.descriptors];
            let ranges = [vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .offset(0)
                .size(16)];
            gpu.layout = device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default()
                    .set_layouts(&layouts)
                    .push_constant_ranges(&ranges),
                None,
            )?;
            let sizes = [vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(5)];
            gpu.descriptor_pool = device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .max_sets(1)
                    .pool_sizes(&sizes),
                None,
            )?;
            gpu.set = device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(gpu.descriptor_pool)
                    .set_layouts(&layouts),
            )?[0];
            let infos: Vec<_> = gpu.buffers[..5]
                .iter()
                .map(|b| {
                    [vk::DescriptorBufferInfo::default()
                        .buffer(b.handle)
                        .range(b.size)]
                })
                .collect();
            let writes: Vec<_> = infos
                .iter()
                .enumerate()
                .map(|(i, info)| {
                    vk::WriteDescriptorSet::default()
                        .dst_set(gpu.set)
                        .dst_binding(i as u32)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(info)
                })
                .collect();
            device.update_descriptor_sets(&writes, &[]);
            for entry in ["initialize", "mutate", "integrate", "readRetained"] {
                let words = read_spv(&mut BufReader::new(File::open(
                    shader_dir.join(format!("ecs.{entry}.spv")),
                )?))?;
                let module = device.create_shader_module(
                    &vk::ShaderModuleCreateInfo::default().code(&words),
                    None,
                )?;
                let create = [vk::ComputePipelineCreateInfo::default()
                    .layout(gpu.layout)
                    .stage(
                        vk::PipelineShaderStageCreateInfo::default()
                            .stage(vk::ShaderStageFlags::COMPUTE)
                            .module(module)
                            .name(c"main"),
                    )];
                let result =
                    device.create_compute_pipelines(vk::PipelineCache::null(), &create, None);
                device.destroy_shader_module(module, None);
                match result {
                    Ok(pipelines) => gpu.pipelines.extend(pipelines),
                    Err((partial, error)) => {
                        for pipeline in partial {
                            if pipeline != vk::Pipeline::null() {
                                device.destroy_pipeline(pipeline, None);
                            }
                        }
                        return Err(error.into());
                    }
                }
            }
            gpu.pool = device.create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .queue_family_index(gpu.ctx.family)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )?;
            gpu.command = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(gpu.pool)
                    .command_buffer_count(1),
            )?[0];
            gpu.fence = device.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?;
        }
        gpu.execute(0, u32::MAX, 0.0, Some(1), false, false)?;
        Ok(gpu)
    }
    fn barrier(&self, src: vk::PipelineStageFlags2, dst: vk::PipelineStageFlags2) {
        let barriers: Vec<_> = self
            .buffers
            .iter()
            .map(|buffer| {
                vk::BufferMemoryBarrier2::default()
                    .src_stage_mask(src)
                    .src_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
                    .dst_stage_mask(dst)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .buffer(buffer.handle)
                    .size(buffer.size)
            })
            .collect();
        // SAFETY: Called while recording the owned command buffer, with live buffer ranges.
        unsafe {
            self.ctx.device.as_ref().unwrap().cmd_pipeline_barrier2(
                self.command,
                &vk::DependencyInfo::default().buffer_memory_barriers(&barriers),
            );
        }
    }
    fn dispatch(&self, pipeline: usize, groups: u32) {
        // SAFETY: Pipeline/layout/descriptors belong to the live device and current command buffer.
        unsafe {
            let device = self.ctx.device.as_ref().unwrap();
            device.cmd_bind_pipeline(
                self.command,
                vk::PipelineBindPoint::COMPUTE,
                self.pipelines[pipeline],
            );
            device.cmd_dispatch(self.command, groups, 1, 1);
        }
    }
    fn execute(
        &mut self,
        count: u32,
        oldest: u32,
        dt: f32,
        initialize: Option<u32>,
        snapshot: bool,
        reader: bool,
    ) -> Result<()> {
        let device = self.ctx.device.as_ref().unwrap();
        // SAFETY: Single submission at a time; the fence protects buffers, mappings and command reuse.
        unsafe {
            device.wait_for_fences(&[self.fence], true, u64::MAX)?;
            device.reset_command_buffer(self.command, vk::CommandBufferResetFlags::empty())?;
            device.begin_command_buffer(
                self.command,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            device.cmd_bind_descriptor_sets(
                self.command,
                vk::PipelineBindPoint::COMPUTE,
                self.layout,
                0,
                &[self.set],
                &[],
            );
            let params = [count, oldest, dt.to_bits(), initialize.unwrap_or(0)];
            let bytes = std::slice::from_raw_parts(params.as_ptr().cast::<u8>(), 16);
            device.cmd_push_constants(
                self.command,
                self.layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                bytes,
            );
            self.barrier(
                vk::PipelineStageFlags2::ALL_COMMANDS,
                vk::PipelineStageFlags2::ALL_COMMANDS,
            );
            if initialize.is_some() {
                self.dispatch(0, WORDS.div_ceil(64) as u32);
            } else {
                self.dispatch(1, 1);
                self.barrier(
                    vk::PipelineStageFlags2::COMPUTE_SHADER,
                    vk::PipelineStageFlags2::COMPUTE_SHADER,
                );
                self.dispatch(2, 1);
            }
            self.barrier(
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::PipelineStageFlags2::TRANSFER,
            );
            let copies = [vk::BufferCopy::default().size(WORDS as u64 * 4)];
            device.cmd_copy_buffer(
                self.command,
                self.buffers[0].handle,
                self.buffers[5].handle,
                &copies,
            );
            if snapshot {
                device.cmd_copy_buffer(
                    self.command,
                    self.buffers[0].handle,
                    self.buffers[3].handle,
                    &copies,
                );
            }
            if reader {
                self.barrier(
                    vk::PipelineStageFlags2::ALL_COMMANDS,
                    vk::PipelineStageFlags2::COMPUTE_SHADER,
                );
                self.dispatch(3, WORDS.div_ceil(64) as u32);
            }
            self.barrier(
                vk::PipelineStageFlags2::ALL_COMMANDS,
                vk::PipelineStageFlags2::HOST,
            );
            device.end_command_buffer(self.command)?;
            device.reset_fences(&[self.fence])?;
            let commands = [vk::CommandBufferSubmitInfo::default().command_buffer(self.command)];
            device.queue_submit2(
                self.ctx.queue,
                &[vk::SubmitInfo2::default().command_buffer_infos(&commands)],
                self.fence,
            )?;
            device.wait_for_fences(&[self.fence], true, u64::MAX)?;
        }
        Ok(())
    }
    pub fn batch(
        &mut self,
        commands: &[u32],
        oldest: u32,
        dt: f32,
        snapshot: bool,
        reader: bool,
    ) -> Result<Vec<u32>> {
        assert!(commands.len().is_multiple_of(10) && commands.len() <= 640);
        self.buffers[1].write(commands);
        self.execute(
            (commands.len() / 10) as u32,
            oldest,
            dt,
            None,
            snapshot,
            reader,
        )?;
        Ok(self.buffers[2].read())
    }
    pub fn reset(&mut self, generation: u32) -> Result<()> {
        assert!(generation > 0);
        self.execute(0, u32::MAX, 0.0, Some(generation), false, false)
    }
    pub fn state(&self) -> Vec<u32> {
        self.buffers[5].read()
    }
    pub fn retained(&self) -> Vec<u32> {
        self.buffers[4].read()
    }
}
impl Drop for Compute {
    fn drop(&mut self) {
        // SAFETY: Retire all GPU work before child objects; Context is the last declared field.
        unsafe {
            let device = self.ctx.device.as_ref().unwrap();
            let _ = device.device_wait_idle();
            if self.fence != vk::Fence::null() {
                device.destroy_fence(self.fence, None);
            }
            if self.pool != vk::CommandPool::null() {
                device.destroy_command_pool(self.pool, None);
            }
            for pipeline in self.pipelines.drain(..) {
                device.destroy_pipeline(pipeline, None);
            }
            if self.descriptor_pool != vk::DescriptorPool::null() {
                device.destroy_descriptor_pool(self.descriptor_pool, None);
            }
            if self.layout != vk::PipelineLayout::null() {
                device.destroy_pipeline_layout(self.layout, None);
            }
            if self.descriptors != vk::DescriptorSetLayout::null() {
                device.destroy_descriptor_set_layout(self.descriptors, None);
            }
        }
        self.buffers.clear();
    }
}
