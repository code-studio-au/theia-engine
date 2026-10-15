use crate::wsi::{PresentOutcome, Probe, RecoveryProbes};
use crate::{Options, RenderMode, Result, indirect::IndirectWork, pipeline::Pipeline};
use ash::{Entry, vk};
use std::{
    ffi::{CStr, c_void},
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use winit::{
    dpi::PhysicalSize,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle},
    window::Window,
};
const LAYER: &CStr = c"VK_LAYER_KHRONOS_validation";
const CHECK: &CStr = c"THEIA_DEBUG_CALLBACK_CHECK";
#[derive(Default)]
pub struct Validation {
    warnings: AtomicUsize,
    errors: AtomicUsize,
    checks: AtomicUsize,
}
impl Validation {
    pub fn counts(&self) -> (usize, usize) {
        (
            self.warnings.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        )
    }
}
unsafe extern "system" fn callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    user: *mut c_void,
) -> vk::Bool32 {
    if data.is_null() || user.is_null() {
        return vk::FALSE;
    }
    // SAFETY: Vulkan supplies callback data; the retained Arc outlives the instance/messenger.
    let (data, state) = unsafe { (&*data, &*user.cast::<Validation>()) };
    // SAFETY: Non-null callback strings are NUL-terminated by Vulkan.
    let check = !data.p_message_id_name.is_null()
        && unsafe { CStr::from_ptr(data.p_message_id_name) } == CHECK;
    if check {
        state.checks.fetch_add(1, Ordering::Relaxed);
    } else if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        state.errors.fetch_add(1, Ordering::Relaxed);
    } else {
        state.warnings.fetch_add(1, Ordering::Relaxed);
    }
    if !data.p_message.is_null() {
        // SAFETY: The string remains valid until this callback returns. Ignore IO errors; never panic across FFI.
        let message = unsafe { CStr::from_ptr(data.p_message) }.to_string_lossy();
        let _ = writeln!(std::io::stderr(), "Vulkan {severity:?}: {message}");
    }
    vk::FALSE
}
fn debug_info(state: &Validation) -> vk::DebugUtilsMessengerCreateInfoEXT<'_> {
    vk::DebugUtilsMessengerCreateInfoEXT::default()
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
        .user_data((state as *const Validation).cast_mut().cast())
}
fn has_extension(properties: &[vk::ExtensionProperties], name: &CStr) -> bool {
    // SAFETY: Vulkan extension names are NUL-terminated fixed arrays.
    properties
        .iter()
        .any(|property| unsafe { CStr::from_ptr(property.extension_name.as_ptr()) } == name)
}
/// Owns all non-swapchain handles. Partial construction is cleaned up by Drop.
struct Context {
    entry: Entry,
    instance: Option<ash::Instance>,
    debug: Option<ash::ext::debug_utils::Instance>,
    messenger: vk::DebugUtilsMessengerEXT,
    surface_loader: Option<ash::khr::surface::Instance>,
    surface: vk::SurfaceKHR,
    device: Option<ash::Device>,
    mesh_commands: Option<ash::ext::mesh_shader::Device>,
    physical: vk::PhysicalDevice,
    graphics_family: u32,
    present_family: u32,
    graphics: vk::Queue,
    present: vk::Queue,
    pool: vk::CommandPool,
    command: vk::CommandBuffer,
    acquired: vk::Semaphore,
    complete: vk::Fence,
    window: Arc<Window>,
    validation: Arc<Validation>,
}
impl Context {
    fn new(window: Arc<Window>, options: &Options, validation: Arc<Validation>) -> Result<Self> {
        // SAFETY: Entry retains the loaded Vulkan library until all objects have been destroyed.
        let entry = unsafe { Entry::load()? };
        let mut ctx = Self {
            entry,
            instance: None,
            debug: None,
            messenger: vk::DebugUtilsMessengerEXT::null(),
            surface_loader: None,
            surface: vk::SurfaceKHR::null(),
            device: None,
            mesh_commands: None,
            physical: vk::PhysicalDevice::null(),
            graphics_family: 0,
            present_family: 0,
            graphics: vk::Queue::null(),
            present: vk::Queue::null(),
            pool: vk::CommandPool::null(),
            command: vk::CommandBuffer::null(),
            acquired: vk::Semaphore::null(),
            complete: vk::Fence::null(),
            window,
            validation,
        };
        ctx.initialize(options)?;
        Ok(ctx)
    }
    fn initialize(&mut self, options: &Options) -> Result<()> {
        // SAFETY: Create-info arrays and pNext chains remain live through calls. Handles are retained
        // in Context immediately after creation, and Drop releases children before their parents.
        unsafe {
            let version = self
                .entry
                .try_enumerate_instance_version()?
                .unwrap_or(vk::API_VERSION_1_0);
            if version < vk::API_VERSION_1_3 {
                return Err("Vulkan 1.3 loader required".into());
            }
            let extensions = self.entry.enumerate_instance_extension_properties(None)?;
            let layers = self.entry.enumerate_instance_layer_properties()?;
            let enabled_layers = if options.no_validation {
                Vec::new()
            } else {
                if !layers
                    .iter()
                    .any(|layer| CStr::from_ptr(layer.layer_name.as_ptr()) == LAYER)
                {
                    return Err("Khronos validation layer missing; install it or explicitly use --no-validation".into());
                }
                vec![LAYER.as_ptr()]
            };
            let mut enabled_extensions =
                ash_window::enumerate_required_extensions(self.window.display_handle()?.as_raw())?
                    .to_vec();
            // Presentation fences are required by this lab to make retirement provable, including teardown.
            for extension in [
                ash::ext::debug_utils::NAME,
                ash::khr::get_surface_capabilities2::NAME,
                ash::ext::surface_maintenance1::NAME,
            ] {
                if !has_extension(&extensions, extension) {
                    return Err(
                        format!("Render Lab requires instance extension {extension:?}").into(),
                    );
                }
                enabled_extensions.push(extension.as_ptr());
            }
            let validation_enables = [vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION];
            let mut validation_features = vk::ValidationFeaturesEXT::default()
                .enabled_validation_features(&validation_enables);
            if !options.no_validation {
                let layer_extensions = self
                    .entry
                    .enumerate_instance_extension_properties(Some(LAYER))?;
                if !has_extension(&layer_extensions, ash::ext::validation_features::NAME) {
                    return Err(
                        "Validation layer does not expose VK_EXT_validation_features".into(),
                    );
                }
                enabled_extensions.push(ash::ext::validation_features::NAME.as_ptr());
            }
            let app = vk::ApplicationInfo::default()
                .application_name(c"Theia Render Lab")
                .api_version(vk::API_VERSION_1_3);
            let mut debug_create = debug_info(&self.validation);
            let mut instance_info = vk::InstanceCreateInfo::default()
                .application_info(&app)
                .enabled_layer_names(&enabled_layers)
                .enabled_extension_names(&enabled_extensions)
                .push_next(&mut debug_create);
            if !options.no_validation {
                instance_info = instance_info.push_next(&mut validation_features);
            }
            self.instance = Some(self.entry.create_instance(&instance_info, None)?);
            let instance = self.instance.as_ref().unwrap();
            let debug = ash::ext::debug_utils::Instance::new(&self.entry, instance);
            self.messenger = debug.create_debug_utils_messenger(&debug_create, None)?;
            self.debug = Some(debug);
            println!(
                "Validation layer explicitly {}",
                if options.no_validation {
                    "disabled"
                } else {
                    "enabled: VK_LAYER_KHRONOS_validation; synchronization validation requested via pNext"
                }
            );
            self.debug.as_ref().unwrap().submit_debug_utils_message(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING, vk::DebugUtilsMessageTypeFlagsEXT::GENERAL, &vk::DebugUtilsMessengerCallbackDataEXT::default().message_id_name(CHECK).message(c"Theia diagnostic: debug callback delivery confirmed (intentional, excluded from totals)"));
            if self.validation.checks.load(Ordering::Relaxed) == 0 {
                return Err("Debug callback did not receive diagnostic".into());
            }
            self.surface_loader = Some(ash::khr::surface::Instance::new(&self.entry, instance));
            self.surface = ash_window::create_surface(
                &self.entry,
                instance,
                self.window.display_handle()?.as_raw(),
                self.window.window_handle()?.as_raw(),
                None,
            )?;
            let backend = match self.window.display_handle()?.as_raw() {
                RawDisplayHandle::Wayland(_) => "Wayland",
                RawDisplayHandle::Xlib(_) => "Xlib",
                RawDisplayHandle::Xcb(_) => "Xcb",
                RawDisplayHandle::Windows(_) => "Windows",
                _ => "Other",
            };
            println!(
                "Native surface created ({backend}); loader {}.{}.{}",
                vk::api_version_major(version),
                vk::api_version_minor(version),
                vk::api_version_patch(version)
            );
        }
        self.select_device(
            options.gpu.as_deref(),
            options.render_mode() == RenderMode::Spatial,
            matches!(
                options.render_mode(),
                RenderMode::Visibility | RenderMode::AssetResolve | RenderMode::Benchmark
            ),
            matches!(
                options.render_mode(),
                RenderMode::Asset
                    | RenderMode::AssetCamera
                    | RenderMode::AssetCull
                    | RenderMode::AssetLod
                    | RenderMode::AssetResolve
                    | RenderMode::Benchmark
            ),
            matches!(
                options.render_mode(),
                RenderMode::AssetCull | RenderMode::AssetLod | RenderMode::AssetResolve
            ),
            (options.texture_auto_mips, options.texture_linear_filter),
        )?;
        // SAFETY: Features were queried on the selected device; all queue indices support the intended
        // operations. Create-info backing storage remains live and handles immediately gain RAII ownership.
        unsafe {
            let instance = self.instance.as_ref().unwrap();
            let families = if self.graphics_family == self.present_family {
                vec![self.graphics_family]
            } else {
                vec![self.graphics_family, self.present_family]
            };
            let priorities = [1.0];
            let queues: Vec<_> = families
                .iter()
                .map(|&family| {
                    vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(family)
                        .queue_priorities(&priorities)
                })
                .collect();
            let extensions = [
                ash::khr::swapchain::NAME.as_ptr(),
                ash::ext::mesh_shader::NAME.as_ptr(),
                ash::ext::swapchain_maintenance1::NAME.as_ptr(),
            ];
            let mut v12 = vk::PhysicalDeviceVulkan12Features::default()
                .draw_indirect_count(true)
                .timeline_semaphore(true)
                .buffer_device_address(true)
                .runtime_descriptor_array(true)
                .descriptor_binding_partially_bound(true)
                .shader_sampled_image_array_non_uniform_indexing(true);
            let mut v13 = vk::PhysicalDeviceVulkan13Features::default()
                .dynamic_rendering(true)
                .synchronization2(true);
            let mut mesh = vk::PhysicalDeviceMeshShaderFeaturesEXT::default()
                .task_shader(true)
                .mesh_shader(true);
            let mut maintenance = vk::PhysicalDeviceSwapchainMaintenance1FeaturesEXT::default()
                .swapchain_maintenance1(true);
            let core_features = vk::PhysicalDeviceFeatures::default()
                .fragment_stores_and_atomics(options.texture_auto_mips);
            self.device = Some(
                instance.create_device(
                    self.physical,
                    &vk::DeviceCreateInfo::default()
                        .enabled_features(&core_features)
                        .queue_create_infos(&queues)
                        .enabled_extension_names(&extensions)
                        .push_next(&mut v12)
                        .push_next(&mut v13)
                        .push_next(&mut mesh)
                        .push_next(&mut maintenance),
                    None,
                )?,
            );
            let device = self.device.as_ref().unwrap();
            self.mesh_commands = Some(ash::ext::mesh_shader::Device::new(instance, device));
            self.graphics = device.get_device_queue(self.graphics_family, 0);
            self.present = device.get_device_queue(self.present_family, 0);
            self.pool = device.create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .queue_family_index(self.graphics_family),
                None,
            )?;
            self.command = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(self.pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )?[0];
            self.acquired = device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?;
            self.complete = device.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?;
        }
        Ok(())
    }
    fn select_device(
        &mut self,
        requested: Option<&str>,
        spatial: bool,
        visibility: bool,
        asset: bool,
        cull: bool,
        texture_profile: (bool, bool),
    ) -> Result<()> {
        let (footprint, linear_filter) = texture_profile;
        let instance = self.instance.as_ref().unwrap();
        let surface = self.surface_loader.as_ref().unwrap();
        let mut candidates = Vec::new();
        // SAFETY: Enumerated physical devices belong to the live instance. All output chains are
        // writable for the call; queue indices come from each device's queried family list.
        unsafe {
            for physical in instance.enumerate_physical_devices()? {
                let mut driver = vk::PhysicalDeviceDriverProperties::default();
                let mut limits = vk::PhysicalDeviceMeshShaderPropertiesEXT::default();
                let mut properties = vk::PhysicalDeviceProperties2::default()
                    .push_next(&mut driver)
                    .push_next(&mut limits);
                instance.get_physical_device_properties2(physical, &mut properties);
                let properties = properties.properties;
                let name = CStr::from_ptr(properties.device_name.as_ptr())
                    .to_string_lossy()
                    .into_owned();
                let mut v12 = vk::PhysicalDeviceVulkan12Features::default();
                let mut v13 = vk::PhysicalDeviceVulkan13Features::default();
                let mut mesh = vk::PhysicalDeviceMeshShaderFeaturesEXT::default();
                let mut maintenance = vk::PhysicalDeviceSwapchainMaintenance1FeaturesEXT::default();
                let mut features = vk::PhysicalDeviceFeatures2::default()
                    .push_next(&mut v12)
                    .push_next(&mut v13)
                    .push_next(&mut mesh)
                    .push_next(&mut maintenance);
                instance.get_physical_device_features2(physical, &mut features);
                let fragment_stores = features.features.fragment_stores_and_atomics == vk::TRUE;
                let extensions = instance.enumerate_device_extension_properties(physical)?;
                let visibility_formats = !visibility
                    || [
                        (
                            vk::Format::R32G32_UINT,
                            vk::FormatFeatureFlags::COLOR_ATTACHMENT
                                | vk::FormatFeatureFlags::SAMPLED_IMAGE,
                        ),
                        (
                            vk::Format::R8G8B8A8_UNORM,
                            vk::FormatFeatureFlags::SAMPLED_IMAGE
                                | vk::FormatFeatureFlags::TRANSFER_DST,
                        ),
                        (
                            vk::Format::D32_SFLOAT,
                            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT
                                | vk::FormatFeatureFlags::SAMPLED_IMAGE,
                        ),
                    ]
                    .iter()
                    .all(|(format, required)| {
                        instance
                            .get_physical_device_format_properties(physical, *format)
                            .optimal_tiling_features
                            .contains(*required)
                    });
                println!(
                    "GPU candidate: {name}; driver {} / {}",
                    CStr::from_ptr(driver.driver_name.as_ptr()).to_string_lossy(),
                    CStr::from_ptr(driver.driver_info.as_ptr()).to_string_lossy()
                );
                let checks = [
                    (
                        "optional sRGB linear sampling",
                        !linear_filter
                            || instance
                                .get_physical_device_format_properties(
                                    physical,
                                    vk::Format::R8G8B8A8_SRGB,
                                )
                                .optimal_tiling_features
                                .contains(vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR),
                    ),
                    (
                        "optional footprint probe profile",
                        !footprint
                            || (fragment_stores
                                && properties.limits.max_per_stage_descriptor_storage_buffers >= 4
                                && properties.limits.max_descriptor_set_storage_buffers >= 4
                                && properties.limits.max_storage_buffer_range >= 3888),
                    ),
                    ("visibility identity/depth formats", visibility_formats),
                    ("Vulkan 1.3", properties.api_version >= vk::API_VERSION_1_3),
                    (
                        "VK_KHR_swapchain",
                        has_extension(&extensions, ash::khr::swapchain::NAME),
                    ),
                    (
                        "VK_EXT_mesh_shader",
                        has_extension(&extensions, ash::ext::mesh_shader::NAME),
                    ),
                    ("meshShader", mesh.mesh_shader == vk::TRUE),
                    (
                        "mesh output/invocation limits",
                        limits.max_mesh_work_group_invocations >= if asset { 64 } else { 32 }
                            && limits.max_mesh_work_group_size[0] >= if asset { 64 } else { 32 }
                            && limits.max_mesh_output_vertices
                                >= if visibility && asset {
                                    192
                                } else if asset {
                                    64
                                } else {
                                    3
                                }
                            && limits.max_mesh_output_primitives >= if asset { 64 } else { 1 }
                            && (!asset
                                || (limits.max_mesh_output_memory_size
                                    >= if visibility && asset { 8192 } else { 4096 }
                                    && limits.max_mesh_output_components >= 7)),
                    ),
                    ("taskShader", mesh.task_shader == vk::TRUE),
                    (
                        "amplification limits",
                        limits.max_task_work_group_invocations >= 32
                            && limits.max_task_work_group_size[0] >= 32
                            && limits.max_task_payload_size >= 20
                            && limits.max_task_payload_and_shared_memory_size >= 20
                            && limits.max_mesh_payload_and_shared_memory_size >= 20
                            && limits.max_mesh_work_group_count[0]
                                >= if spatial || asset { 32 } else { 2 }
                            && limits.max_mesh_work_group_total_count
                                >= if spatial || asset { 32 } else { 2 },
                    ),
                    (
                        "spatial compute limits",
                        !spatial
                            || (properties.limits.max_compute_work_group_invocations >= 64
                                && properties.limits.max_compute_work_group_size[0] >= 64
                                && properties.limits.max_compute_shared_memory_size >= 524
                                && properties.limits.max_storage_buffer_range >= 9728),
                    ),
                    ("dynamicRendering", v13.dynamic_rendering == vk::TRUE),
                    (
                        "meshlet cull compute limits",
                        !cull
                            || (properties.limits.max_compute_work_group_invocations >= 32
                                && properties.limits.max_compute_work_group_size[0] >= 32
                                && properties.limits.max_compute_shared_memory_size >= 8
                                && properties.limits.max_storage_buffer_range >= 256
                                && properties.limits.max_push_constants_size >= 64),
                    ),
                    ("synchronization2", v13.synchronization2 == vk::TRUE),
                    ("timelineSemaphore", v12.timeline_semaphore == vk::TRUE),
                    ("drawIndirectCount", v12.draw_indirect_count == vk::TRUE),
                    ("bufferDeviceAddress", v12.buffer_device_address == vk::TRUE),
                    (
                        "runtimeDescriptorArray",
                        v12.runtime_descriptor_array == vk::TRUE,
                    ),
                    (
                        "descriptorBindingPartiallyBound",
                        v12.descriptor_binding_partially_bound == vk::TRUE,
                    ),
                    (
                        "shaderSampledImageArrayNonUniformIndexing",
                        v12.shader_sampled_image_array_non_uniform_indexing == vk::TRUE,
                    ),
                    (
                        "lab presentation fences",
                        has_extension(&extensions, ash::ext::swapchain_maintenance1::NAME)
                            && maintenance.swapchain_maintenance1 == vk::TRUE,
                    ),
                ];
                for (label, supported) in checks {
                    println!("  {label}: {supported}");
                }
                println!(
                    "  Mesh limits: {} vertices, {} primitives; task payload {} bytes; preferred task/mesh invocations {}/{}",
                    limits.max_mesh_output_vertices,
                    limits.max_mesh_output_primitives,
                    limits.max_task_payload_size,
                    limits.max_preferred_task_work_group_invocations,
                    limits.max_preferred_mesh_work_group_invocations
                );
                if checks.iter().any(|(_, supported)| !supported)
                    || requested
                        .is_some_and(|filter| !name.to_lowercase().contains(&filter.to_lowercase()))
                {
                    println!("  Rejected: missing requirement or GPU filter mismatch");
                    continue;
                }
                let families = instance.get_physical_device_queue_family_properties(physical);
                let graphics = families.iter().position(|family| {
                    family.queue_count > 0
                        && family
                            .queue_flags
                            .contains(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE)
                });
                let Some(graphics) = graphics.map(|index| index as u32) else {
                    continue;
                };
                let mut present = None;
                for (index, family) in families.iter().enumerate() {
                    if family.queue_count > 0
                        && surface.get_physical_device_surface_support(
                            physical,
                            index as u32,
                            self.surface,
                        )?
                    {
                        if present.is_none() {
                            present = Some(index as u32);
                        }
                        if index as u32 == graphics {
                            present = Some(graphics);
                            break;
                        }
                    }
                }
                let Some(present) = present else {
                    continue;
                };
                if surface
                    .get_physical_device_surface_formats(physical, self.surface)?
                    .is_empty()
                    || surface
                        .get_physical_device_surface_present_modes(physical, self.surface)?
                        .is_empty()
                {
                    continue;
                }
                let score = if properties.vendor_id == 0x1002 && properties.device_id == 0x744c {
                    1000
                } else if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                    100
                } else {
                    10
                };
                candidates.push((score, physical, graphics, present, name));
            }
        }
        let Some((_, physical, graphics, present, name)) =
            candidates.into_iter().max_by_key(|candidate| candidate.0)
        else {
            return Err("No GPU satisfies the Vulkan/mesh/task profile and Render Lab presentation-fence requirements (or --gpu filter)".into());
        };
        self.physical = physical;
        self.graphics_family = graphics;
        self.present_family = present;
        if linear_filter {
            // SAFETY: The selected physical device belongs to this live instance.
            let limits = unsafe { instance.get_physical_device_properties(physical) }.limits;
            println!(
                "Texture filter precision: {} fraction bits",
                limits.sub_texel_precision_bits
            );
        }
        println!(
            "Selected GPU: {name}; graphics/compute family {graphics}, present family {present}; presentation-fence retirement enabled"
        );
        Ok(())
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: Renderer retires the swapchain before Context destruction. Any successfully created
        // device may have submissions, so idle it before freeing command/sync objects and the device.
        unsafe {
            if let Some(device) = &self.device {
                if let Err(error) = device.device_wait_idle() {
                    eprintln!("Device cleanup wait failed: {error}");
                }
                if self.complete != vk::Fence::null() {
                    device.destroy_fence(self.complete, None);
                }
                if self.acquired != vk::Semaphore::null() {
                    device.destroy_semaphore(self.acquired, None);
                }
                if self.pool != vk::CommandPool::null() {
                    device.destroy_command_pool(self.pool, None);
                }
                device.destroy_device(None);
            }
            if self.surface != vk::SurfaceKHR::null() {
                self.surface_loader
                    .as_ref()
                    .unwrap()
                    .destroy_surface(self.surface, None);
            }
            if self.messenger != vk::DebugUtilsMessengerEXT::null() {
                self.debug
                    .as_ref()
                    .unwrap()
                    .destroy_debug_utils_messenger(self.messenger, None);
            }
            if let Some(instance) = &self.instance {
                instance.destroy_instance(None);
            }
        }
    }
}
struct Swapchain {
    loader: ash::khr::swapchain::Device,
    device: ash::Device,
    handle: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    views: Vec<vk::ImageView>,
    ready: Vec<vk::Semaphore>,
    fences: Vec<vk::Fence>,
    pending: Vec<bool>,
    extent: vk::Extent2D,
    pipeline: Option<Pipeline>,
    visibility: Option<crate::visibility::Target>,
    depth: Option<crate::visibility::Image>,
}
fn choose_extent(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    size: PhysicalSize<u32>,
) -> Option<vk::Extent2D> {
    if size.width == 0 || size.height == 0 {
        return None;
    }
    let extent = if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D {
            width: size.width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: size.height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    };
    (extent.width > 0 && extent.height > 0).then_some(extent)
}
impl Swapchain {
    fn new(
        ctx: &Context,
        old: vk::SwapchainKHR,
        size: PhysicalSize<u32>,
        shader_dir: Option<&std::path::Path>,
        mode: RenderMode,
        descriptor_layout: Option<vk::DescriptorSetLayout>,
        resolve_buffers: crate::visibility::ResolveBuffers,
    ) -> Result<Option<Self>> {
        let device = ctx.device.as_ref().unwrap();
        let surface = ctx.surface_loader.as_ref().unwrap();
        // SAFETY: Surface/device remain live through construction and throughout Swapchain lifetime;
        // partial construction owns every successful child immediately and is cleaned up by Drop.
        unsafe {
            let capabilities =
                surface.get_physical_device_surface_capabilities(ctx.physical, ctx.surface)?;
            let Some(extent) = choose_extent(&capabilities, size) else {
                return Ok(None);
            };
            if !capabilities
                .supported_usage_flags
                .contains(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            {
                return Err("Surface does not support color attachments".into());
            }
            let formats = surface.get_physical_device_surface_formats(ctx.physical, ctx.surface)?;
            let mut format = *formats
                .iter()
                .find(|format| {
                    format.format == vk::Format::B8G8R8A8_SRGB
                        && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                })
                .or_else(|| formats.first())
                .ok_or("Surface has no formats")?;
            if format.format == vk::Format::UNDEFINED {
                format.format = vk::Format::B8G8R8A8_SRGB;
            }
            let mut count = capabilities.min_image_count.saturating_add(1);
            if capabilities.max_image_count != 0 {
                count = count.min(capabilities.max_image_count);
            }
            let alpha = [
                vk::CompositeAlphaFlagsKHR::OPAQUE,
                vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
                vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
                vk::CompositeAlphaFlagsKHR::INHERIT,
            ]
            .into_iter()
            .find(|flag| capabilities.supported_composite_alpha.contains(*flag))
            .ok_or("Surface has no supported composite alpha")?;
            let families = [ctx.graphics_family, ctx.present_family];
            let mut info = vk::SwapchainCreateInfoKHR::default()
                .surface(ctx.surface)
                .min_image_count(count)
                .image_format(format.format)
                .image_color_space(format.color_space)
                .image_extent(extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .pre_transform(capabilities.current_transform)
                .composite_alpha(alpha)
                .present_mode(vk::PresentModeKHR::FIFO)
                .clipped(true)
                .old_swapchain(old);
            if ctx.graphics_family == ctx.present_family {
                info = info.image_sharing_mode(vk::SharingMode::EXCLUSIVE);
            } else {
                info = info
                    .image_sharing_mode(vk::SharingMode::CONCURRENT)
                    .queue_family_indices(&families);
            }
            let loader = ash::khr::swapchain::Device::new(ctx.instance.as_ref().unwrap(), device);
            let handle = loader.create_swapchain(&info, None)?;
            let mut chain = Self {
                loader,
                device: device.clone(),
                handle,
                images: Vec::new(),
                views: Vec::new(),
                ready: Vec::new(),
                fences: Vec::new(),
                pending: Vec::new(),
                extent,
                pipeline: None,
                visibility: None,
                depth: None,
            };
            if let Some(shader_dir) = shader_dir {
                if matches!(
                    mode,
                    RenderMode::AssetCamera | RenderMode::AssetCull | RenderMode::AssetLod
                ) {
                    let instance = ctx.instance.as_ref().unwrap();
                    if !instance
                        .get_physical_device_format_properties(ctx.physical, vk::Format::D32_SFLOAT)
                        .optimal_tiling_features
                        .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
                    {
                        return Err("Camera mode requires D32 depth attachment support".into());
                    }
                    let memory = instance.get_physical_device_memory_properties(ctx.physical);
                    chain.depth = Some(crate::visibility::Image::new(
                        device,
                        &memory,
                        extent,
                        vk::Format::D32_SFLOAT,
                        vk::ImageAspectFlags::DEPTH,
                        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                    )?);
                }
                if matches!(
                    mode,
                    RenderMode::Visibility | RenderMode::AssetResolve | RenderMode::Benchmark
                ) {
                    let memory = ctx
                        .instance
                        .as_ref()
                        .unwrap()
                        .get_physical_device_memory_properties(ctx.physical);
                    chain.visibility = Some(crate::visibility::Target::new(
                        device,
                        &memory,
                        extent,
                        format.format,
                        shader_dir,
                        (
                            descriptor_layout.ok_or("Visibility mode needs GPU work layout")?,
                            resolve_buffers,
                        ),
                        mode == RenderMode::AssetResolve,
                    )?);
                } else {
                    chain.pipeline = Some(Pipeline::new(
                        device,
                        format.format,
                        shader_dir,
                        mode,
                        descriptor_layout,
                    )?);
                }
            }
            chain.images = chain.loader.get_swapchain_images(handle)?;
            for &image in &chain.images {
                chain.views.push(
                    device.create_image_view(
                        &vk::ImageViewCreateInfo::default()
                            .image(image)
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .format(format.format)
                            .subresource_range(color_range()),
                        None,
                    )?,
                );
                chain
                    .ready
                    .push(device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?);
                chain
                    .fences
                    .push(device.create_fence(&vk::FenceCreateInfo::default(), None)?);
                chain.pending.push(false);
            }
            println!(
                "Swapchain: {}x{}, {:?}, FIFO, {} images",
                extent.width,
                extent.height,
                format.format,
                chain.images.len()
            );
            Ok(Some(chain))
        }
    }
    fn wait_present(&mut self, index: usize) -> Result<()> {
        if self.pending[index] {
            // SAFETY: Only queued presentations set pending. Their fence establishes retirement;
            // after the wait the fence is no longer in use and can be reset for the next presentation.
            unsafe {
                self.device
                    .wait_for_fences(&[self.fences[index]], true, u64::MAX)?;
                self.device.reset_fences(&[self.fences[index]])?;
            }
            self.pending[index] = false;
        }
        Ok(())
    }
    fn retire(&mut self) -> Result<()> {
        // SAFETY: Live device; all rendering work must finish before freeing views and semaphores.
        unsafe {
            self.device.device_wait_idle()?;
        }
        for index in 0..self.pending.len() {
            self.wait_present(index)?;
        }
        Ok(())
    }
}
impl Drop for Swapchain {
    fn drop(&mut self) {
        if let Err(error) = self.retire() {
            eprintln!("Swapchain retirement failed: {error}");
        }
        // SAFETY: Normal retirement waits rendering and presentation fences. Partial construction has
        // no submissions; after device loss no pending work can continue. Device/instance are still live.
        unsafe {
            for &fence in &self.fences {
                self.device.destroy_fence(fence, None);
            }
            for &semaphore in &self.ready {
                self.device.destroy_semaphore(semaphore, None);
            }
            for &view in &self.views {
                self.device.destroy_image_view(view, None);
            }
            self.loader.destroy_swapchain(self.handle, None);
        }
    }
}
fn color_range() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(1)
}
pub struct Renderer {
    // Rust drops fields in declaration order after Drop::drop. Keep graphics children
    // before work resources, and their device-owning context last as a lifetime backstop.
    chain: Option<Swapchain>,
    stream: Option<crate::streaming::Stream>,
    bundle_stream: Option<crate::bundle_streaming::Stream>,
    texture_refinement: Option<crate::texture_refinement::Refinement>,
    benchmark: Option<crate::benchmark::Recorder>,
    work: Option<IndirectWork>,
    proxy_work: Option<IndirectWork>,
    lod: Option<crate::indirect::Lod>,
    materials: Option<crate::asset::Buffer>,
    uvs: Option<crate::asset::Buffer>,
    texture: Option<crate::texture::ResidentTexture>,
    textures: Vec<crate::texture::ResidentTexture>,
    texture_table: Option<crate::asset::Buffer>,
    footprint: Option<crate::asset::Buffer>,
    texture_tail: Option<[u32; 4]>,
    authority: Option<[u8; 64]>,
    pub recreate: bool,
    generations: u64,
    shader_dir: Option<std::path::PathBuf>,
    mode: RenderMode,
    work_items: u32,
    cycle_work: bool,
    presented: u64,
    last_input: Option<u32>,
    probes: RecoveryProbes,
    ctx: Context,
}
impl Renderer {
    pub fn new(
        window: Arc<Window>,
        options: &Options,
        validation: Arc<Validation>,
    ) -> Result<Self> {
        let ctx = Context::new(window, options, validation)?;
        let mut renderer = Self {
            authority: if options.render_mode() == RenderMode::Authority {
                Some(
                    if options.authority_demo || options.network_address.is_some() {
                        [0; 64]
                    } else {
                        crate::authority::fixture(options.authority_alpha.unwrap_or(0.5))?.bytes
                    },
                )
            } else {
                None
            },
            ctx,
            chain: None,
            stream: None,
            bundle_stream: None,
            texture_refinement: None,
            benchmark: None,
            recreate: true,
            generations: 0,
            mode: options.render_mode(),
            work: None,
            proxy_work: None,
            lod: None,
            materials: None,
            uvs: None,
            texture: None,
            textures: Vec::new(),
            texture_table: None,
            footprint: None,
            texture_tail: None,
            work_items: options.work_items.unwrap_or(2),
            cycle_work: options.smoke,
            presented: 0,
            last_input: None,
            probes: RecoveryProbes::new(options.wsi_smoke),
            shader_dir: (options.render_mode() != RenderMode::Clear).then(|| {
                options.shader_dir.clone().unwrap_or_else(|| {
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build/shaders")
                })
            }),
        };
        if options.benchmark {
            // SAFETY: Live selected physical device; properties and queue family are immutable.
            let (properties, mesh_limits, bits) = unsafe {
                let instance = renderer.ctx.instance.as_ref().unwrap();
                let mut mesh_limits = vk::PhysicalDeviceMeshShaderPropertiesEXT::default();
                let mut properties =
                    vk::PhysicalDeviceProperties2::default().push_next(&mut mesh_limits);
                instance.get_physical_device_properties2(renderer.ctx.physical, &mut properties);
                let queues =
                    instance.get_physical_device_queue_family_properties(renderer.ctx.physical);
                (
                    properties.properties,
                    mesh_limits,
                    queues[renderer.ctx.graphics_family as usize].timestamp_valid_bits,
                )
            };
            let count = options.benchmark_config().players + crate::benchmark::STATIC_OBJECTS;
            if (renderer.mode == RenderMode::Benchmark || options.benchmark_mixed)
                && ((properties.limits.max_storage_buffer_range as u64)
                    < crate::benchmark::WORK_BYTES
                    || properties.limits.max_push_constants_size < 96
                    || mesh_limits.max_task_work_group_count[0] < count
                    || mesh_limits.max_task_work_group_total_count < count
                    || properties.limits.max_compute_work_group_count[0] < count.div_ceil(64)
                    || mesh_limits.max_mesh_output_vertices < 48
                    || mesh_limits.max_mesh_output_primitives < 16)
            {
                return Err(
                    "Selected device cannot support bounded benchmark work/mesh limits".into(),
                );
            }
            if options.benchmark_mixed
                && (properties.limits.max_per_stage_descriptor_storage_buffers < 5
                    || properties.limits.max_descriptor_set_storage_buffers < 5)
            {
                return Err("Mixed benchmark requires five fragment storage buffers".into());
            }
            let hud_storage = 5 + u32::from(options.benchmark_mixed);
            if options.benchmark_cooked
                && options.benchmark_config().overlay
                && (properties.limits.max_push_constants_size < 112
                    || properties.limits.max_per_stage_descriptor_storage_buffers < hud_storage
                    || properties.limits.max_descriptor_set_storage_buffers < hud_storage)
            {
                return Err("Cooked debug HUD exceeds push/storage limits for this workload; use --benchmark-no-overlay on this device".into());
            }
            // SAFETY: Vulkan's fixed device name is null terminated.
            let name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            let mut metadata = serde_json::json!({"device":name,"vendor_id":properties.vendor_id,"device_id":properties.device_id,"driver_version":properties.driver_version,"api_version":properties.api_version,"timestamp_period_ns":properties.limits.timestamp_period,"timestamp_valid_bits":bits,"validation":!options.no_validation,"debug_assertions":cfg!(debug_assertions),"work_requested_bytes":crate::benchmark::WORK_BYTES,"shader_dir":renderer.shader_dir});
            metadata["scene_kind"] = serde_json::json!(if options.benchmark_mixed {
                "mixed-world"
            } else if options.benchmark_cooked {
                "cooked-bundle"
            } else {
                "procedural"
            });
            if options.benchmark_cooked {
                metadata["work_requested_bytes"] = serde_json::Value::Null;
                metadata["initial_bundle"] = serde_json::json!(options.bundle);
                metadata["replacement_bundle"] = serde_json::json!(options.stream_bundle);
                metadata["streaming"] = serde_json::json!(options.stream_bundle.is_some());
                metadata["bundle_cpu_budget"] =
                    serde_json::json!(options.bundle_byte_budget.unwrap_or(64 * 1024 * 1024));
                metadata["bundle_gpu_budget"] =
                    serde_json::json!(options.bundle_gpu_budget.unwrap_or(32 * 1024 * 1024));
                metadata["texture_image_budget"] = serde_json::json!(
                    options
                        .texture_budget
                        .unwrap_or(theia_assets::texture::DEFAULT_BUDGET as u64)
                );
                metadata["tracked_allocation_scope"] = serde_json::json!(
                    "work and render targets; active/pending cooked buffers/images are charged separately by bundle budgets and excluded from this frame metric"
                );
            }
            if options.benchmark_mixed {
                metadata["proxy_work_requested_bytes"] =
                    serde_json::json!(crate::benchmark::WORK_BYTES);
                metadata["proxy_ground_model"] = serde_json::json!(
                    "fixed 16-metre triangular grid, bounded whole-world fixture"
                );
                metadata["visibility_identity"] = serde_json::json!(
                    "frame-local bit31 marks visual proxies; lower31 stores dense id+1; zero background"
                );
            }
            renderer.benchmark = Some(crate::benchmark::Recorder::new(
                renderer.ctx.device.as_ref().unwrap(),
                options.benchmark_config(),
                properties.limits.timestamp_period,
                bits,
                metadata,
            )?);
        }
        if renderer.mode == RenderMode::Authority {
            let mut mesh = vk::PhysicalDeviceMeshShaderPropertiesEXT::default();
            let mut props = vk::PhysicalDeviceProperties2::default().push_next(&mut mesh);
            // SAFETY: Physical device/instance are retained by the context; writable chain is live.
            unsafe {
                renderer
                    .ctx
                    .instance
                    .as_ref()
                    .unwrap()
                    .get_physical_device_properties2(renderer.ctx.physical, &mut props);
            }
            if props.properties.limits.max_push_constants_size < 64
                || mesh.max_mesh_output_vertices < 4
                || mesh.max_mesh_output_primitives < 2
                || mesh.max_mesh_work_group_count[0] < 3
                || mesh.max_mesh_work_group_total_count < 3
                || mesh.max_mesh_output_memory_size < 4 * 28 + 2 * 12
                || mesh.max_mesh_output_components < 7
            {
                return Err("Device cannot support bounded authority mesh/push profile".into());
            }
        }
        if matches!(
            renderer.mode,
            RenderMode::Indirect
                | RenderMode::Spatial
                | RenderMode::Visibility
                | RenderMode::Asset
                | RenderMode::AssetCamera
                | RenderMode::AssetCull
                | RenderMode::AssetLod
                | RenderMode::AssetResolve
                | RenderMode::Benchmark
        ) {
            // SAFETY: The selected physical device belongs to the retained live instance.
            let memory = unsafe {
                renderer
                    .ctx
                    .instance
                    .as_ref()
                    .unwrap()
                    .get_physical_device_memory_properties(renderer.ctx.physical)
            };
            let lod_pair = options
                .lod
                .as_ref()
                .map(|(fine, coarse)| crate::lod_reference::load(fine, coarse))
                .transpose()?;
            if let Some(pair) = &lod_pair {
                renderer.lod = Some(crate::indirect::Lod {
                    bounds: pair.bounds,
                    error: pair.error,
                    pixels: options.lod_pixels.unwrap_or(6.),
                    extent: [0; 2],
                    entry_fraction: if options.lod_hysteresis_test {
                        0.2
                    } else {
                        options.lod_hysteresis.unwrap_or(0.)
                    },
                    reset: true,
                    test_trajectory: options.lod_hysteresis_test,
                });
            }
            let cell_asset = if let Some(path) = &options.cell {
                let (stream, bytes) = crate::streaming::Stream::new(
                    path,
                    options.resident_budget.unwrap_or(16 * 1024 * 1024),
                )?;
                renderer.stream = Some(stream);
                Some(bytes)
            } else {
                None
            };
            let asset = if let Some(pair) = &lod_pair {
                Some(pair.fine.clone())
            } else {
                options
                    .asset
                    .as_deref()
                    .map(|path| {
                        crate::asset::select(
                            path,
                            options.fallback.as_deref(),
                            options.asset_budget.unwrap_or(theia_assets::DEFAULT_BUDGET),
                        )
                    })
                    .transpose()?
                    .or(cell_asset)
            };
            if let Some(bytes) = &asset {
                // SAFETY: Selected physical device belongs to the retained live instance.
                let limits = unsafe {
                    renderer
                        .ctx
                        .instance
                        .as_ref()
                        .unwrap()
                        .get_physical_device_properties(renderer.ctx.physical)
                        .limits
                };
                if let Some(pair) = &lod_pair
                    && (pair.coarse.len() > limits.max_storage_buffer_range as usize
                        || limits.max_push_constants_size < 112
                        || limits.max_per_stage_descriptor_storage_buffers < 3
                        || limits.max_descriptor_set_storage_buffers < 3)
                {
                    return Err("GPU LOD requires 112 compute push bytes and three storage descriptors within device ranges".into());
                }
                if bytes.len() > limits.max_storage_buffer_range as usize {
                    return Err("Cooked asset exceeds selected GPU storage-buffer range".into());
                }
                if matches!(
                    renderer.mode,
                    RenderMode::AssetCamera
                        | RenderMode::AssetCull
                        | RenderMode::AssetLod
                        | RenderMode::AssetResolve
                ) && limits.max_push_constants_size < 64
                {
                    return Err("Camera mode requires 64 mesh-stage push constant bytes".into());
                }
            }
            if let Some(stream) = &mut renderer.stream {
                // SAFETY: Live instance and selected physical device; descriptor offsets use its queried limit.
                let alignment = unsafe {
                    renderer
                        .ctx
                        .instance
                        .as_ref()
                        .unwrap()
                        .get_physical_device_properties(renderer.ctx.physical)
                        .limits
                        .min_storage_buffer_offset_alignment
                };
                stream.initialize_arena(
                    renderer.ctx.device.as_ref().unwrap(),
                    &memory,
                    alignment,
                )?;
            }
            if let Some(path) = &options.materials {
                let bytes = asset
                    .as_ref()
                    .ok_or("Authored materials need fixed geometry")?;
                let mesh = theia_assets::Mesh::decode(bytes, None, theia_assets::DEFAULT_BUDGET)?;
                let material = crate::asset::load_material(path, &mesh)?;
                // SAFETY: Selected physical device belongs to the retained live instance.
                let limit = unsafe {
                    renderer
                        .ctx
                        .instance
                        .as_ref()
                        .unwrap()
                        .get_physical_device_properties(renderer.ctx.physical)
                        .limits
                        .max_storage_buffer_range
                };
                if material.len() > limit as usize {
                    return Err("Material exceeds device storage range".into());
                }
                renderer.materials = Some(crate::asset::Buffer::upload(
                    renderer.ctx.device.as_ref().unwrap(),
                    &memory,
                    &material,
                    renderer.ctx.graphics_family,
                    renderer.ctx.graphics,
                    theia_assets::material::DEFAULT_BUDGET as u64,
                    None,
                )?);
            }
            if let Some(path) = &options.uvs {
                let bytes = asset.as_ref().ok_or("Authored UVs need fixed geometry")?;
                let mesh = theia_assets::Mesh::decode(bytes, None, theia_assets::DEFAULT_BUDGET)?;
                let uvs = crate::asset::load_uv(path, &mesh)?;
                // SAFETY: Selected physical device belongs to the retained live instance.
                let limit = unsafe {
                    renderer
                        .ctx
                        .instance
                        .as_ref()
                        .unwrap()
                        .get_physical_device_properties(renderer.ctx.physical)
                        .limits
                        .max_storage_buffer_range
                };
                if uvs.len() > limit as usize {
                    return Err("UV page exceeds device storage range".into());
                }
                renderer.uvs = Some(crate::asset::Buffer::upload(
                    renderer.ctx.device.as_ref().unwrap(),
                    &memory,
                    &uvs,
                    renderer.ctx.graphics_family,
                    renderer.ctx.graphics,
                    theia_assets::uv::DEFAULT_BUDGET as u64,
                    None,
                )?);
            }
            if let Some(path) = &options.texture {
                let bytes = if options.texture_mip_level.is_some() || options.texture_auto_mips {
                    crate::texture::load_mips(path)?
                } else {
                    crate::texture::load(path)?
                };
                let mips = (options.texture_mip_level.is_some() || options.texture_auto_mips)
                    .then(|| {
                        theia_assets::texture_mips::TextureMipChain::decode(
                            &bytes,
                            None,
                            theia_assets::texture::DEFAULT_BUDGET,
                        )
                    })
                    .transpose()?;
                let texture = if mips.is_none() {
                    Some(theia_assets::texture::Texture::decode(
                        &bytes,
                        None,
                        theia_assets::texture::DEFAULT_BUDGET,
                    )?)
                } else {
                    None
                };
                let (width, height, count) = if let Some(chain) = &mips {
                    (chain.width, chain.height, chain.levels.len() as u32)
                } else {
                    let tex = texture.as_ref().unwrap();
                    (tex.width, tex.height, 1)
                };
                // SAFETY: Selected physical device and instance are retained and live.
                let properties = unsafe {
                    renderer
                        .ctx
                        .instance
                        .as_ref()
                        .unwrap()
                        .get_physical_device_image_format_properties(
                            renderer.ctx.physical,
                            vk::Format::R8G8B8A8_SRGB,
                            vk::ImageType::TYPE_2D,
                            vk::ImageTiling::OPTIMAL,
                            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                            vk::ImageCreateFlags::empty(),
                        )?
                };
                if width > properties.max_extent.width
                    || height > properties.max_extent.height
                    || count > properties.max_mip_levels
                {
                    return Err("Resident texture exceeds device image limits".into());
                }
                let budget = options
                    .texture_budget
                    .unwrap_or(theia_assets::texture::DEFAULT_BUDGET as u64);
                renderer.texture = Some(if let Some(raw_budget) = options.texture_tail_budget {
                    let tail = theia_assets::texture_residency::MipTail::fit(
                        mips.as_ref().unwrap(),
                        raw_budget,
                    )?;
                    let source = tail.source_extent();
                    renderer.texture_tail = Some([
                        source[0],
                        source[1],
                        tail.first_level(),
                        tail.source_level_count(),
                    ]);
                    crate::texture::ResidentTexture::upload_tail(
                        renderer.ctx.device.as_ref().unwrap(),
                        &memory,
                        &tail,
                        renderer.ctx.graphics_family,
                        renderer.ctx.graphics,
                        budget,
                        options.texture_linear_filter,
                    )?
                } else if options.texture_auto_mips {
                    crate::texture::ResidentTexture::upload_auto_mips(
                        renderer.ctx.device.as_ref().unwrap(),
                        &memory,
                        mips.as_ref().unwrap(),
                        renderer.ctx.graphics_family,
                        renderer.ctx.graphics,
                        budget,
                        options.texture_linear_filter,
                    )?
                } else if let Some(chain) = &mips {
                    crate::texture::ResidentTexture::upload_mips(
                        renderer.ctx.device.as_ref().unwrap(),
                        &memory,
                        chain,
                        options.texture_mip_level.unwrap(),
                        renderer.ctx.graphics_family,
                        renderer.ctx.graphics,
                        budget,
                    )?
                } else {
                    crate::texture::ResidentTexture::upload(
                        renderer.ctx.device.as_ref().unwrap(),
                        &memory,
                        texture.as_ref().unwrap(),
                        renderer.ctx.graphics_family,
                        renderer.ctx.graphics,
                        budget,
                    )?
                });
            }
            if options.texture_auto_mips {
                let buffer = crate::asset::Buffer::allocate(
                    renderer.ctx.device.as_ref().unwrap(),
                    &memory,
                    3888,
                    vk::BufferUsageFlags::STORAGE_BUFFER,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    1024 * 1024,
                )?;
                println!(
                    "GPU footprint probe: 81 records, 3888 requested bytes, {} physical bytes; no runtime readback",
                    buffer.allocated_bytes
                );
                renderer.footprint = Some(buffer);
            }
            if let Some(path) = &options.texture_table {
                let mesh = theia_assets::Mesh::decode(
                    asset.as_ref().ok_or("Texture table needs fixed geometry")?,
                    None,
                    theia_assets::DEFAULT_BUDGET,
                )?;
                let bytes =
                    crate::asset::load_material(options.materials.as_deref().unwrap(), &mesh)?;
                let materials = theia_assets::material::MaterialPage::decode(
                    &bytes,
                    None,
                    &mesh,
                    theia_assets::material::DEFAULT_BUDGET,
                )?;
                let set = crate::texture::load_set(path, &materials)?;
                // SAFETY: Physical device and instance are retained and live.
                let properties = unsafe {
                    renderer
                        .ctx
                        .instance
                        .as_ref()
                        .unwrap()
                        .get_physical_device_properties(renderer.ctx.physical)
                };
                let count = set.images.len() as u32;
                if count + 4 > properties.limits.max_per_stage_descriptor_sampled_images
                    || count + 4 > properties.limits.max_descriptor_set_sampled_images
                    || set.table.len() > properties.limits.max_storage_buffer_range as usize
                {
                    return Err("Texture table exceeds device descriptor/storage limits".into());
                }
                // SAFETY: Same retained physical device; query the exact upload/sample profile.
                let image_limits = unsafe {
                    renderer
                        .ctx
                        .instance
                        .as_ref()
                        .unwrap()
                        .get_physical_device_image_format_properties(
                            renderer.ctx.physical,
                            vk::Format::R8G8B8A8_SRGB,
                            vk::ImageType::TYPE_2D,
                            vk::ImageTiling::OPTIMAL,
                            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                            vk::ImageCreateFlags::empty(),
                        )?
                };
                let budget = options
                    .texture_budget
                    .unwrap_or(theia_assets::texture::DEFAULT_BUDGET as u64);
                let mut remaining = budget;
                let mut pending = Vec::new();
                for bytes in &set.images {
                    let image = theia_assets::texture::Texture::decode(
                        bytes,
                        None,
                        theia_assets::texture::DEFAULT_BUDGET,
                    )?;
                    if image.width > image_limits.max_extent.width
                        || image.height > image_limits.max_extent.height
                    {
                        return Err("Resident texture exceeds device image limits".into());
                    }
                    let upload = crate::texture::Upload::begin(
                        renderer.ctx.device.as_ref().unwrap(),
                        &memory,
                        &image,
                        renderer.ctx.graphics_family,
                        renderer.ctx.graphics,
                        remaining,
                    )?;
                    remaining = remaining
                        .checked_sub(upload.resident_bytes())
                        .ok_or("Texture image aggregate budget exhausted")?;
                    pending.push(upload);
                }
                let ready = pending
                    .iter()
                    .map(|upload| upload.ready())
                    .collect::<Result<Vec<_>>>()?;
                println!(
                    "Complete texture set submitted before completion waits: {} uploads, {} staging allocation bytes, {} already signalled dedicated fences; {} physical image bytes within {budget}",
                    pending.len(),
                    pending.iter().map(|v| v.staging_bytes()).sum::<u64>(),
                    ready.iter().filter(|&&v| v).count(),
                    budget - remaining
                );
                for upload in pending {
                    renderer.textures.push(upload.finish()?);
                }
                renderer.texture_table = Some(crate::asset::Buffer::upload(
                    renderer.ctx.device.as_ref().unwrap(),
                    &memory,
                    &set.table,
                    renderer.ctx.graphics_family,
                    renderer.ctx.graphics,
                    theia_assets::texture_table::DEFAULT_BUDGET as u64,
                    None,
                )?);
                println!(
                    "Resident texture table ready: {count} hash-sorted images, {} aggregate physical image bytes within {budget}; immutable GPU material selection",
                    budget - remaining
                );
            }
            renderer.work = Some(IndirectWork::new(
                renderer.ctx.device.as_ref().unwrap(),
                &memory,
                renderer.shader_dir.as_ref().unwrap(),
                renderer.mode,
                asset.as_deref().map(|bytes| {
                    (
                        bytes,
                        renderer
                            .stream
                            .as_ref()
                            .and_then(|stream| stream.arena.as_ref()),
                    )
                }),
                (
                    renderer.ctx.graphics_family,
                    renderer.ctx.graphics,
                    if renderer.lod.is_some() {
                        options.lod_budget.unwrap_or(4 * 1024 * 1024)
                    } else if renderer.stream.is_some() {
                        options.resident_budget.unwrap_or(16 * 1024 * 1024)
                    } else {
                        u64::MAX
                    },
                ),
                lod_pair.as_ref().map(|v| v.coarse.as_slice()),
            )?);
            if options.benchmark_mixed {
                renderer.proxy_work = Some(IndirectWork::new(
                    renderer.ctx.device.as_ref().unwrap(),
                    &memory,
                    renderer.shader_dir.as_ref().unwrap(),
                    RenderMode::Benchmark,
                    None,
                    (
                        renderer.ctx.graphics_family,
                        renderer.ctx.graphics,
                        u64::MAX,
                    ),
                    None,
                )?);
            }
        }
        if let Some(path) = &options.stream_bundle {
            let stream = crate::bundle_streaming::Stream::new(
                [options.bundle.as_ref().unwrap().clone(), path.clone()],
                options.bundle_byte_budget.unwrap_or(64 * 1024 * 1024),
                options.bundle_gpu_budget.unwrap_or(32 * 1024 * 1024),
                options
                    .texture_budget
                    .unwrap_or(theia_assets::texture::DEFAULT_BUDGET as u64),
            )?;
            if renderer.bundle_resource_bytes() > stream.budget() {
                return Err(
                    "Initial complete bundle exceeds aggregate physical resource budget".into(),
                );
            }
            renderer.bundle_stream = Some(stream);
        }
        renderer.rebuild(renderer.ctx.window.inner_size())?;
        if let Some(target_budget) = options.texture_refine_budget {
            let path = options.texture.as_ref().unwrap();
            let bytes = crate::texture::load_mips(path)?;
            renderer.texture_refinement = Some(crate::texture_refinement::Refinement::new(
                path.clone(),
                theia_assets::content_id(&bytes),
                renderer.texture_tail.unwrap()[2],
                target_budget,
                options
                    .texture_budget
                    .unwrap_or(theia_assets::texture::DEFAULT_BUDGET as u64),
                options.texture_linear_filter,
            ));
            println!(
                "Mip refinement scheduler ready: original source verified; finer dense budget {target_budget}"
            );
        }
        Ok(renderer)
    }
    fn bundle_resource_bytes(&self) -> u64 {
        self.work.as_ref().map_or(0, IndirectWork::resident_bytes)
            + self.materials.as_ref().map_or(0, |v| v.allocated_bytes)
            + self.uvs.as_ref().map_or(0, |v| v.allocated_bytes)
            + self.texture_table.as_ref().map_or(0, |v| v.allocated_bytes)
            + self
                .textures
                .iter()
                .map(|v| v.image.allocated_bytes)
                .sum::<u64>()
    }
    fn tick_bundle(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        let active_bytes = self.bundle_resource_bytes();
        let active_image_bytes = self.textures.iter().map(|v| v.image.allocated_bytes).sum();
        let Some(stream) = &mut self.bundle_stream else {
            return Ok(());
        };
        // SAFETY: Context retains the selected physical device and its instance.
        let (memory, limits, image_limits) = unsafe {
            let instance = self.ctx.instance.as_ref().unwrap();
            (
                instance.get_physical_device_memory_properties(self.ctx.physical),
                instance
                    .get_physical_device_properties(self.ctx.physical)
                    .limits,
                instance.get_physical_device_image_format_properties(
                    self.ctx.physical,
                    vk::Format::R8G8B8A8_SRGB,
                    vk::ImageType::TYPE_2D,
                    vk::ImageTiling::OPTIMAL,
                    vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                    vk::ImageCreateFlags::empty(),
                )?,
            )
        };
        let ready = stream.tick(
            self.presented,
            active_bytes,
            active_image_bytes,
            crate::bundle_streaming::Gpu {
                device: self.ctx.device.as_ref().unwrap(),
                memory: &memory,
                family: self.ctx.graphics_family,
                queue: self.ctx.graphics,
                limits,
                image_limits,
            },
        )?;
        if let Some(ready) = ready {
            // The sole frame fence is retired. Retire presentation and destroy graphics
            // descriptors/pipelines before dropping their old resource owners. This
            // qualification rebuilds a full generation, including descriptor-count changes.
            if let Some(chain) = &mut self.chain {
                chain.retire()?;
            }
            drop(self.chain.take());
            let ([geometry, materials, uvs, table], images, publication) = ready.into_parts();
            drop(self.work.as_mut().unwrap().replace_asset(geometry));
            self.materials = Some(materials);
            self.uvs = Some(uvs);
            self.texture_table = Some(table);
            self.textures = images;
            self.recreate = true;
            self.rebuild(size)?;
            if self.recreate {
                return Err("Bundle publication target generation unavailable".into());
            }
            self.bundle_stream
                .as_mut()
                .unwrap()
                .published(publication, self.presented)?;
        }
        Ok(())
    }
    fn tick_texture_refinement(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        let Some(refinement) = &mut self.texture_refinement else {
            return Ok(());
        };
        if refinement.finished() {
            return Ok(());
        }
        // SAFETY: The selected physical device belongs to the live context.
        let memory = unsafe {
            self.ctx
                .instance
                .as_ref()
                .unwrap()
                .get_physical_device_memory_properties(self.ctx.physical)
        };
        let ready = refinement.tick(
            self.presented,
            self.texture.as_ref().unwrap().image.allocated_bytes,
            crate::texture_refinement::Gpu {
                device: self.ctx.device.as_ref().unwrap(),
                memory: &memory,
                family: self.ctx.graphics_family,
                queue: self.ctx.graphics,
            },
        )?;
        if let Some(ready) = ready {
            // The frame fence was retired by draw. Conservatively retire the old
            // presentation/graphics generation before releasing descriptor owners.
            if let Some(chain) = &mut self.chain {
                chain.retire()?;
            }
            drop(self.chain.take());
            self.texture = Some(ready.texture);
            self.texture_tail = Some(ready.metadata);
            self.recreate = true;
            self.rebuild(size)?;
            if self.recreate {
                return Err("Mip refinement publication generation unavailable".into());
            }
            self.texture_refinement
                .as_mut()
                .unwrap()
                .published(ready.metadata[2], self.presented);
        }
        Ok(())
    }
    fn rebuild(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        if let Some(chain) = &mut self.chain {
            chain.retire()?;
        }
        let old = self
            .chain
            .as_ref()
            .map_or(vk::SwapchainKHR::null(), |chain| chain.handle);
        if let Some(chain) = Swapchain::new(
            &self.ctx,
            old,
            size,
            self.shader_dir.as_deref(),
            self.mode,
            self.work.as_ref().map(|work| work.descriptor_layout),
            crate::visibility::ResolveBuffers {
                proxy_work: self
                    .proxy_work
                    .as_ref()
                    .map(|work| (work.descriptor_layout, work.benchmark_descriptor())),
                benchmark: self.mode == RenderMode::Benchmark,
                cooked_hud_work: self
                    .benchmark
                    .as_ref()
                    .filter(|b| self.mode == RenderMode::AssetResolve && b.config.overlay)
                    .map(|_| self.work.as_ref().unwrap().benchmark_descriptor()),
                materials: self.materials.as_ref().map(|buffer| {
                    vk::DescriptorBufferInfo::default()
                        .buffer(buffer.handle)
                        .offset(buffer.offset)
                        .range(buffer.bytes)
                }),
                texture: self.texture.as_ref().map(|texture| texture.descriptor()),
                texture_count: self.textures.len() as u32,
                texture_images: std::array::from_fn(|i| {
                    self.textures
                        .get(i)
                        .map_or(vk::DescriptorImageInfo::default(), |v| v.descriptor())
                }),
                footprint: self.footprint.as_ref().map(|buffer| {
                    vk::DescriptorBufferInfo::default()
                        .buffer(buffer.handle)
                        .offset(buffer.offset)
                        .range(buffer.bytes)
                }),
                mip_tail: self.texture_tail,
                texture_table: self.texture_table.as_ref().map(|buffer| {
                    vk::DescriptorBufferInfo::default()
                        .buffer(buffer.handle)
                        .offset(buffer.offset)
                        .range(buffer.bytes)
                }),
                uvs: self.uvs.as_ref().map(|buffer| {
                    vk::DescriptorBufferInfo::default()
                        .buffer(buffer.handle)
                        .offset(buffer.offset)
                        .range(buffer.bytes)
                }),
            },
        )? {
            self.chain = Some(chain);
            self.recreate = false;
            self.generations += 1;
            println!("Swapchain generation {} ready", self.generations);
        }
        Ok(())
    }
    pub fn authority_push(&mut self, bytes: [u8; 64]) -> Result<()> {
        if self.mode != RenderMode::Authority {
            return Err("Authority push requires the separate authority mode".into());
        }
        self.authority = Some(bytes);
        Ok(())
    }
    pub fn draw(&mut self, size: PhysicalSize<u32>) -> Result<bool> {
        let cpu_started = std::time::Instant::now();
        if size.width == 0 || size.height == 0 {
            self.recreate = true;
            return Ok(false);
        }
        if self.recreate {
            self.rebuild(size)?;
            if self.recreate {
                return Ok(false);
            }
        }
        let input = if self.cycle_work {
            ((self.presented / 20) % 3) as u32
        } else {
            self.work_items
        };
        if self.work.is_some()
            && self.last_input != Some(input)
            && self.mode != RenderMode::Benchmark
        {
            if matches!(
                self.mode,
                RenderMode::AssetCull | RenderMode::AssetLod | RenderMode::AssetResolve
            ) {
                println!(
                    "GPU frustum qualification view: {input} (0 translated empty, 1 narrow left, 2 full; compute selects meshlet IDs/counts)"
                );
            } else if matches!(
                self.mode,
                RenderMode::Asset
                    | RenderMode::AssetCamera
                    | RenderMode::AssetCull
                    | RenderMode::AssetLod
                    | RenderMode::AssetResolve
            ) {
                println!(
                    "Cooked mesh qualification scenario: {input} (0 empty, 1 first meshlet, 2 all; GPU reads asset header/counts; no count readback)"
                );
            } else if self.mode == RenderMode::Spatial {
                println!(
                    "Spatial qualification scenario: {input} (0 empty, 1 left/low, 2 full; GPU produces visibility/counts; no host readback)"
                );
            } else {
                println!(
                    "Indirect qualification input: {input} work items (GPU produces command/count; no host readback)"
                );
            }
            self.last_input = Some(input);
        }
        // SAFETY: This fence covers the only frame in flight. Retire old descriptor/buffer readers
        // before cell publication; no submitted command may observe a descriptor update.
        unsafe {
            self.ctx.device.as_ref().unwrap().wait_for_fences(
                &[self.ctx.complete],
                true,
                u64::MAX,
            )?;
        }
        if let Some(benchmark) = &mut self.benchmark {
            benchmark.collect()?;
        }
        if let Some(stream) = &mut self.stream {
            // SAFETY: Selected physical device belongs to the live instance retained in Context.
            let (memory, limit) = unsafe {
                let instance = self.ctx.instance.as_ref().unwrap();
                (
                    instance.get_physical_device_memory_properties(self.ctx.physical),
                    instance
                        .get_physical_device_properties(self.ctx.physical)
                        .limits
                        .max_storage_buffer_range,
                )
            };
            stream.tick(
                self.presented,
                self.work.as_mut().unwrap(),
                self.ctx.device.as_ref().unwrap(),
                &memory,
                (self.ctx.graphics_family, self.ctx.graphics),
                limit,
            )?;
        }
        self.tick_bundle(size)?;
        self.tick_texture_refinement(size)?;
        let cooked_stats = (self.benchmark.is_some() && self.mode == RenderMode::AssetResolve)
            .then(|| crate::benchmark::CookedStats {
                bundle_bytes: self.bundle_resource_bytes(),
                textures: self.textures.len() as u32,
                publications: self.bundle_stream.as_ref().map_or(0, |s| s.publications),
                deferrals: self.bundle_stream.as_ref().map_or(0, |s| s.rejections),
            });
        let chain = self.chain.as_mut().unwrap();
        let ctx = &self.ctx;
        let device = ctx.device.as_ref().unwrap();
        let benchmark_push = self
            .benchmark
            .as_ref()
            .map(|b| {
                crate::benchmark::push(
                    &b.config,
                    self.presented,
                    chain.extent.width,
                    chain.extent.height,
                )
            })
            .transpose()?;
        let camera_rows = if let Some((_, rows)) = benchmark_push {
            Some(rows)
        } else if matches!(
            self.mode,
            RenderMode::AssetCull | RenderMode::AssetLod | RenderMode::AssetResolve
        ) {
            Some(crate::camera::view_rows(
                chain.extent.width,
                chain.extent.height,
                input,
            )?)
        } else if self.mode == RenderMode::AssetCamera {
            Some(crate::camera::rows(
                chain.extent.width,
                chain.extent.height,
            )?)
        } else {
            None
        };
        if self.mode == RenderMode::Benchmark {
            chain
                .visibility
                .as_mut()
                .unwrap()
                .bind_geometry(self.work.as_ref().unwrap().benchmark_descriptor());
        }
        if self.mode == RenderMode::AssetResolve {
            chain
                .visibility
                .as_mut()
                .unwrap()
                .bind_geometry(self.work.as_ref().unwrap().geometry_descriptor());
        }
        // SAFETY: A single frame in flight; its fence protects command buffer/acquire semaphore reuse.
        // Image-specific presentation semaphores are reused only after acquire and a presentation-fence
        // wait. Barriers order color writes/presentation and depth reuse after earlier fragment tests.
        // Camera push constants match the qualified 64-byte mesh layout. Swapchain owns all referenced views.
        unsafe {
            // Inject before acquire so an out-of-date probe cannot leave its semaphore signaled.
            let acquire = if self.probes.hit(Probe::AcquireOutOfDate, self.presented) {
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR)
            } else {
                let result = chain.loader.acquire_next_image(
                    chain.handle,
                    u64::MAX,
                    ctx.acquired,
                    vk::Fence::null(),
                );
                if !matches!(result, Ok((_, false))) {
                    println!("WSI DRIVER acquire result: {result:?}");
                }
                result
            };
            let (index, mut suboptimal) = match acquire {
                Ok(value) => value,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.recreate = true;
                    return Ok(false);
                }
                Err(error) => return Err(error.into()),
            };
            suboptimal |= self.probes.hit(Probe::AcquireSuboptimal, self.presented);
            let i = index as usize;
            chain.wait_present(i)?;
            device.reset_command_buffer(ctx.command, vk::CommandBufferResetFlags::empty())?;
            device.begin_command_buffer(
                ctx.command,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            if let Some(benchmark) = &self.benchmark {
                benchmark.begin(ctx.command);
            }
            if let Some(buffer) = &self.footprint {
                let reuse = [vk::BufferMemoryBarrier2::default()
                    .src_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .buffer(buffer.handle)
                    .offset(buffer.offset)
                    .size(buffer.bytes)];
                device.cmd_pipeline_barrier2(
                    ctx.command,
                    &vk::DependencyInfo::default().buffer_memory_barriers(&reuse),
                );
            }
            if self.mode == RenderMode::Benchmark {
                let (push, _) = benchmark_push.as_ref().unwrap();
                self.work.as_ref().unwrap().generate_benchmark(
                    ctx.command,
                    push,
                    self.benchmark.as_ref().unwrap().config.players
                        + crate::benchmark::STATIC_OBJECTS,
                );
            } else if let Some(work) = &self.work {
                work.generate(
                    ctx.command,
                    input,
                    camera_rows,
                    self.lod.map(|mut lod| {
                        lod.extent = [chain.extent.width, chain.extent.height];
                        // One fixed view/page pair owns this buffer. Resizing keeps
                        // history; a fresh work buffer's first dispatch starts fine.
                        lod.reset = self.presented == 0;
                        if lod.test_trajectory {
                            let phase = ((self.presented / 20) % 8) as usize;
                            lod.pixels = [6., 7., 6., 4., 7., 6., 6., 4.][phase];
                            // Explicit qualification probe: a new history context
                            // throughout phase six must ignore old coarse state.
                            lod.reset |= phase == 6;
                        }
                        lod
                    }),
                );
            }
            if let Some(proxy) = &self.proxy_work {
                let mut push = benchmark_push.as_ref().unwrap().0;
                push[88..92].copy_from_slice(&1u32.to_ne_bytes());
                proxy.generate_benchmark(
                    ctx.command,
                    &push,
                    self.benchmark.as_ref().unwrap().config.players,
                );
            }
            if let Some(benchmark) = &self.benchmark {
                benchmark.mark(ctx.command, 1);
            }
            if let Some(depth) = &chain.depth {
                let barriers = [vk::ImageMemoryBarrier2::default()
                    .src_stage_mask(
                        vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                            | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                    )
                    .src_access_mask(
                        vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ
                            | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    )
                    .dst_stage_mask(
                        vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                            | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                    )
                    .dst_access_mask(
                        vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ
                            | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    )
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(depth.handle)
                    .subresource_range(depth.range())];
                device.cmd_pipeline_barrier2(
                    ctx.command,
                    &vk::DependencyInfo::default().image_memory_barriers(&barriers),
                );
            }
            if let Some(target) = &chain.visibility {
                target.record_identity(
                    ctx.command,
                    ctx.mesh_commands.as_ref().unwrap(),
                    self.work.as_ref().unwrap(),
                    self.proxy_work.as_ref(),
                    camera_rows,
                );
            }
            if let Some(benchmark) = &self.benchmark {
                benchmark.mark(ctx.command, 2);
            }
            let to_color = [vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags2::NONE)
                .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(chain.images[i])
                .subresource_range(color_range())];
            device.cmd_pipeline_barrier2(
                ctx.command,
                &vk::DependencyInfo::default().image_memory_barriers(&to_color),
            );
            let attachments = [vk::RenderingAttachmentInfo::default()
                .image_view(chain.views[i])
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.025, 0.07, 0.12, 1.0],
                    },
                })];
            let depth_attachment = chain.depth.as_ref().map(|depth| {
                vk::RenderingAttachmentInfo::default()
                    .image_view(depth.view)
                    .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.,
                            stencil: 0,
                        },
                    })
            });
            let mut rendering = vk::RenderingInfo::default()
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D::default(),
                    extent: chain.extent,
                })
                .layer_count(1)
                .color_attachments(&attachments);
            if let Some(depth) = &depth_attachment {
                rendering = rendering.depth_attachment(depth);
            }
            device.cmd_begin_rendering(ctx.command, &rendering);
            let overlay = self
                .benchmark
                .as_ref()
                .filter(|b| self.mode == RenderMode::Benchmark || b.config.overlay)
                .map(|b| {
                    let base = b.overlay(
                        self.presented,
                        self.work.as_ref().unwrap().allocated_bytes
                            + self.proxy_work.as_ref().map_or(0, |p| p.allocated_bytes)
                            + chain.visibility.as_ref().unwrap().allocated_bytes(),
                    );
                    let mut bytes = [0; 48];
                    bytes[..32].copy_from_slice(&base);
                    if let Some(stats) = cooked_stats {
                        for (i, value) in stats.hud_words().iter().enumerate() {
                            bytes[32 + i * 4..36 + i * 4].copy_from_slice(&value.to_ne_bytes());
                        }
                    }
                    bytes
                });
            if let Some(target) = &chain.visibility {
                target.record_resolve(
                    ctx.command,
                    ctx.mesh_commands.as_ref().unwrap(),
                    camera_rows,
                    overlay
                        .as_ref()
                        .map(|v| &v[..if cooked_stats.is_some() { 48 } else { 32 }]),
                );
            } else if let Some(pipeline) = &chain.pipeline {
                device.cmd_bind_pipeline(
                    ctx.command,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.handle,
                );
                device.cmd_set_viewport(
                    ctx.command,
                    0,
                    &[vk::Viewport::default()
                        .width(chain.extent.width as f32)
                        .height(chain.extent.height as f32)
                        .min_depth(0.0)
                        .max_depth(1.0)],
                );
                device.cmd_set_scissor(
                    ctx.command,
                    0,
                    &[vk::Rect2D {
                        offset: vk::Offset2D::default(),
                        extent: chain.extent,
                    }],
                );
                let commands = ctx.mesh_commands.as_ref().unwrap();
                if matches!(
                    self.mode,
                    RenderMode::AssetCamera | RenderMode::AssetCull | RenderMode::AssetLod
                ) {
                    let rows = camera_rows.unwrap();
                    let mut bytes = [0u8; 64];
                    for (i, value) in rows.iter().flatten().enumerate() {
                        bytes[i * 4..i * 4 + 4].copy_from_slice(&value.to_ne_bytes());
                    }
                    device.cmd_push_constants(
                        ctx.command,
                        pipeline.layout,
                        vk::ShaderStageFlags::MESH_EXT,
                        0,
                        &bytes,
                    );
                }
                if let Some(bytes) = &self.authority {
                    device.cmd_push_constants(
                        ctx.command,
                        pipeline.layout,
                        vk::ShaderStageFlags::MESH_EXT,
                        0,
                        bytes,
                    );
                    commands.cmd_draw_mesh_tasks(ctx.command, 3, 1, 1);
                } else if let Some(work) = &self.work {
                    work.draw(ctx.command, pipeline.layout, commands);
                } else {
                    commands.cmd_draw_mesh_tasks(ctx.command, 1, 1, 1);
                }
            }
            device.cmd_end_rendering(ctx.command);
            if let Some(benchmark) = &self.benchmark {
                benchmark.mark(ctx.command, 3);
            }
            let to_present = [vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(chain.images[i])
                .subresource_range(color_range())];
            device.cmd_pipeline_barrier2(
                ctx.command,
                &vk::DependencyInfo::default().image_memory_barriers(&to_present),
            );
            device.end_command_buffer(ctx.command)?;
            let waits = [vk::SemaphoreSubmitInfo::default()
                .semaphore(ctx.acquired)
                .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)];
            let signals = [vk::SemaphoreSubmitInfo::default()
                .semaphore(chain.ready[i])
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)];
            let commands = [vk::CommandBufferSubmitInfo::default().command_buffer(ctx.command)];
            // Reset only after successful acquisition/recording, never on the out-of-date early return.
            device.reset_fences(&[ctx.complete])?;
            device.queue_submit2(
                ctx.graphics,
                &[vk::SubmitInfo2::default()
                    .wait_semaphore_infos(&waits)
                    .signal_semaphore_infos(&signals)
                    .command_buffer_infos(&commands)],
                ctx.complete,
            )?;
            if let Some(stream) = &mut self.stream {
                stream.submitted();
            }
            let present_waits = [chain.ready[i]];
            let chains = [chain.handle];
            let indices = [index];
            let fences = [chain.fences[i]];
            let mut fence_info = vk::SwapchainPresentFenceInfoEXT::default().fences(&fences);
            let info = vk::PresentInfoKHR::default()
                .wait_semaphores(&present_waits)
                .swapchains(&chains)
                .image_indices(&indices)
                .push_next(&mut fence_info);
            let mut present = chain.loader.queue_present(ctx.present, &info);
            if !matches!(present, Ok(false)) {
                println!("WSI DRIVER present result: {present:?}");
            }
            // Override only successful real presents: semaphore waits and fence are already queued.
            // Never hide a real driver error or fabricate an unqueued presentation fence.
            if present.is_ok() {
                if self.probes.hit(Probe::PresentOutOfDate, self.presented) {
                    present = Err(vk::Result::ERROR_OUT_OF_DATE_KHR);
                } else if self.probes.hit(Probe::PresentSuboptimal, self.presented) {
                    present = Ok(true);
                }
            }
            let outcome = PresentOutcome::classify(present);
            chain.pending[i] = outcome.pending;
            self.recreate |= suboptimal || outcome.recreate;
            if let Some(error) = outcome.error {
                return Err(error.into());
            }
            if let Some(benchmark) = &mut self.benchmark {
                if !outcome.presented {
                    return Err("Benchmark presentation interrupted; report withheld".into());
                }
                benchmark.submitted(
                    self.presented,
                    cpu_started.elapsed().as_secs_f64() * 1000.,
                    [chain.extent.width, chain.extent.height],
                    self.work.as_ref().unwrap().allocated_bytes
                        + self.proxy_work.as_ref().map_or(0, |p| p.allocated_bytes)
                        + chain.visibility.as_ref().unwrap().allocated_bytes(),
                    cooked_stats,
                );
            }
            self.presented += u64::from(outcome.presented);
            Ok(outcome.presented)
        }
    }
    pub fn qualification_complete(&mut self) -> Result<()> {
        if let Some(benchmark) = &mut self.benchmark {
            // SAFETY: Live complete fence covers the last benchmark query recording.
            unsafe {
                self.ctx.device.as_ref().unwrap().wait_for_fences(
                    &[self.ctx.complete],
                    true,
                    u64::MAX,
                )?;
            }
            benchmark.finish(
                self.generations,
                self.bundle_stream
                    .as_ref()
                    .map(|s| (s.publications, s.rejections)),
            )?;
        }
        if let Some(stream) = &self.stream {
            println!(
                "Cell qualification: {} publications, {} physical-budget deferrals",
                stream.publications, stream.rejections
            );
            if self.cycle_work && stream.publications < 2 && stream.rejections == 0 {
                return Err(
                    "Cell smoke did not qualify page replacement or physical-budget deferral"
                        .into(),
                );
            }
        }
        if let Some(stream) = &self.bundle_stream {
            println!(
                "Bundle qualification: {} complete publications, {} budget deferrals",
                stream.publications, stream.rejections
            );
            if self.cycle_work && stream.publications < 2 && stream.rejections == 0 {
                return Err("Bundle smoke did not qualify replacement or budget deferral".into());
            }
        }
        if self.probes.remaining() != 0 {
            return Err("WSI qualification ended before all four recovery probes ran".into());
        }
        if let Some(refinement) = &self.texture_refinement {
            refinement.check_complete(self.presented)?;
        }
        Ok(())
    }
}
impl Drop for Renderer {
    fn drop(&mut self) {
        // Destroy graphics pipelines before their work descriptor layout, then work resources
        // before Context destroys the device. This also runs when new() fails in rebuild().
        drop(self.chain.take());
        drop(self.stream.take());
        drop(self.bundle_stream.take());
        drop(self.texture_refinement.take());
        drop(self.benchmark.take());
        drop(self.work.take());
        drop(self.proxy_work.take());
        drop(self.materials.take());
        drop(self.uvs.take());
        drop(self.texture.take());
        self.textures.clear();
        drop(self.texture_table.take());
        drop(self.footprint.take());
        println!(
            "Render Lab teardown: {} swapchain generations",
            self.generations
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_size_is_deferred_even_with_fixed_surface_extent() {
        let capabilities = vk::SurfaceCapabilitiesKHR {
            current_extent: vk::Extent2D {
                width: 960,
                height: 540,
            },
            ..Default::default()
        };
        assert!(choose_extent(&capabilities, PhysicalSize::new(0, 540)).is_none());
        assert!(choose_extent(&capabilities, PhysicalSize::new(960, 0)).is_none());
        assert_eq!(
            choose_extent(&capabilities, PhysicalSize::new(800, 600)),
            Some(capabilities.current_extent)
        );
    }
    #[test]
    fn variable_extent_is_clamped() {
        let capabilities = vk::SurfaceCapabilitiesKHR {
            current_extent: vk::Extent2D {
                width: u32::MAX,
                height: u32::MAX,
            },
            min_image_extent: vk::Extent2D {
                width: 64,
                height: 64,
            },
            max_image_extent: vk::Extent2D {
                width: 1920,
                height: 1080,
            },
            ..Default::default()
        };
        assert_eq!(
            choose_extent(&capabilities, PhysicalSize::new(32, 2160)),
            Some(vk::Extent2D {
                width: 64,
                height: 1080
            })
        );
    }
}
