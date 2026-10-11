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
    let texture = AssetTexture::decode(&data, None, DEFAULT_BUDGET)?;
    if texture.provenance.is_none() {
        return Err("Texture requires cooked provenance".into());
    }
    Ok(data)
}
pub fn load_bundle(path: &Path) -> Result<[std::path::PathBuf; 4]> {
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
    let extensions = ["theia-mesh", "theia-material", "theia-uv", "ktx2"];
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
    let checked = bundle.validate(std::array::from_fn(|i| children[i].as_slice()))?;
    if checked.mesh.meshlets.len() > 32
        || checked
            .uvs
            .texcoords
            .iter()
            .flatten()
            .any(|v| v.abs() > 64.)
    {
        return Err("Bundle exceeds renderer meshlet/UV qualification profile".into());
    }
    println!(
        "Complete immutable texture bundle admitted: four dependency hashes/lengths, geometry bindings and opaque texture verified before Vulkan initialization"
    );
    Ok(paths)
}
pub struct ResidentTexture {
    pub image: Image,
    pub sampler: vk::Sampler,
    device: ash::Device,
}
struct Pending {
    // Dedicated work retires before staging/image drops on every error path.
    transfer: Transfer,
    staging: Buffer,
    image: Option<Image>,
}
impl ResidentTexture {
    pub fn upload(
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        texture: &AssetTexture<'_>,
        family: u32,
        queue: vk::Queue,
        budget: u64,
    ) -> Result<Self> {
        let extent = vk::Extent2D {
            width: texture.width,
            height: texture.height,
        };
        let image = Image::bounded(
            device,
            memory,
            extent,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageAspectFlags::COLOR,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            budget,
        )?;
        let staging = Buffer::allocate(
            device,
            memory,
            texture.texels.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            DEFAULT_BUDGET as u64,
        )?;
        let mut pending = Pending {
            transfer: Transfer {
                device: device.clone(),
                pool: vk::CommandPool::null(),
                fence: vk::Fence::null(),
                submitted: false,
            },
            staging,
            image: Some(image),
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
            std::ptr::copy_nonoverlapping(
                texture.texels.as_ptr(),
                pointer.cast::<u8>(),
                texture.texels.len(),
            );
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
                &[vk::BufferImageCopy::default()
                    .image_subresource(
                        vk::ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1),
                    )
                    .image_extent(vk::Extent3D {
                        width: texture.width,
                        height: texture.height,
                        depth: 1,
                    })],
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
            device.wait_for_fences(&[pending.transfer.fence], true, u64::MAX)?;
            pending.transfer.submitted = false;
        }
        let mut resident = Self {
            image: pending.image.take().unwrap(),
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
                    .max_lod(0.),
                None,
            )?
        };
        println!(
            "Resident texture upload: {}x{} RGBA8_SRGB, {} texel bytes, {} physical image bytes, {} staging bytes; dedicated fence retired; nearest repeat mip 0",
            texture.width,
            texture.height,
            texture.texels.len(),
            resident.image.allocated_bytes,
            pending.staging.allocated_bytes
        );
        Ok(resident)
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
    let coordinates = std::array::from_fn::<_, 2, _>(|axis| uv[axis] - uv[axis].floor());
    let x = ((coordinates[0] * texture.width as f32).floor() as u32).min(texture.width - 1);
    let y = ((coordinates[1] * texture.height as f32).floor() as u32).min(texture.height - 1);
    let offset = ((y * texture.width + x) * 4) as usize;
    std::array::from_fn(|axis| {
        let v = texture.texels[offset + axis] as f32 / 255.;
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
