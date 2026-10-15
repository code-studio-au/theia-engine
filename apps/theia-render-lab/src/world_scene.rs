//! Bounded four-chunk resident world fixture. No camera demand or live replacement.
use crate::{
    Result,
    bundle_streaming::{Gpu, Input},
    indirect::IndirectWork,
    world_upload::{self, PresentationSet},
};
use ash::vk;
use std::{io::Read, path::Path};
use theia_assets::bundle_residency::WorldResidency;
pub const CHUNKS: usize = 4;
pub const MAX_IMAGES: usize = CHUNKS * theia_assets::texture_table::MAX_TEXTURES;

pub(super) struct Scene {
    pub work: [IndirectWork; CHUNKS],
    pub resources: [PresentationSet; CHUNKS],
    // Encoded sources outlive every GPU owner.
    _cpu: WorldResidency,
}
impl Scene {
    pub fn load(
        path: &Path,
        gpu: Gpu<'_>,
        shaders: &Path,
        budgets: (usize, u64, u64),
    ) -> Result<Self> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 1024 * 1024 {
            return Err("World fixture manifest exceeds 1 MiB".into());
        }
        let scene: serde_json::Value = serde_json::from_slice(&bytes)?;
        let entries = scene["quadrants"]
            .as_array()
            .ok_or("Missing world fixture chunks")?;
        if scene["passed"] != true || entries.len() != CHUNKS {
            return Err("World fixture needs four checked chunks".into());
        }
        let inputs = entries
            .iter()
            .map(|e| {
                Input::load(Path::new(
                    e["cook"]["bundle"]["path"]
                        .as_str()
                        .ok_or("Missing world bundle path")?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let images: u32 = inputs.iter().map(|i| i.leaves.len() as u32).sum();
        let limits = gpu.limits;
        if limits.max_per_stage_descriptor_storage_buffers < 16
            || limits.max_descriptor_set_storage_buffers < 16
            || limits.max_per_stage_descriptor_sampled_images < images + 4
            || limits.max_descriptor_set_sampled_images < images + 4
            || limits.max_per_stage_descriptor_samplers < 2
            || limits.max_descriptor_set_samplers < 2
            || limits.max_per_stage_resources < images + 22
            || limits.max_push_constants_size < 80
        {
            return Err("Four-chunk scene exceeds descriptor/push limits".into());
        }
        let mut cpu = WorldResidency::new(budgets.0, CHUNKS)?;
        let mut staged = Vec::new();
        for (id, input) in inputs.iter().enumerate() {
            staged.push((
                id as u64,
                cpu.stage(
                    &input.manifest,
                    theia_assets::content_id(&input.manifest),
                    std::array::from_fn(|i| input.roots[i].as_slice()),
                    &input.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                )?,
            ));
        }
        let mut remaining = budgets.1;
        let mut image_remaining = budgets.2;
        let mut work = Vec::new();
        let mut resources = Vec::new();
        for input in &inputs {
            let set = world_upload::upload(input, &gpu, remaining, image_remaining)?;
            remaining = remaining
                .checked_sub(set.bytes())
                .ok_or("Scene physical accounting underflow")?;
            image_remaining = image_remaining
                .checked_sub(set.image_bytes())
                .ok_or("Scene image accounting underflow")?;
            let [geometry, materials, uvs, table] = set.buffers;
            work.push(IndirectWork::resident(
                gpu.device, gpu.memory, shaders, geometry,
            )?);
            resources.push(PresentationSet {
                materials,
                uvs,
                table,
                images: set.images,
            });
        }
        let snapshot = cpu.publish(staged, &[])?;
        println!(
            "Resident four-chunk world ready: {}",
            serde_json::json!({"chunks":CHUNKS,
            "root_buffers":16,"images":images,"physical_bytes":budgets.1-remaining,
            "image_bytes":budgets.2-image_remaining,"cpu_unique_bytes":cpu.live_bytes(),
            "cpu_epoch":snapshot.epoch(),"live_replacement":false,"authority":false})
        );
        Ok(Self {
            work: work.try_into().map_err(|_| "Wrong scene work count")?,
            resources: resources
                .try_into()
                .map_err(|_| "Wrong scene resource count")?,
            _cpu: cpu,
        })
    }
    pub fn physical_bytes(&self) -> u64 {
        self.work
            .iter()
            .map(IndirectWork::resident_bytes)
            .sum::<u64>()
            + self
                .resources
                .iter()
                .map(PresentationSet::bytes)
                .sum::<u64>()
    }
    pub fn descriptors(&self) -> crate::visibility::SceneBuffers {
        let descriptor = |b: &crate::asset::Buffer| {
            vk::DescriptorBufferInfo::default()
                .buffer(b.handle)
                .offset(b.offset)
                .range(b.bytes)
        };
        let mut images = [vk::DescriptorImageInfo::default(); MAX_IMAGES];
        let mut offsets = [0; CHUNKS];
        let mut count = 0;
        for (slot, set) in self.resources.iter().enumerate() {
            offsets[slot] = count;
            for image in &set.images {
                images[count as usize] = image.descriptor();
                count += 1;
            }
        }
        crate::visibility::SceneBuffers {
            geometry: std::array::from_fn(|i| self.work[i].geometry_descriptor()),
            materials: std::array::from_fn(|i| descriptor(&self.resources[i].materials)),
            uvs: std::array::from_fn(|i| descriptor(&self.resources[i].uvs)),
            tables: std::array::from_fn(|i| descriptor(&self.resources[i].table)),
            images,
            count,
            offsets,
        }
    }
}
