//! One bounded resident KTX2 image; native image tools never enter this runtime.
use crate::{
    Result,
    asset::{Buffer, Transfer},
    visibility::Image,
};
use ash::vk;
use std::{io::Read, path::Path};
use theia_assets::texture::{DEFAULT_BUDGET, Texture as AssetTexture};
pub fn load(path: &Path) -> Result<Vec<u8>> {
    let data = load_bytes(path)?;
    let texture = AssetTexture::decode(&data, None, DEFAULT_BUDGET)?;
    if texture.provenance.is_none() {
        return Err("Texture requires cooked provenance".into());
    }
    Ok(data)
}
pub fn load_mips(path: &Path) -> Result<Vec<u8>> {
    let data = load_bytes(path)?;
    let texture = theia_assets::texture_mips::TextureMipChain::decode(&data, None, DEFAULT_BUDGET)?;
    if texture.provenance.is_none() {
        return Err("Mip texture requires cooked provenance".into());
    }
    Ok(data)
}
fn load_bytes(path: &Path) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    std::fs::File::open(path)?
        .take(DEFAULT_BUDGET as u64 + 1)
        .read_to_end(&mut data)?;
    let hash = theia_assets::content_id(&data)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect::<String>();
    if path.file_stem().and_then(|v| v.to_str()) != Some(&hash) {
        return Err("Texture hash filename mismatch".into());
    }
    Ok(data)
}
pub struct BundlePaths {
    pub paths: [std::path::PathBuf; 4],
    pub indexed: bool,
}
pub fn load_bundle(path: &Path) -> Result<BundlePaths> {
    use theia_assets::bundle::{BYTES, Bundle};
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let hex = |id: [u8; 32]| id.iter().map(|v| format!("{v:02x}")).collect::<String>();
    if path.file_stem().and_then(|v| v.to_str()) != Some(&hex(theia_assets::content_id(&bytes))) {
        return Err("Bundle hash filename mismatch".into());
    }
    let bundle = Bundle::decode(&bytes, None)?;
    let indexed = bundle.profile == theia_assets::bundle::BundleProfile::TextureTable;
    let extensions = [
        "theia-mesh",
        "theia-material",
        "theia-uv",
        if indexed { "theia-textures" } else { "ktx2" },
    ];
    let paths = std::array::from_fn(|i| {
        path.parent().unwrap().join(format!(
            "{}.{}",
            hex(bundle.dependencies[i].content_id),
            extensions[i]
        ))
    });
    let mut children = Vec::new();
    for (path, dep) in paths.iter().zip(bundle.dependencies) {
        let mut data = Vec::new();
        std::fs::File::open(path)?
            .take(dep.bytes + 1)
            .read_to_end(&mut data)?;
        children.push(data);
    }
    let slices = std::array::from_fn(|i| children[i].as_slice());
    let (mesh, uvs) = if indexed {
        let checked = bundle.validate_table_roots(slices)?;
        load_set(&paths[3], &checked.materials)?;
        (checked.mesh, checked.uvs)
    } else {
        let checked = bundle.validate(slices)?;
        (checked.mesh, checked.uvs)
    };
    if mesh.meshlets.len() > 32 || uvs.texcoords.iter().flatten().any(|v| v.abs() > 64.) {
        return Err("Bundle exceeds renderer meshlet/UV qualification profile".into());
    }
    println!(
        "Complete immutable texture bundle admitted: four roots and all texture dependencies, hashes/lengths/bindings verified before Vulkan initialization; indexed={indexed}"
    );
    Ok(BundlePaths { paths, indexed })
}

pub struct ResidentTexture {
    pub image: Image,
    pub sampler: vk::Sampler,
    device: ash::Device,
}
pub struct TextureSet {
    pub table: Vec<u8>,
    pub images: Vec<Vec<u8>>,
}
pub fn load_set(
    path: &Path,
    materials: &theia_assets::material::MaterialPage,
) -> Result<TextureSet> {
    use theia_assets::texture_table::{DEFAULT_BUDGET as TABLE_BUDGET, TextureTable};
    let mut table = Vec::new();
    std::fs::File::open(path)?
        .take(TABLE_BUDGET as u64 + 1)
        .read_to_end(&mut table)?;
    let hex = |id: [u8; 32]| id.iter().map(|v| format!("{v:02x}")).collect::<String>();
    if path.file_stem().and_then(|v| v.to_str()) != Some(&hex(theia_assets::content_id(&table))) {
        return Err("Texture table hash filename mismatch".into());
    }
    let decoded = TextureTable::decode(&table, None, materials, TABLE_BUDGET)?;
    let mut images = Vec::new();
    for dep in &decoded.textures {
        images.push(load(
            &path.parent().unwrap().join(hex(dep.content_id) + ".ktx2"),
        )?);
    }
    decoded.validate_textures(&images.iter().map(Vec::as_slice).collect::<Vec<_>>())?;
    Ok(TextureSet { table, images })
}
pub fn admit_table(
    path: &Path,
    mesh_path: &Path,
    material_path: &Path,
    uv_path: &Path,
) -> Result<()> {
    let bytes = crate::asset::load(mesh_path)?;
    let mesh = theia_assets::Mesh::decode(&bytes, None, theia_assets::DEFAULT_BUDGET)?;
    let bytes = crate::asset::load_material(material_path, &mesh)?;
    let materials = theia_assets::material::MaterialPage::decode(
        &bytes,
        None,
        &mesh,
        theia_assets::material::DEFAULT_BUDGET,
    )?;
    crate::asset::load_uv(uv_path, &mesh)?;
    let set = load_set(path, &materials)?;
    println!(
        "Complete material texture set admitted: {} immutable textures, all lengths/hashes/geometry/material bindings verified before Vulkan initialization",
        set.images.len()
    );
    Ok(())
}
pub struct Upload {
    // Dedicated work retires before staging/image drops on every error path.
    transfer: Transfer,
    staging: Buffer,
    image: Option<Image>,
    extent: vk::Extent2D,
    texel_bytes: usize,
    mip_levels: u32,
    view_level: Option<u32>,
}
impl Upload {
    /// Submit one owned transfer without waiting for its dedicated fence.
    pub fn begin(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        texture: &AssetTexture<'_>,
        family: u32,
        queue: vk::Queue,
        budget: u64,
    ) -> Result<Self> {
        Self::begin_levels(
            device,
            memory,
            &[theia_assets::texture_mips::MipLevel {
                width: texture.width,
                height: texture.height,
                texels: texture.texels,
            }],
            Some(0),
            family,
            queue,
            budget,
        )
    }
    fn begin_levels(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        levels: &[theia_assets::texture_mips::MipLevel<'_>],
        view_level: Option<u32>,
        family: u32,
        queue: vk::Queue,
        budget: u64,
    ) -> Result<Self> {
        if levels.is_empty()
            || levels.len() > 11
            || view_level.is_some_and(|level| level as usize >= levels.len())
        {
            return Err("Invalid texture upload mip/view count".into());
        }
        let extent = vk::Extent2D {
            width: levels[0].width,
            height: levels[0].height,
        };
        if !(1..=theia_assets::texture::MAX_DIMENSION).contains(&extent.width)
            || !(1..=theia_assets::texture::MAX_DIMENSION).contains(&extent.height)
            || levels.len() > (u32::BITS - extent.width.max(extent.height).leading_zeros()) as usize
        {
            return Err("Texture upload dimensions/mip count exceed the bounded profile".into());
        }
        let mut texel_bytes = 0usize;
        let mut regions = Vec::with_capacity(levels.len());
        for (index, level) in levels.iter().enumerate() {
            let width = (extent.width >> index).max(1);
            let height = (extent.height >> index).max(1);
            if (level.width, level.height) != (width, height)
                || level.texels.len() as u64 != u64::from(width) * u64::from(height) * 4
            {
                return Err("Texture upload mip dimensions/texel bytes mismatch".into());
            }
            regions.push(
                vk::BufferImageCopy::default()
                    .buffer_offset(texel_bytes as u64)
                    .image_subresource(
                        vk::ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .mip_level(index as u32)
                            .layer_count(1),
                    )
                    .image_extent(vk::Extent3D {
                        width,
                        height,
                        depth: 1,
                    }),
            );
            texel_bytes = texel_bytes
                .checked_add(level.texels.len())
                .filter(|&v| v <= DEFAULT_BUDGET)
                .ok_or("Texture upload staging byte budget exceeded")?;
        }
        let image = Image::bounded_mips(
            device,
            memory,
            extent,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            budget,
            (levels.len() as u32, view_level),
        )?;
        let staging = Buffer::allocate(
            device,
            memory,
            texel_bytes as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            DEFAULT_BUDGET as u64,
        )?;
        let mut pending = Self {
            transfer: Transfer {
                device: device.clone(),
                pool: vk::CommandPool::null(),
                fence: vk::Fence::null(),
                submitted: false,
            },
            staging,
            image: Some(image),
            extent,
            texel_bytes,
            mip_levels: levels.len() as u32,
            view_level,
        };
        let image = pending.image.as_ref().unwrap();
        // SAFETY: Mapping/copy sizes are bounded. Pending owns submitted commands,
        // staging and image until its dedicated fence completes, including failures.
        unsafe {
            let pointer = device.map_memory(
                pending.staging.memory,
                0,
                pending.staging.bytes,
                vk::MemoryMapFlags::empty(),
            )?;
            let mut offset = 0;
            for level in levels {
                std::ptr::copy_nonoverlapping(
                    level.texels.as_ptr(),
                    pointer.cast::<u8>().add(offset),
                    level.texels.len(),
                );
                offset += level.texels.len();
            }
            device.unmap_memory(pending.staging.memory);
            pending.transfer.pool = device.create_command_pool(
                &vk::CommandPoolCreateInfo::default().queue_family_index(family),
                None,
            )?;
            pending.transfer.fence = device.create_fence(&vk::FenceCreateInfo::default(), None)?;
            let command = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(pending.transfer.pool)
                    .command_buffer_count(1),
            )?[0];
            device.begin_command_buffer(
                command,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            let begin = [vk::ImageMemoryBarrier2::default()
                .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .image(image.handle)
                .subresource_range(image.range())
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)];
            device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().image_memory_barriers(&begin),
            );
            device.cmd_copy_buffer_to_image(
                command,
                pending.staging.handle,
                image.handle,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &regions,
            );
            let end = [vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(image.handle)
                .subresource_range(image.range())
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)];
            device.cmd_pipeline_barrier2(
                command,
                &vk::DependencyInfo::default().image_memory_barriers(&end),
            );
            device.end_command_buffer(command)?;
            let commands = [vk::CommandBufferSubmitInfo::default().command_buffer(command)];
            device.queue_submit2(
                queue,
                &[vk::SubmitInfo2::default().command_buffer_infos(&commands)],
                pending.transfer.fence,
            )?;
            pending.transfer.submitted = true;
        }
        println!(
            "Texture transfer submitted: {}x{}, {} physical image bytes, {} staging allocation bytes; completion not yet waited",
            extent.width,
            extent.height,
            pending.resident_bytes(),
            pending.staging_bytes()
        );
        Ok(pending)
    }
    pub fn ready(&self) -> Result<bool> {
        // SAFETY: Upload retains the live device and dedicated submitted fence.
        Ok(unsafe { self.transfer.device.get_fence_status(self.transfer.fence)? })
    }
    pub fn resident_bytes(&self) -> u64 {
        self.image.as_ref().unwrap().allocated_bytes
    }
    pub fn staging_bytes(&self) -> u64 {
        self.staging.allocated_bytes
    }
    pub fn finish(mut self) -> Result<ResidentTexture> {
        // SAFETY: Fence covers this upload and all resources remain owned through retirement.
        unsafe {
            self.transfer
                .device
                .wait_for_fences(&[self.transfer.fence], true, u64::MAX)?;
        }
        self.transfer.submitted = false;
        let device = &self.transfer.device;
        let mut resident = ResidentTexture {
            image: self.image.take().unwrap(),
            sampler: vk::Sampler::null(),
            device: device.clone(),
        };
        // SAFETY: Device is live; resident owns the created sampler immediately.
        resident.sampler = unsafe {
            device.create_sampler(
                &vk::SamplerCreateInfo::default()
                    .mag_filter(vk::Filter::NEAREST)
                    .min_filter(vk::Filter::NEAREST)
                    .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                    .address_mode_u(vk::SamplerAddressMode::REPEAT)
                    .address_mode_v(vk::SamplerAddressMode::REPEAT)
                    .address_mode_w(vk::SamplerAddressMode::REPEAT)
                    .min_lod(0.)
                    .max_lod(if self.view_level.is_none() {
                        (self.mip_levels - 1) as f32
                    } else {
                        0.
                    }),
                None,
            )?
        };
        println!(
            "Resident texture upload: {}x{} RGBA8_SRGB, {} texel bytes, {} physical image bytes, {} staging bytes; dedicated fence retired; nearest repeat mip 0",
            self.extent.width,
            self.extent.height,
            self.texel_bytes,
            resident.image.allocated_bytes,
            self.staging.allocated_bytes
        );
        if let Some(level) = self.view_level {
            if self.mip_levels > 1 {
                println!(
                    "Resident mip chain ready: {} uploaded levels; fixed view level {level}; shader samples view-relative level zero",
                    self.mip_levels
                );
            }
        } else {
            println!(
                "Resident mip chain ready: {} uploaded levels; complete view; GPU footprint selects mip",
                self.mip_levels
            );
        }
        Ok(resident)
    }
}
impl ResidentTexture {
    pub fn upload_auto_mips(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        texture: &theia_assets::texture_mips::TextureMipChain<'_>,
        family: u32,
        queue: vk::Queue,
        budget: u64,
    ) -> Result<Self> {
        Upload::begin_levels(device, memory, &texture.levels, None, family, queue, budget)?.finish()
    }
    pub fn upload_mips(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        texture: &theia_assets::texture_mips::TextureMipChain<'_>,
        view_level: u32,
        family: u32,
        queue: vk::Queue,
        budget: u64,
    ) -> Result<Self> {
        Upload::begin_levels(
            device,
            memory,
            &texture.levels,
            Some(view_level),
            family,
            queue,
            budget,
        )?
        .finish()
    }
    pub fn upload(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        texture: &AssetTexture<'_>,
        family: u32,
        queue: vk::Queue,
        budget: u64,
    ) -> Result<Self> {
        Upload::begin(device, memory, texture, family, queue, budget)?.finish()
    }
    pub fn descriptor(&self) -> vk::DescriptorImageInfo {
        vk::DescriptorImageInfo::default()
            .image_view(self.image.view)
            .sampler(self.sampler)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
    }
}
impl Drop for ResidentTexture {
    fn drop(&mut self) {
        // SAFETY: Renderer retires frame/descriptor users before this owner drops.
        unsafe {
            if self.sampler != vk::Sampler::null() {
                self.device.destroy_sampler(self.sampler, None);
            }
        }
    }
}

/// CPU oracle: explicit nearest repeat at level zero, sRGB RGB decode before factors.
pub fn reference_sample(texture: &AssetTexture<'_>, uv: [f32; 2]) -> [f32; 3] {
    reference_texels(texture.width, texture.height, texture.texels, uv)
}
pub fn reference_mip(
    texture: &theia_assets::texture_mips::TextureMipChain<'_>,
    level: u32,
    uv: [f32; 2],
) -> Result<[f32; 3]> {
    let mip = texture
        .levels
        .get(level as usize)
        .ok_or("CPU mip view is outside the chain")?;
    Ok(reference_texels(mip.width, mip.height, mip.texels, uv))
}
fn reference_texels(width: u32, height: u32, texels: &[u8], uv: [f32; 2]) -> [f32; 3] {
    let coordinates = std::array::from_fn::<_, 2, _>(|axis| uv[axis] - uv[axis].floor());
    let x = ((coordinates[0] * width as f32).floor() as u32).min(width - 1);
    let y = ((coordinates[1] * height as f32).floor() as u32).min(height - 1);
    let offset = ((y * width + x) * 4) as usize;
    std::array::from_fn(|axis| {
        let v = texels[offset + axis] as f32 / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mip_reference_selects_authored_level_and_rejects_missing_levels() {
        let chain = theia_assets::texture_mips::TextureMipChain::decode(
            include_bytes!("../../../fixtures/textures/opaque_4x4_mips.ktx2"),
            None,
            DEFAULT_BUDGET,
        )
        .unwrap();
        for (level, red) in [0.00303527, 0.08021982, 0.3049873].into_iter().enumerate() {
            let sampled = reference_mip(&chain, level as u32, [-0.25, 1.75]).unwrap();
            assert!((sampled[0] - red).abs() < 1e-6);
            assert!((sampled[1] - 0.03189603).abs() < 1e-6);
        }
        assert!(reference_mip(&chain, 3, [0., 0.]).is_err());
        assert!(reference_mip(&chain, u32::MAX, [0., 0.]).is_err());
    }
    #[test]
    fn nearest_repeat_preserves_top_left_and_decodes_srgb_before_factors() {
        let tex = AssetTexture::decode(
            include_bytes!("../../../fixtures/textures/opaque_2x2.ktx2"),
            None,
            DEFAULT_BUDGET,
        )
        .unwrap();
        let first = reference_sample(&tex, [0.25, 0.25]);
        assert!((first[0] - 0.8713671).abs() < 1e-6);
        assert!((first[1] - 0.014443844).abs() < 1e-6);
        assert!((first[2] - 0.029556835).abs() < 1e-6);
        assert_eq!(
            reference_sample(&tex, [-0.25, 1.75]),
            reference_sample(&tex, [0.75, 0.75])
        );
        assert_eq!(
            reference_sample(&tex, [1., -1.]),
            reference_sample(&tex, [0., 0.])
        );
        assert_ne!(
            reference_sample(&tex, [0.25, 0.25]),
            reference_sample(&tex, [0.25, 0.75])
        );
    }
}
