//! Minimal task/mesh pipeline ownership. No conventional vertex/index buffers.
use crate::{RenderMode, Result};
use ash::vk;
use std::{fs::File, io::BufReader, path::Path};

pub(super) struct ShaderModule {
    device: ash::Device,
    pub(super) handle: vk::ShaderModule,
}
impl ShaderModule {
    pub(super) fn load(device: &ash::Device, path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|error| format!("Cannot open shader {}: {error}. Run python3 tools/compile_render_shaders.py or use --shader-dir / --clear-only", path.display()))?;
        let words = ash::util::read_spv(&mut BufReader::new(file))
            .map_err(|error| format!("Invalid SPIR-V {}: {error}", path.display()))?;
        // SAFETY: read_spv returns aligned words; offline compilation/spirv-val qualify their
        // contents. The device is live; Vulkan copies the shader code during creation.
        let handle = unsafe {
            device
                .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&words), None)?
        };
        Ok(Self {
            device: device.clone(),
            handle,
        })
    }
}
impl Drop for ShaderModule {
    fn drop(&mut self) {
        // SAFETY: The parent device outlives the module; pipeline creation has returned.
        unsafe {
            self.device.destroy_shader_module(self.handle, None);
        }
    }
}
pub struct Pipeline {
    device: ash::Device,
    pub handle: vk::Pipeline,
    pub layout: vk::PipelineLayout,
}
impl Pipeline {
    pub fn new(
        device: &ash::Device,
        format: vk::Format,
        shader_dir: &Path,
        mode: RenderMode,
        descriptor_layout: Option<vk::DescriptorSetLayout>,
    ) -> Result<Self> {
        let task_amplification = matches!(
            mode,
            RenderMode::Indirect | RenderMode::DirectTask | RenderMode::Spatial
        );
        let mesh_file = if mode == RenderMode::Spatial {
            "spatial.mesh.spv"
        } else if task_amplification {
            "triangle.amplified.mesh.spv"
        } else {
            "triangle.mesh.spv"
        };
        let mesh = ShaderModule::load(device, &shader_dir.join(mesh_file))?;
        let fragment = ShaderModule::load(device, &shader_dir.join("triangle.frag.spv"))?;
        let task = if task_amplification {
            Some(ShaderModule::load(
                device,
                &shader_dir.join(if mode == RenderMode::Spatial {
                    "spatial.task.spv"
                } else if mode == RenderMode::Indirect {
                    "triangle.indirect.task.spv"
                } else {
                    "triangle.task.spv"
                }),
            )?)
        } else {
            None
        };
        let mut pipeline = Self {
            device: device.clone(),
            handle: vk::Pipeline::null(),
            layout: vk::PipelineLayout::null(),
        };
        // SAFETY: Modules, create-info arrays and pNext data remain live through creation. Enabled
        // mesh features were checked by Context. Pipeline ownership is retained on every success;
        // on error Drop releases the layout and partial Vulkan pipeline outputs are destroyed below.
        unsafe {
            let layouts: Vec<_> = descriptor_layout.into_iter().collect();
            pipeline.layout = device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default().set_layouts(&layouts),
                None,
            )?;
            // Slang's SPIR-V entry-point name is "main", independently of source function names.
            let mut stages = vec![
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::MESH_EXT)
                    .module(mesh.handle)
                    .name(c"main"),
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(fragment.handle)
                    .name(c"main"),
            ];
            if let Some(task) = &task {
                stages.insert(
                    0,
                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::TASK_EXT)
                        .module(task.handle)
                        .name(c"main"),
                );
            }
            let viewport = vk::PipelineViewportStateCreateInfo::default()
                .viewport_count(1)
                .scissor_count(1);
            let raster = vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .line_width(1.0);
            let multisample = vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            let attachments = [vk::PipelineColorBlendAttachmentState::default()
                .color_write_mask(vk::ColorComponentFlags::RGBA)];
            let blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&attachments);
            let dynamics = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamics);
            let formats = [format];
            let mut rendering =
                vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&formats);
            // Mesh pipelines omit vertex input and input assembly. No depth attachment in this slice.
            let info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&stages)
                .viewport_state(&viewport)
                .rasterization_state(&raster)
                .multisample_state(&multisample)
                .color_blend_state(&blend)
                .dynamic_state(&dynamic)
                .layout(pipeline.layout)
                .push_next(&mut rendering);
            match device.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None) {
                Ok(handles) => pipeline.handle = handles[0],
                Err((handles, error)) => {
                    for handle in handles {
                        if handle != vk::Pipeline::null() {
                            device.destroy_pipeline(handle, None);
                        }
                    }
                    return Err(format!("Slang graphics pipeline creation failed: {error}").into());
                }
            }
        }
        if mode == RenderMode::Spatial {
            println!(
                "GPU spatial pipeline created: CSR candidates -> bounds culling -> indirect task/mesh visibility; up to 32 mesh groups"
            );
        } else if mode == RenderMode::Indirect {
            println!(
                "GPU indirect graphics pipeline created: task reads compute-generated work descriptor; mesh uses shared transforms"
            );
        } else if task_amplification {
            println!(
                "Task amplification pipeline created: TaskEXT -> 2 MeshEXT groups -> fragment; 32 lanes per stage, 20-byte transform payload; no vertex/index buffers"
            );
        } else {
            println!(
                "Mesh-only triangle pipeline created: MeshEXT + fragment, 32 lanes, 3 vertices, 1 primitive; no vertex/index buffers"
            );
        }
        Ok(pipeline)
    }
}
impl Drop for Pipeline {
    fn drop(&mut self) {
        // SAFETY: Swapchain retirement completes submissions before this owner is dropped;
        // during failed construction no command buffer has referenced the handles.
        unsafe {
            if self.handle != vk::Pipeline::null() {
                self.device.destroy_pipeline(self.handle, None);
            }
            if self.layout != vk::PipelineLayout::null() {
                self.device.destroy_pipeline_layout(self.layout, None);
            }
        }
    }
}
