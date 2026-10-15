//! Identity/depth targets and material resolve, owned by one swapchain generation.
use crate::{Result, indirect::IndirectWork, pipeline::Pipeline};
use ash::vk;
use std::path::Path;
pub(super) struct Image {
    device: ash::Device,
    pub(super) handle: vk::Image,
    memory: vk::DeviceMemory,
    pub(super) view: vk::ImageView,
    aspect: vk::ImageAspectFlags,
    pub(super) allocated_bytes: u64,
    mip_levels: u32,
}
impl Image {
    pub(super) fn new(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        extent: vk::Extent2D,
        format: vk::Format,
        aspect: vk::ImageAspectFlags,
        usage: vk::ImageUsageFlags,
    ) -> Result<Self> {
        Self::bounded(device, memory, extent, format, aspect, usage, u64::MAX)
    }
    pub(super) fn bounded(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        extent: vk::Extent2D,
        format: vk::Format,
        aspect: vk::ImageAspectFlags,
        usage: vk::ImageUsageFlags,
        budget: u64,
    ) -> Result<Self> {
        Self::allocate(
            device,
            memory,
            extent,
            format,
            aspect,
            usage,
            (budget, 1, Some(0)),
        )
    }
    pub(super) fn bounded_mips(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        extent: vk::Extent2D,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        budget: u64,
        levels: (u32, Option<u32>),
    ) -> Result<Self> {
        Self::allocate(
            device,
            memory,
            extent,
            format,
            vk::ImageAspectFlags::COLOR,
            usage,
            (budget, levels.0, levels.1),
        )
    }
    fn allocate(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        extent: vk::Extent2D,
        format: vk::Format,
        aspect: vk::ImageAspectFlags,
        usage: vk::ImageUsageFlags,
        allocation: (u64, u32, Option<u32>),
    ) -> Result<Self> {
        let (budget, mip_levels, view_level) = allocation;
        if mip_levels == 0 || mip_levels > 11 || view_level.is_some_and(|level| level >= mip_levels)
        {
            return Err("Invalid image mip allocation/view range".into());
        }
        let mut image = Self {
            device: device.clone(),
            handle: vk::Image::null(),
            memory: vk::DeviceMemory::null(),
            view: vk::ImageView::null(),
            aspect,
            allocated_bytes: 0,
            mip_levels,
        };
        // SAFETY: Device and create-info storage are live. Each successful handle is immediately retained.
        unsafe {
            image.handle = device.create_image(
                &vk::ImageCreateInfo::default()
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(format)
                    .extent(vk::Extent3D {
                        width: extent.width,
                        height: extent.height,
                        depth: 1,
                    })
                    .mip_levels(mip_levels)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )?;
            let requirements = device.get_image_memory_requirements(image.handle);
            if requirements.size > budget {
                return Err("Texture image allocation exceeds physical byte budget".into());
            }
            image.allocated_bytes = requirements.size;
            let index = (0..memory.memory_type_count)
                .find(|&i| {
                    requirements.memory_type_bits & (1 << i) != 0
                        && memory.memory_types[i as usize]
                            .property_flags
                            .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
                })
                .ok_or("No device-local visibility image memory")?;
            image.memory = device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(index),
                None,
            )?;
            device.bind_image_memory(image.handle, image.memory, 0)?;
            image.view = device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(image.handle)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .subresource_range(view_level.map_or(image.range(), |level| {
                        image.range().base_mip_level(level).level_count(1)
                    })),
                None,
            )?;
        }
        Ok(image)
    }
    pub(super) fn range(&self) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::default()
            .aspect_mask(self.aspect)
            .level_count(self.mip_levels)
            .layer_count(1)
    }
}
impl Drop for Image {
    fn drop(&mut self) {
        // SAFETY: Swapchain retirement has completed all users; parent device remains alive.
        unsafe {
            if self.view != vk::ImageView::null() {
                self.device.destroy_image_view(self.view, None);
            }
            if self.handle != vk::Image::null() {
                self.device.destroy_image(self.handle, None);
            }
            if self.memory != vk::DeviceMemory::null() {
                self.device.free_memory(self.memory, None);
            }
        }
    }
}
#[derive(Clone, Copy, Default)]
pub struct ResolveBuffers {
    pub world_geometry: Option<(vk::DescriptorSetLayout, vk::DescriptorBufferInfo)>,
    pub proxy_work: Option<(vk::DescriptorSetLayout, vk::DescriptorBufferInfo)>,
    pub cooked_hud_work: Option<vk::DescriptorBufferInfo>,
    pub benchmark: bool,
    pub materials: Option<vk::DescriptorBufferInfo>,
    pub uvs: Option<vk::DescriptorBufferInfo>,
    pub texture: Option<vk::DescriptorImageInfo>,
    pub texture_table: Option<vk::DescriptorBufferInfo>,
    pub footprint: Option<vk::DescriptorBufferInfo>,
    pub mip_tail: Option<[u32; 4]>,
    pub texture_images: [vk::DescriptorImageInfo; theia_assets::texture_table::MAX_TEXTURES],
    pub texture_count: u32,
}
impl ResolveBuffers {
    fn image_count(&self) -> u32 {
        if self.texture_table.is_some() {
            self.texture_count
        } else {
            u32::from(self.texture.is_some())
        }
    }
}
pub struct Target {
    proxy_visibility: Option<Pipeline>,
    world_visibility: Option<Pipeline>,
    visibility: Option<Pipeline>,
    resolve: Option<Pipeline>,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
    set: vk::DescriptorSet,
    identity: Image,
    depth: Image,
    extent: vk::Extent2D,
    materials: [Image; 2],
    sampler: vk::Sampler,
    device: ash::Device,
    cooked: bool,
    benchmark: bool,
    cooked_hud: bool,
    authored: bool,
    uv: bool,
    geometry: Option<(vk::Buffer, u64, u64)>,
    mip_tail: Option<[u32; 4]>,
}
impl Target {
    pub fn new(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        extent: vk::Extent2D,
        color_format: vk::Format,
        shaders: &Path,
        work_info: (vk::DescriptorSetLayout, ResolveBuffers),
        cooked: bool,
    ) -> Result<Self> {
        let (work_layout, buffers) = work_info;
        let material_info = buffers.materials;
        let uv = buffers.uvs.is_some();
        let authored = material_info.is_some();
        let image_count = buffers.image_count();
        if buffers.footprint.is_some()
            && (buffers.texture_table.is_some() || buffers.texture.is_none() || !authored || !uv)
        {
            return Err("Automatic mip probes require fixed geometry/material/UV and one full-chain texture".into());
        }
        let extra_storage = buffers.texture_table.or(buffers.footprint);
        if buffers.mip_tail.is_some() && buffers.footprint.is_none() {
            return Err("Mip-tail resolve requires automatic mip probes".into());
        }
        if buffers.texture_table.is_some()
            && !(1..=theia_assets::texture_table::MAX_TEXTURES as u32).contains(&image_count)
        {
            return Err("Invalid bounded texture descriptor count".into());
        }
        let identity = Image::new(
            device,
            memory,
            extent,
            vk::Format::R32G32_UINT,
            vk::ImageAspectFlags::COLOR,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::COLOR_ATTACHMENT,
        )?;
        let depth = Image::new(
            device,
            memory,
            extent,
            vk::Format::D32_SFLOAT,
            vk::ImageAspectFlags::DEPTH,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        )?;
        let material = || {
            Image::new(
                device,
                memory,
                vk::Extent2D {
                    width: 1,
                    height: 1,
                },
                vk::Format::R8G8B8A8_UNORM,
                vk::ImageAspectFlags::COLOR,
                vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            )
        };
        let materials = [material()?, material()?];
        let mut target = Self {
            proxy_visibility: None,
            world_visibility: None,
            visibility: None,
            resolve: None,
            pool: vk::DescriptorPool::null(),
            layout: vk::DescriptorSetLayout::null(),
            set: vk::DescriptorSet::null(),
            identity,
            depth,
            extent,
            materials,
            sampler: vk::Sampler::null(),
            device: device.clone(),
            cooked,
            benchmark: buffers.benchmark,
            cooked_hud: buffers.cooked_hud_work.is_some(),
            authored,
            uv,
            geometry: None,
            mip_tail: buffers.mip_tail,
        };
        // SAFETY: Images/layouts are live, arrays are retained through calls and handles gain RAII immediately.
        unsafe {
            target.sampler = device.create_sampler(
                &vk::SamplerCreateInfo::default()
                    .mag_filter(vk::Filter::NEAREST)
                    .min_filter(vk::Filter::NEAREST)
                    .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                    .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .max_lod(0.0),
                None,
            )?;
            let mut bindings: Vec<_> = (0..4)
                .map(|binding| {
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(binding)
                        .descriptor_type(if binding == 3 {
                            vk::DescriptorType::SAMPLER
                        } else {
                            vk::DescriptorType::SAMPLED_IMAGE
                        })
                        .descriptor_count(if binding == 2 { 2 } else { 1 })
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                })
                .collect();
            if cooked || buffers.benchmark {
                bindings.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(4)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                );
            }
            if authored {
                bindings.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(5)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                );
            }
            if uv {
                bindings.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(6)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                );
            }
            if image_count > 0 {
                for (binding, ty) in [
                    (7, vk::DescriptorType::SAMPLED_IMAGE),
                    (8, vk::DescriptorType::SAMPLER),
                ] {
                    bindings.push(
                        vk::DescriptorSetLayoutBinding::default()
                            .binding(binding)
                            .descriptor_type(ty)
                            .descriptor_count(if binding == 7 { image_count } else { 1 })
                            .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                    );
                }
            }
            if extra_storage.is_some() {
                bindings.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(9)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                );
            }
            if buffers.cooked_hud_work.is_some() {
                bindings.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(10)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                );
            }
            if buffers.proxy_work.is_some() {
                bindings.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(11)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                );
            }
            if buffers.world_geometry.is_some() {
                bindings.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(12)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                );
            }
            let mut flags = vec![
                vk::DescriptorBindingFlags::empty(),
                vk::DescriptorBindingFlags::empty(),
                vk::DescriptorBindingFlags::PARTIALLY_BOUND,
                vk::DescriptorBindingFlags::empty(),
            ];
            if cooked || buffers.benchmark {
                flags.push(vk::DescriptorBindingFlags::empty());
            }
            if authored {
                flags.push(vk::DescriptorBindingFlags::empty());
            }
            if uv {
                flags.push(vk::DescriptorBindingFlags::empty());
            }
            if image_count > 0 {
                flags.extend([vk::DescriptorBindingFlags::empty(); 2]);
            }
            if extra_storage.is_some() {
                flags.push(vk::DescriptorBindingFlags::empty());
            }
            if buffers.cooked_hud_work.is_some() {
                flags.push(vk::DescriptorBindingFlags::empty());
            }
            if buffers.proxy_work.is_some() {
                flags.push(vk::DescriptorBindingFlags::empty());
            }
            if buffers.world_geometry.is_some() {
                flags.push(vk::DescriptorBindingFlags::empty());
            }
            let mut binding_flags =
                vk::DescriptorSetLayoutBindingFlagsCreateInfo::default().binding_flags(&flags);
            target.layout = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(&bindings)
                    .push_next(&mut binding_flags),
                None,
            )?;
            let mut sizes = vec![
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(4 + image_count),
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::SAMPLER)
                    .descriptor_count(1 + u32::from(image_count > 0)),
            ];
            if cooked || buffers.benchmark {
                sizes.push(
                    vk::DescriptorPoolSize::default()
                        .ty(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(
                            1 + u32::from(authored)
                                + u32::from(uv)
                                + u32::from(extra_storage.is_some())
                                + u32::from(buffers.cooked_hud_work.is_some())
                                + u32::from(buffers.proxy_work.is_some())
                                + u32::from(buffers.world_geometry.is_some()),
                        ),
                );
            }
            target.pool = device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .max_sets(1)
                    .pool_sizes(&sizes),
                None,
            )?;
            let layouts = [target.layout];
            target.set = device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(target.pool)
                    .set_layouts(&layouts),
            )?[0];
            let images = [
                [vk::DescriptorImageInfo::default()
                    .image_view(target.identity.view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)],
                [vk::DescriptorImageInfo::default()
                    .image_view(target.depth.view)
                    .image_layout(vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL)],
            ];
            let mut writes: Vec<_> = images
                .iter()
                .enumerate()
                .map(|(binding, image)| {
                    vk::WriteDescriptorSet::default()
                        .dst_set(target.set)
                        .dst_binding(binding as u32)
                        .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                        .image_info(image)
                })
                .collect();
            let material_infos: Vec<_> = target
                .materials
                .iter()
                .map(|image| {
                    vk::DescriptorImageInfo::default()
                        .image_view(image.view)
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                })
                .collect();
            let sampler = [vk::DescriptorImageInfo::default().sampler(target.sampler)];
            writes.push(
                vk::WriteDescriptorSet::default()
                    .dst_set(target.set)
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&material_infos),
            );
            writes.push(
                vk::WriteDescriptorSet::default()
                    .dst_set(target.set)
                    .dst_binding(3)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&sampler),
            );
            let material_buffers = material_info.map(|info| [info]);
            if let Some(info) = &material_buffers {
                writes.push(
                    vk::WriteDescriptorSet::default()
                        .dst_set(target.set)
                        .dst_binding(5)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(info),
                );
            }
            let uv_buffers = buffers.uvs.map(|info| [info]);
            if let Some(info) = &uv_buffers {
                writes.push(
                    vk::WriteDescriptorSet::default()
                        .dst_set(target.set)
                        .dst_binding(6)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(info),
                );
            }
            let texture_images = if buffers.texture_table.is_some() {
                buffers.texture_images[..image_count as usize].to_vec()
            } else {
                buffers.texture.into_iter().collect::<Vec<_>>()
            };
            if !texture_images.is_empty() {
                writes.push(
                    vk::WriteDescriptorSet::default()
                        .dst_set(target.set)
                        .dst_binding(7)
                        .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                        .image_info(&texture_images),
                );
                writes.push(
                    vk::WriteDescriptorSet::default()
                        .dst_set(target.set)
                        .dst_binding(8)
                        .descriptor_type(vk::DescriptorType::SAMPLER)
                        .image_info(&texture_images[..1]),
                );
            }
            let texture_tables = extra_storage.map(|v| [v]);
            if let Some(info) = &texture_tables {
                writes.push(
                    vk::WriteDescriptorSet::default()
                        .dst_set(target.set)
                        .dst_binding(9)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(info),
                );
            }
            let cooked_work = buffers.cooked_hud_work.map(|v| [v]);
            if let Some(info) = &cooked_work {
                writes.push(
                    vk::WriteDescriptorSet::default()
                        .dst_set(target.set)
                        .dst_binding(10)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(info),
                );
            }
            let proxy_work = buffers.proxy_work.map(|(_, v)| [v]);
            if let Some(info) = &proxy_work {
                writes.push(
                    vk::WriteDescriptorSet::default()
                        .dst_set(target.set)
                        .dst_binding(11)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(info),
                );
            }
            let world_geometry = buffers.world_geometry.map(|(_, buffer)| [buffer]);
            if let Some(info) = &world_geometry {
                writes.push(
                    vk::WriteDescriptorSet::default()
                        .dst_set(target.set)
                        .dst_binding(12)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(info),
                );
            }
            device.update_descriptor_sets(&writes, &[]);
        }
        target.visibility = Some(if buffers.benchmark {
            Pipeline::benchmark(device, vk::Format::R32G32_UINT, shaders, work_layout, false)?
        } else {
            Pipeline::visibility(device, shaders, work_layout, cooked)?
        });
        if let Some((layout, _)) = buffers.proxy_work {
            target.proxy_visibility = Some(Pipeline::benchmark(
                device,
                vk::Format::R32G32_UINT,
                shaders,
                layout,
                false,
            )?);
        }
        if let Some((layout, _)) = buffers.world_geometry {
            target.world_visibility =
                Some(Pipeline::world_pair_visibility(device, shaders, layout)?);
        }
        if buffers.benchmark {
            target.resolve = Some(Pipeline::benchmark(
                device,
                color_format,
                shaders,
                target.layout,
                true,
            )?);
            return Ok(target);
        }
        target.resolve = Some(Pipeline::resolve(
            device,
            color_format,
            shaders,
            target.layout,
            crate::pipeline::ResolveMode {
                world_pair: buffers.world_geometry.is_some(),
                mixed: buffers.proxy_work.is_some(),
                cooked_hud: buffers.cooked_hud_work.is_some(),
                cooked,
                authored,
                uv,
                texture: image_count > 0,
                texture_table: buffers.texture_table.is_some(),
                automatic_mips: buffers.footprint.is_some(),
                mip_tail: buffers.mip_tail.is_some(),
            },
        )?);
        println!(
            "Visibility targets ready: R32G32_UINT {}/primitive + D32_SFLOAT depth; mesh material resolve",
            if cooked { "meshlet" } else { "instance" }
        );
        Ok(target)
    }
    pub fn bind_geometry(&mut self, info: vk::DescriptorBufferInfo) {
        let key = (info.buffer, info.offset, info.range);
        if self.geometry == Some(key) {
            return;
        }
        // SAFETY: Called after the sole frame fence; work owns this uploaded range until subsequent retirement.
        unsafe {
            self.device.update_descriptor_sets(
                &[vk::WriteDescriptorSet::default()
                    .dst_set(self.set)
                    .dst_binding(4)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[info])],
                &[],
            );
        }
        self.geometry = Some(key);
    }
    pub fn allocated_bytes(&self) -> u64 {
        self.identity.allocated_bytes
            + self.depth.allocated_bytes
            + self
                .materials
                .iter()
                .map(|v| v.allocated_bytes)
                .sum::<u64>()
    }
    fn push_rows(
        &self,
        command: vk::CommandBuffer,
        layout: vk::PipelineLayout,
        stage: vk::ShaderStageFlags,
        rows: [[f32; 4]; 4],
    ) {
        // SAFETY: Sixteen contiguous f32 values occupy camera prefix bytes 0..64; command is recording.
        unsafe {
            self.device.cmd_push_constants(
                command,
                layout,
                stage,
                0,
                std::slice::from_raw_parts(rows.as_ptr().cast::<u8>(), 64),
            );
        }
    }
    pub fn record_identity(
        &self,
        command: vk::CommandBuffer,
        mesh: &ash::ext::mesh_shader::Device,
        work: &IndirectWork,
        proxy: Option<&IndirectWork>,
        world: Option<&IndirectWork>,
        rows: Option<[[f32; 4]; 4]>,
    ) {
        let visibility = self.visibility.as_ref().unwrap();
        // SAFETY: Caller owns/records the command buffer and waits the single frame fence. Targets
        // are live; resource barriers order prior resolve reads, attachment writes and new resolve reads.
        unsafe {
            for (index, image) in self.materials.iter().enumerate() {
                let begin = [vk::ImageMemoryBarrier2::default()
                    .src_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .src_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                    .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                    .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(image.handle)
                    .subresource_range(image.range())];
                self.device.cmd_pipeline_barrier2(
                    command,
                    &vk::DependencyInfo::default().image_memory_barriers(&begin),
                );
                let tint = if index == 0 {
                    [1.0; 4]
                } else {
                    [89.0 / 255.0, 153.0 / 255.0, 1.0, 1.0]
                };
                self.device.cmd_clear_color_image(
                    command,
                    image.handle,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &vk::ClearColorValue { float32: tint },
                    &[image.range()],
                );
                let end = [begin[0]
                    .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                    .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
                self.device.cmd_pipeline_barrier2(
                    command,
                    &vk::DependencyInfo::default().image_memory_barriers(&end),
                );
            }
            let color = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                .src_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(self.identity.handle)
                .subresource_range(self.identity.range());
            let depth = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                .src_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
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
                .image(self.depth.handle)
                .subresource_range(self.depth.range());
            let begin = [color, depth];
            self.device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().image_memory_barriers(&begin),
            );
            let colors = [vk::RenderingAttachmentInfo::default()
                .image_view(self.identity.view)
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue { uint32: [0; 4] },
                })];
            let depth_attachment = vk::RenderingAttachmentInfo::default()
                .image_view(self.depth.view)
                .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                });
            self.device.cmd_begin_rendering(
                command,
                &vk::RenderingInfo::default()
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D::default(),
                        extent: self.extent,
                    })
                    .layer_count(1)
                    .color_attachments(&colors)
                    .depth_attachment(&depth_attachment),
            );
            self.device.cmd_bind_pipeline(
                command,
                vk::PipelineBindPoint::GRAPHICS,
                visibility.handle,
            );
            self.device.cmd_set_viewport(
                command,
                0,
                &[vk::Viewport::default()
                    .width(self.extent.width as f32)
                    .height(self.extent.height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0)],
            );
            self.device.cmd_set_scissor(
                command,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D::default(),
                    extent: self.extent,
                }],
            );
            if self.cooked || self.benchmark {
                self.push_rows(
                    command,
                    visibility.layout,
                    vk::ShaderStageFlags::MESH_EXT,
                    rows.unwrap(),
                );
            }
            work.draw(command, visibility.layout, mesh);
            if let Some(proxy) = proxy {
                let pipeline = self.proxy_visibility.as_ref().unwrap();
                self.device.cmd_bind_pipeline(
                    command,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.handle,
                );
                self.push_rows(
                    command,
                    pipeline.layout,
                    vk::ShaderStageFlags::MESH_EXT,
                    rows.unwrap(),
                );
                proxy.draw(command, pipeline.layout, mesh);
            }
            if let Some(world) = world {
                let pipeline = self.world_visibility.as_ref().unwrap();
                self.device.cmd_bind_pipeline(
                    command,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.handle,
                );
                self.push_rows(
                    command,
                    pipeline.layout,
                    vk::ShaderStageFlags::MESH_EXT,
                    rows.unwrap(),
                );
                world.draw(command, pipeline.layout, mesh);
            }
            self.device.cmd_end_rendering(command);
            let end = [
                color
                    .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                    .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                    .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                depth
                    .src_stage_mask(
                        vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                            | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                    )
                    .src_access_mask(vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                    .old_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .new_layout(vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL),
            ];
            self.device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().image_memory_barriers(&end),
            );
        }
    }
    pub fn record_resolve(
        &self,
        command: vk::CommandBuffer,
        mesh: &ash::ext::mesh_shader::Device,
        rows: Option<[[f32; 4]; 4]>,
        overlay: Option<&[u8]>,
    ) {
        let pipeline = self.resolve.as_ref().unwrap();
        // SAFETY: Called inside compatible swapchain dynamic rendering; descriptor images have been published.
        unsafe {
            if let Some(bytes) = overlay {
                self.device.cmd_push_constants(
                    command,
                    pipeline.layout,
                    vk::ShaderStageFlags::FRAGMENT,
                    if self.cooked_hud { 64 } else { 0 },
                    bytes,
                );
            }
            self.device.cmd_bind_pipeline(
                command,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.handle,
            );
            self.device.cmd_bind_descriptor_sets(
                command,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.layout,
                0,
                &[self.set],
                &[],
            );
            if self.cooked && (!self.authored || self.uv) {
                self.push_rows(
                    command,
                    pipeline.layout,
                    vk::ShaderStageFlags::FRAGMENT,
                    rows.unwrap(),
                );
            }
            if let Some(tail) = self.mip_tail {
                // SAFETY: Four u32 words occupy the tail pipeline's fragment push
                // bytes 64..80; this command is recording with that layout bound.
                self.device.cmd_push_constants(
                    command,
                    pipeline.layout,
                    vk::ShaderStageFlags::FRAGMENT,
                    64,
                    std::slice::from_raw_parts(tail.as_ptr().cast::<u8>(), 16),
                );
            }
            mesh.cmd_draw_mesh_tasks(command, 1, 1, 1);
        }
    }
}
impl Drop for Target {
    fn drop(&mut self) {
        // SAFETY: Owning swapchain retired submissions. Pipelines precede descriptor layout and image destruction.
        drop(self.resolve.take());
        drop(self.proxy_visibility.take());
        drop(self.world_visibility.take());
        drop(self.visibility.take());
        unsafe {
            if self.pool != vk::DescriptorPool::null() {
                self.device.destroy_descriptor_pool(self.pool, None);
            }
            if self.sampler != vk::Sampler::null() {
                self.device.destroy_sampler(self.sampler, None);
            }
            if self.layout != vk::DescriptorSetLayout::null() {
                self.device.destroy_descriptor_set_layout(self.layout, None);
            }
        }
    }
}
