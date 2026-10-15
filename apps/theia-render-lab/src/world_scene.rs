//! Bounded four-chunk resident world fixture. No camera demand or live replacement.
use crate::{
    Result,
    bundle_streaming::{Gpu, Input},
    indirect::IndirectWork,
    world_upload::{self, PresentationSet, Set},
};
use ash::vk;
use std::{io::Read, path::Path};
use theia_assets::{
    bundle_residency::WorldResidency,
    world_coverage::{ProtectedWorld, Region},
    world_demand::ChunkBounds,
};
enum CpuOwners {
    Fixed { _world: WorldResidency },
    Covered { _world: ProtectedWorld },
}
pub const CHUNKS: usize = 4;
pub const MAX_IMAGES: usize = CHUNKS * theia_assets::texture_table::MAX_TEXTURES;

pub(super) struct Scene {
    pub work: [IndirectWork; CHUNKS],
    pub resources: [PresentationSet; CHUNKS],
    // Encoded sources outlive every GPU owner.
    _protected: Vec<Set>,
    _cpu: CpuOwners,
}
impl Scene {
    pub fn load(
        path: &Path,
        gpu: Gpu<'_>,
        shaders: &Path,
        budgets: (usize, u64, u64),
        covered: bool,
    ) -> Result<Self> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 1024 * 1024 {
            return Err("World fixture manifest exceeds 1 MiB".into());
        }
        let scene: serde_json::Value = serde_json::from_slice(&bytes)?;
        if covered {
            return Self::load_covered(&scene, gpu, shaders, budgets);
        }
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
            _protected: Vec::new(),
            _cpu: CpuOwners::Fixed { _world: cpu },
        })
    }
    fn load_covered(
        scene: &serde_json::Value,
        gpu: Gpu<'_>,
        shaders: &Path,
        budgets: (usize, u64, u64),
    ) -> Result<Self> {
        let mut inputs = Vec::new();
        for role in ["fallbacks", "quadrants"] {
            let entries = scene[role]
                .as_array()
                .ok_or("Missing protected world role")?;
            if scene["passed"] != true || entries.len() != CHUNKS {
                return Err("Protected world needs four checked coarse/fine pairs".into());
            }
            inputs.push(
                entries
                    .iter()
                    .map(|entry| {
                        Input::load(Path::new(
                            entry["cook"]["bundle"]["path"]
                                .as_str()
                                .ok_or("Missing protected world bundle path")?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            );
        }
        let images: u32 = (0..CHUNKS)
            .map(|i| inputs[0][i].leaves.len().max(inputs[1][i].leaves.len()) as u32)
            .sum();
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
            return Err("Protected scene exceeds selected descriptor/push limits".into());
        }
        let regions = inputs[1]
            .iter()
            .enumerate()
            .map(|(i, input)| {
                let mesh = theia_assets::Mesh::decode(
                    &input.roots[0],
                    None,
                    theia_assets::DEFAULT_BUDGET,
                )?;
                Ok(Region {
                    fine: ChunkBounds {
                        id: i as u64,
                        min: std::array::from_fn(|a| {
                            mesh.positions
                                .iter()
                                .map(|p| p[a])
                                .fold(f32::INFINITY, f32::min)
                        }),
                        max: std::array::from_fn(|a| {
                            mesh.positions
                                .iter()
                                .map(|p| p[a])
                                .fold(f32::NEG_INFINITY, f32::max)
                        }),
                    },
                    fallback: i as u64 + CHUNKS as u64,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut cpu = WorldResidency::new(budgets.0, CHUNKS * 2)?;
        let mut staged = Vec::new();
        for (i, input) in inputs[0].iter().enumerate() {
            staged.push((
                i as u64 + CHUNKS as u64,
                cpu.stage(
                    &input.manifest,
                    theia_assets::content_id(&input.manifest),
                    std::array::from_fn(|i| input.roots[i].as_slice()),
                    &input.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                )?,
            ));
        }
        let (mut remaining, mut image_remaining) = (budgets.1, budgets.2);
        let mut sets = Vec::new();
        for input in &inputs[0] {
            let set = world_upload::upload(input, &gpu, remaining, image_remaining)?;
            remaining = remaining
                .checked_sub(set.bytes())
                .ok_or("Protected physical accounting underflow")?;
            image_remaining = image_remaining
                .checked_sub(set.image_bytes())
                .ok_or("Protected image accounting underflow")?;
            sets.push(set);
        }
        cpu.publish(staged, &[])?;
        let mut cpu = ProtectedWorld::new(cpu, &regions)?;
        let mut protected = Vec::new();
        let (mut deferred_cpu, mut deferred_gpu) = (Vec::new(), Vec::new());
        for (i, input) in inputs[1].iter().enumerate() {
            let staged = match cpu.stage(
                &input.manifest,
                theia_assets::content_id(&input.manifest),
                std::array::from_fn(|i| input.roots[i].as_slice()),
                &input.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            ) {
                Ok(value) => value,
                Err(error) if error.0 == "live bundle byte budget exhausted" => {
                    deferred_cpu.push(i);
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            let set = match world_upload::upload(input, &gpu, remaining, image_remaining) {
                Ok(value) => value,
                Err(error) if world_upload::budget_rejection(error.as_ref()) => {
                    drop(staged);
                    deferred_gpu.push(i);
                    continue;
                }
                Err(error) => return Err(error),
            };
            // Startup has no graphics users. Complete transfers retired before
            // mirrored CPU publication; protected coarse GPU owners remain live.
            cpu.publish_fine(vec![(i as u64, staged)], &[])?;
            remaining = remaining
                .checked_sub(set.bytes())
                .ok_or("Protected fine accounting underflow")?;
            image_remaining = image_remaining
                .checked_sub(set.image_bytes())
                .ok_or("Protected fine image accounting underflow")?;
            protected.push(std::mem::replace(&mut sets[i], set));
        }
        let (mut work, mut resources) = (Vec::new(), Vec::new());
        for set in sets {
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
        let snapshot = cpu.snapshot();
        println!(
            "Protected four-chunk world ready: {}",
            serde_json::json!({
                "regions":CHUNKS,"owned_chunks":snapshot.world().chunk_count(),
                "selected":snapshot.selections().iter().map(|s|s.chunk).collect::<Vec<_>>(),
                "protected_epochs":(4..8).map(|id|snapshot.world().chunk(id).unwrap().epoch()).collect::<Vec<_>>(),
                "cpu_epoch":snapshot.world().epoch(),"cpu_unique_bytes":cpu.live_bytes(),
                "physical_bytes":budgets.1-remaining,"image_bytes":budgets.2-image_remaining,
                "root_buffers":snapshot.world().chunk_count()*4,
                "images":resources.iter().map(|s|s.images.len()).sum::<usize>()+protected.iter().map(|s|s.images.len()).sum::<usize>(),
                "deferred_cpu":deferred_cpu,"deferred_gpu":deferred_gpu,
                "live_publication":false,"camera_demand":false,"authority":false,
            })
        );
        Ok(Self {
            work: work.try_into().map_err(|_| "Wrong protected draw count")?,
            resources: resources
                .try_into()
                .map_err(|_| "Wrong protected resource count")?,
            _protected: protected,
            _cpu: CpuOwners::Covered { _world: cpu },
        })
    }
    pub fn physical_bytes(&self) -> u64 {
        self._protected.iter().map(Set::bytes).sum::<u64>()
            + self
                .work
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
