//! Bounded four-region world fixture with isolated fixed, covered and camera-driven profiles.
use crate::{
    Result,
    bundle_streaming::{Gpu, Input},
    indirect::IndirectWork,
    world_upload::{self, PresentationSet, Set},
};
use ash::vk;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use theia_assets::{
    bundle_residency::WorldResidency,
    world_coverage::{ProtectedWorld, Region},
    world_demand::ChunkBounds,
};
enum CpuOwners {
    Fixed { _world: WorldResidency },
    Covered { _world: ProtectedWorld },
}
struct Retirement {
    history: Vec<serde_json::Value>, // Initial state + four retirements + at most one fine reload.
    report: Option<PathBuf>,
    completed: [bool; CHUNKS],
}
pub(super) struct FinePublication {
    region: usize,
    staged: theia_assets::bundle_residency::StagedBundle,
}
pub(super) struct Eviction {
    slot: usize,
    // Graphics retired before these old GPU owners release; encoded owners last.
    _retired: Set,
    _reader: theia_assets::world_coverage::Snapshot,
}
pub const CHUNKS: usize = 4;
pub const MAX_IMAGES: usize = CHUNKS * theia_assets::texture_table::MAX_TEXTURES;

pub(super) struct Scene {
    pub work: [IndirectWork; CHUNKS],
    pub resources: [PresentationSet; CHUNKS],
    // Encoded sources outlive every GPU owner.
    _protected: [Option<Set>; CHUNKS],
    retirement: Option<Retirement>,
    reload: Option<crate::world_refinement::Reload>,
    demand: Option<crate::world_camera::Stream>,
    _cpu: CpuOwners,
    fine_sources: Option<[PathBuf; CHUNKS]>,
}
impl Scene {
    pub fn load(
        path: &Path,
        gpu: Gpu<'_>,
        shaders: &Path,
        budgets: (usize, u64, u64),
        covered: bool,
        camera_mode: bool,
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
            return Self::load_covered(&scene, gpu, shaders, budgets, camera_mode);
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
            _protected: std::array::from_fn(|_| None),
            retirement: None,
            reload: None,
            _cpu: CpuOwners::Fixed { _world: cpu },
            fine_sources: None,
            demand: None,
        })
    }
    fn load_covered(
        scene: &serde_json::Value,
        gpu: Gpu<'_>,
        shaders: &Path,
        budgets: (usize, u64, u64),
        camera_mode: bool,
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
        let mut protected: [Option<Set>; CHUNKS] = std::array::from_fn(|_| None);
        let (mut deferred_cpu, mut deferred_gpu) = (Vec::new(), Vec::new());
        for (i, input) in inputs[1].iter().enumerate() {
            if camera_mode {
                break;
            }
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
            protected[i] = Some(std::mem::replace(&mut sets[i], set));
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
                "images":resources.iter().map(|s|s.images.len()).sum::<usize>()+protected.iter().flatten().map(|s|s.images.len()).sum::<usize>(),
                "deferred_cpu":deferred_cpu,"deferred_gpu":deferred_gpu,
                "startup_phase":true,"camera_demand":camera_mode,"authority":false,
            })
        );
        Ok(Self {
            work: work.try_into().map_err(|_| "Wrong protected draw count")?,
            resources: resources
                .try_into()
                .map_err(|_| "Wrong protected resource count")?,
            _protected: protected,
            retirement: None,
            reload: None,
            _cpu: CpuOwners::Covered { _world: cpu },
            fine_sources: Some(
                scene["quadrants"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|entry| {
                        entry["cook"]["bundle"]["path"]
                            .as_str()
                            .map(PathBuf::from)
                            .ok_or("Missing fine source path")
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?
                    .try_into()
                    .map_err(|_| "Wrong fine source count")?,
            ),
            demand: None,
        })
    }
    pub fn enable_retirement(&mut self, report: Option<PathBuf>) -> Result<()> {
        if !matches!(self._cpu, CpuOwners::Covered { .. }) {
            return Err("Fine retirement needs protected startup".into());
        }
        self.retirement = Some(Retirement {
            history: vec![self.covered_stats(0)?],
            report,
            completed: std::array::from_fn(|i| self._protected[i].is_none()),
        });
        self.retirement_report(0, false)?;
        println!(
            "Protected fine retirement enabled: slots 0..3 after presented frames 20/40/60/80; retained coarse buffers reused"
        );
        Ok(())
    }
    pub fn next_retirement(&self, frame: u64) -> Option<usize> {
        self.retirement.as_ref()?;
        self._protected.iter().enumerate().find_map(|(slot, set)| {
            (!self.retirement.as_ref().unwrap().completed[slot]
                && set.is_some()
                && frame >= (slot as u64 + 1) * 20)
                .then_some(slot)
        })
    }
    /// Sole frame fence and previous graphics/presentation generation must retire
    /// before replacing descriptors or returning old GPU owners for destruction.
    pub fn prepare_retirement(&mut self, slot: usize) -> Result<Eviction> {
        let CpuOwners::Covered { _world: cpu } = &self._cpu else {
            return Err("Retirement requires protected CPU owners".into());
        };
        let reader = cpu.snapshot();
        let set = self
            ._protected
            .get_mut(slot)
            .and_then(Option::take)
            .ok_or("No retained coarse set for selected fine region")?;
        let [geometry, materials, uvs, table] = set.buffers;
        let old_geometry = self.work[slot].replace_asset(geometry);
        let old = std::mem::replace(
            &mut self.resources[slot],
            PresentationSet {
                materials,
                uvs,
                table,
                images: set.images,
            },
        );
        Ok(Eviction {
            slot,
            _retired: Set {
                buffers: [old_geometry, old.materials, old.uvs, old.table],
                images: old.images,
            },
            _reader: reader,
        })
    }
    /// New complete graphics generation is ready; publish matching CPU selection.
    pub fn retirement_published(&mut self, eviction: Eviction, frame: u64) -> Result<()> {
        let CpuOwners::Covered { _world: cpu } = &mut self._cpu else {
            return Err("Missing protected publication owner".into());
        };
        let slot = eviction.slot;
        cpu.publish_fine(vec![], &[slot as u64])?;
        // Drop old GPU buffers/images before releasing their retained CPU reader.
        drop(eviction);
        if let Some(demand) = &mut self.demand {
            demand.evicted();
            self.camera_event(frame, "evicted")?;
            return Ok(());
        }
        let stats = self.covered_stats(frame)?;
        let history = &mut self
            .retirement
            .as_mut()
            .ok_or("Missing retirement schedule")?
            .history;
        if history.len() > CHUNKS {
            return Err("Retirement history bound exceeded".into());
        }
        println!("Protected fine region {slot} retired; coarse reactivated: {stats}");
        history.push(stats);
        self.retirement.as_mut().unwrap().completed[slot] = true;
        self.retirement_report(frame, false)?;
        Ok(())
    }
    fn covered_stats(&self, frame: u64) -> Result<serde_json::Value> {
        let CpuOwners::Covered { _world: cpu } = &self._cpu else {
            return Err("Missing protected stats owner".into());
        };
        let snapshot = cpu.snapshot();
        Ok(
            serde_json::json!({"frame":frame,"epoch":snapshot.world().epoch(),
            "selected":snapshot.selections().iter().map(|s|s.chunk).collect::<Vec<_>>(),
            "protected_epochs":(4..8).map(|id|snapshot.world().chunk(id).unwrap().epoch()).collect::<Vec<_>>(),
            "owned_chunks":snapshot.world().chunk_count(),"cpu_unique_bytes":cpu.live_bytes(),
            "physical_bytes":self.physical_bytes(),"image_bytes":self.resources.iter().map(|s|s.images.iter().map(|v|v.image.allocated_bytes).sum::<u64>()).sum::<u64>()+
                self._protected.iter().flatten().map(Set::image_bytes).sum::<u64>(),
            "images":self.resources.iter().map(|s|s.images.len()).sum::<usize>()+self._protected.iter().flatten().map(|s|s.images.len()).sum::<usize>()}),
        )
    }
    fn retirement_report(&self, frame: u64, completed: bool) -> Result<()> {
        if let Some(retirement) = &self.retirement
            && let Some(path) = &retirement.report
        {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                path,
                serde_json::to_vec_pretty(&serde_json::json!({
                    "completed":completed,"history":retirement.history,"final":self.covered_stats(frame)?,
                    "retirements":retirement.completed.iter().enumerate().filter(|(i,done)|**done && retirement.history[0]["selected"][*i]==*i).count(),
                "camera_demand":false,"fine_reload":self.reload.is_some(),"reload":self.reload.as_ref().map(|r|r.stats()).transpose()?,
                }))?,
            )?;
        }
        Ok(())
    }
    pub fn benchmark_stats(&self, frame: u64) -> Result<serde_json::Value> {
        self.demand
            .as_ref()
            .ok_or("Missing camera world stats")?
            .frame_stats(self.covered_stats(frame)?, frame)
    }
    pub fn stream_outcome(&self) -> Option<(u64, u64)> {
        self.demand
            .as_ref()
            .map(crate::world_camera::Stream::outcome)
    }
    pub fn qualification_complete(&self, frame: u64) -> Result<()> {
        if self.retirement.is_some()
            && frame >= 81
            && self
                .retirement
                .as_ref()
                .unwrap()
                .completed
                .iter()
                .any(|v| !*v)
        {
            return Err("Scheduled fine retirement did not complete".into());
        }
        self.retirement_report(frame, true)?;
        if let Some(demand) = &self.demand {
            demand.report(self.covered_stats(frame)?, true)?;
        }
        Ok(())
    }
    pub fn enable_camera(
        &mut self,
        route: (u64, usize, Option<PathBuf>),
        budgets: [u64; 2],
        gpu: Gpu<'_>,
        fixed_gpu_bytes: u64,
    ) -> Result<()> {
        let sources = self
            .fine_sources
            .clone()
            .ok_or("Camera demand requires protected source catalog")?;
        self.demand = Some(crate::world_camera::Stream::new(
            sources, budgets, route, gpu,
        )?);
        self.demand.as_mut().unwrap().fixed_gpu_bytes = fixed_gpu_bytes;
        self.camera_event(0, "initial")
    }
    fn camera_event(&mut self, frame: u64, kind: &str) -> Result<()> {
        let row = self.covered_stats(frame)?;
        self.demand.as_mut().unwrap().record(row, kind)?;
        self.demand
            .as_ref()
            .unwrap()
            .report(self.covered_stats(frame)?, false)
    }
    pub fn proxy_push(
        &self,
        frame: u64,
        width: u32,
        height: u32,
        players: u32,
    ) -> Result<[u8; 96]> {
        let CpuOwners::Covered { _world: cpu } = &self._cpu else {
            return Err("World proxies require covered camera world".into());
        };
        let mask =
            cpu.snapshot()
                .selections()
                .iter()
                .enumerate()
                .fold(0u32, |mask, (slot, selection)| {
                    mask | if selection.chunk == slot as u64 {
                        1 << slot
                    } else {
                        0
                    }
                });
        self.demand
            .as_ref()
            .ok_or("Missing camera world demand")?
            .proxy_push(frame, width, height, players, mask)
    }
    pub fn camera_rows(
        &self,
        frame: u64,
        width: u32,
        height: u32,
    ) -> Result<Option<[[f32; 4]; 4]>> {
        self.demand
            .as_ref()
            .map(|s| s.rows(frame, width, height))
            .transpose()
    }
    pub fn tick_camera(&mut self, frame: u64) -> Result<Option<crate::world_camera::Change>> {
        let active = [self.physical_bytes(), self.image_bytes()];
        let CpuOwners::Covered { _world: cpu } = &mut self._cpu else {
            return Ok(None);
        };
        let Some(demand) = &mut self.demand else {
            return Ok(None);
        };
        let tick = demand.tick(frame, cpu, active)?;
        if let Some(event) = tick.event {
            self.camera_event(frame, event)?;
        }
        Ok(tick.change)
    }
    pub fn enable_reload(&mut self, budgets: [u64; 2], gpu: Gpu<'_>) -> Result<()> {
        if self.retirement.is_none() {
            return Err("Fine reload requires the retirement schedule".into());
        }
        let path = self
            .fine_sources
            .as_ref()
            .ok_or("Missing protected fine reload source")?[2]
            .clone();
        self.reload = Some(crate::world_refinement::Reload::new(path, budgets, gpu));
        self.retirement_report(0, false)
    }
    pub fn tick_reload(&mut self, frame: u64) -> Result<Option<crate::world_refinement::Ready>> {
        let active = [self.physical_bytes(), self.image_bytes()];
        let CpuOwners::Covered { _world: cpu } = &mut self._cpu else {
            return Ok(None);
        };
        let Some(reload) = &mut self.reload else {
            return Ok(None);
        };
        let (changed, ready) = reload.tick(frame, cpu, active)?;
        if changed && ready.is_none() {
            self.retirement_report(frame, false)?;
        }
        Ok(ready)
    }
    pub fn prepare_refinement(
        &mut self,
        ready: crate::world_refinement::Ready,
    ) -> Result<FinePublication> {
        let slot = ready.region;
        if slot >= CHUNKS {
            return Err("Fine request region outside profile".into());
        }
        let CpuOwners::Covered { _world: cpu } = &self._cpu else {
            return Err("Missing fine publication owner".into());
        };
        if self._protected[slot].is_some() || cpu.snapshot().world().chunk(slot as u64).is_some() {
            return Err("Fine reload region is already selected".into());
        }
        let [geometry, materials, uvs, table] = ready.set.buffers;
        let old_geometry = self.work[slot].replace_asset(geometry);
        let old = std::mem::replace(
            &mut self.resources[slot],
            PresentationSet {
                materials,
                uvs,
                table,
                images: ready.set.images,
            },
        );
        self._protected[slot] = Some(Set {
            buffers: [old_geometry, old.materials, old.uvs, old.table],
            images: old.images,
        });
        Ok(FinePublication {
            region: slot,
            staged: ready.staged,
        })
    }
    pub fn refinement_published(&mut self, publication: FinePublication, frame: u64) -> Result<()> {
        let CpuOwners::Covered { _world: cpu } = &mut self._cpu else {
            return Err("Missing fine acknowledgement owner".into());
        };
        let slot = publication.region;
        cpu.publish_fine(vec![(slot as u64, publication.staged)], &[])?;
        if let Some(demand) = &mut self.demand {
            demand.published(slot)?;
            self.camera_event(frame, "published")?;
            return Ok(());
        }
        self.reload
            .as_mut()
            .ok_or("Missing fine reload acknowledgement")?
            .published = true;
        let stats = self.covered_stats(frame)?;
        let history = &mut self.retirement.as_mut().unwrap().history;
        if history.len() > CHUNKS + 1 {
            return Err("Fine reload history bound exceeded".into());
        }
        println!("Protected fine region 2 reloaded: {stats}");
        history.push(stats);
        self.retirement_report(frame, false)
    }
    pub fn image_bytes(&self) -> u64 {
        self.resources
            .iter()
            .map(|s| {
                s.images
                    .iter()
                    .map(|v| v.image.allocated_bytes)
                    .sum::<u64>()
            })
            .sum::<u64>()
            + self
                ._protected
                .iter()
                .flatten()
                .map(Set::image_bytes)
                .sum::<u64>()
    }
    pub fn physical_bytes(&self) -> u64 {
        self._protected
            .iter()
            .flatten()
            .map(Set::bytes)
            .sum::<u64>()
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
