//! Complete synchronous bundle upload shared by resident pairs and ownership probes.
//! Transfers retire before returned owners can enter a graphics generation.
use crate::{
    Result, asset,
    bundle_streaming::{Gpu, Input},
    texture,
};
use std::{
    collections::BTreeMap,
    io::Read,
    path::Path,
    sync::{Arc, Weak},
};
use theia_assets::bundle_residency::{Snapshot, StagedBundle, WorldResidency};

pub(super) struct Set {
    // All transfer fences have retired before these resident owners are exposed.
    pub buffers: [asset::Buffer; 4],
    pub images: Vec<texture::ResidentTexture>,
}
impl Set {
    pub fn bytes(&self) -> u64 {
        self.buffers.iter().map(|b| b.allocated_bytes).sum::<u64>() + self.image_bytes()
    }
    pub fn image_bytes(&self) -> u64 {
        self.images.iter().map(|i| i.image.allocated_bytes).sum()
    }
}
/// Owns every submitted transfer until completion or cancellation. Caller keeps
/// its complete CPU reservation alive until this owner has retired all copies.
pub(super) struct PendingSet {
    buffers: Vec<asset::Upload>,
    images: Vec<texture::Upload>,
}
impl PendingSet {
    pub fn begin(
        input: &Input,
        gpu: &Gpu<'_>,
        mut remaining: u64,
        mut image_remaining: u64,
    ) -> Result<Self> {
        if input
            .roots
            .iter()
            .any(|b| b.len() > gpu.limits.max_storage_buffer_range as usize)
        {
            return Err("World upload root exceeds device storage limit".into());
        }
        // Partial failure retires transfers before releasing the caller's CPU
        // reservation. Success returns pending ownership for polling or finish.
        let mut pending = Self {
            buffers: Vec::new(),
            images: Vec::new(),
        };
        for bytes in &input.roots {
            let upload = asset::Upload::begin(
                gpu.device, gpu.memory, bytes, gpu.family, gpu.queue, remaining, None,
            )?;
            remaining = remaining
                .checked_sub(upload.resident_bytes())
                .ok_or("World buffer accounting underflow")?;
            pending.buffers.push(upload);
        }
        for bytes in &input.leaves {
            let image = theia_assets::texture::Texture::decode(
                bytes,
                None,
                theia_assets::texture::DEFAULT_BUDGET,
            )?;
            if image.width > gpu.image_limits.max_extent.width
                || image.height > gpu.image_limits.max_extent.height
            {
                return Err("World image exceeds device format limits".into());
            }
            let upload = texture::Upload::begin(
                gpu.device,
                gpu.memory,
                &image,
                gpu.family,
                gpu.queue,
                remaining.min(image_remaining),
            )?;
            remaining = remaining
                .checked_sub(upload.resident_bytes())
                .ok_or("World image accounting underflow")?;
            image_remaining = image_remaining
                .checked_sub(upload.resident_bytes())
                .ok_or("World image-only accounting underflow")?;
            pending.images.push(upload);
        }
        Ok(pending)
    }
    pub fn ready(&self) -> Result<bool> {
        for upload in &self.buffers {
            if !upload.ready()? {
                return Ok(false);
            }
        }
        for upload in &self.images {
            if !upload.ready()? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn physical_bytes(&self) -> u64 {
        self.buffers
            .iter()
            .map(asset::Upload::resident_bytes)
            .sum::<u64>()
            + self.image_bytes()
    }
    pub fn image_bytes(&self) -> u64 {
        self.images
            .iter()
            .map(texture::Upload::resident_bytes)
            .sum()
    }
    pub fn staging_bytes(&self) -> u64 {
        self.buffers
            .iter()
            .map(asset::Upload::staging_bytes)
            .chain(self.images.iter().map(texture::Upload::staging_bytes))
            .sum()
    }
    pub fn finish(self) -> Result<Set> {
        let buffers = self
            .buffers
            .into_iter()
            .map(asset::Upload::finish)
            .collect::<Result<Vec<_>>>()?;
        let images = self
            .images
            .into_iter()
            .map(texture::Upload::finish)
            .collect::<Result<Vec<_>>>()?;
        Ok(Set {
            buffers: buffers
                .try_into()
                .map_err(|_| "Wrong complete GPU root count")?,
            images,
        })
    }
}
/// Blocking initial upload; frame-loop consumers poll PendingSet before finish.
pub(super) fn upload(
    input: &Input,
    gpu: &Gpu<'_>,
    remaining: u64,
    image_remaining: u64,
) -> Result<Set> {
    PendingSet::begin(input, gpu, remaining, image_remaining)?.finish()
}

struct Ready {
    // GPU owners drop before the complete CPU reservation on cancellation/failure.
    gpu: Arc<Set>,
    staged: StagedBundle,
}
#[derive(Clone)]
struct Reader {
    gpu: Arc<Set>,
    cpu: Snapshot,
}
struct World {
    current: BTreeMap<u64, Reader>,
    allocations: Vec<Weak<Set>>,
    cpu: WorldResidency,
    byte_budget: u64,
    image_budget: u64,
}
impl World {
    fn new(cpu_budget: usize, slots: usize, byte_budget: u64, image_budget: u64) -> Result<Self> {
        Ok(Self {
            current: BTreeMap::new(),
            allocations: Vec::new(),
            cpu: WorldResidency::new(cpu_budget, slots)?,
            byte_budget,
            image_budget,
        })
    }
    fn live(&self) -> (u64, u64) {
        self.allocations
            .iter()
            .filter_map(Weak::upgrade)
            .fold((0, 0), |(bytes, images), set| {
                (bytes + set.bytes(), images + set.image_bytes())
            })
    }
    fn stage(&mut self, input: &Input, gpu: &Gpu<'_>) -> Result<Ready> {
        let staged = self.cpu.stage(
            &input.manifest,
            theia_assets::content_id(&input.manifest),
            std::array::from_fn(|i| input.roots[i].as_slice()),
            &input.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        )?;
        self.allocations.retain(|w| w.strong_count() != 0);
        let (active, images) = self.live();
        let remaining = self
            .byte_budget
            .checked_sub(active)
            .ok_or("World GPU ledger overflow")?;
        let image_remaining = self
            .image_budget
            .checked_sub(images)
            .ok_or("World image ledger overflow")?;
        let set = Arc::new(upload(input, gpu, remaining, image_remaining)?);
        assert!(set.bytes() <= remaining && set.image_bytes() <= image_remaining);
        self.allocations.push(Arc::downgrade(&set));
        Ok(Ready { gpu: set, staged })
    }
    /// This probe never submits graphics using the owners. A renderer consumer
    /// must additionally retire actual descriptor/frame readers before removal.
    fn publish(&mut self, id: u64, ready: Ready) -> Result<()> {
        let snapshot = self.cpu.publish(vec![(id, ready.staged)], &[])?;
        self.current.insert(
            id,
            Reader {
                gpu: ready.gpu,
                cpu: snapshot.chunk(id).unwrap().clone(),
            },
        );
        Ok(())
    }
    fn remove(&mut self, ids: &[u64]) -> Result<()> {
        self.cpu.publish(vec![], ids)?;
        for id in ids {
            self.current.remove(id);
        }
        Ok(())
    }
}
pub(super) fn budget_rejection(error: &(dyn std::error::Error + 'static)) -> bool {
    error
        .downcast_ref::<asset::AllocationBudgetExceeded>()
        .is_some()
        || error.to_string() == "Texture image allocation exceeds physical byte budget"
}
pub fn qualify(path: &Path, gpu: Gpu<'_>) -> Result<()> {
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
        .ok_or("World fixture has no quadrants")?;
    if scene["passed"] != true || entries.len() != 4 {
        return Err("World upload needs checked four-quadrant fixture".into());
    }
    let inputs = entries
        .iter()
        .map(|entry| {
            Input::load(Path::new(
                entry["cook"]["bundle"]["path"]
                    .as_str()
                    .ok_or("Missing world bundle path")?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut world = World::new(64 * 1024 * 1024, 4, 32 * 1024 * 1024, 4 * 1024 * 1024)?;
    let cancelled = world.stage(&inputs[0], &gpu)?;
    assert!(world.live().0 > 0);
    drop(cancelled);
    assert_eq!(world.live(), (0, 0));
    assert_eq!(world.cpu.live_bytes(), 0);
    assert_eq!(world.cpu.snapshot().epoch(), 0);
    let mut sizes = Vec::new();
    for (id, input) in inputs.iter().enumerate() {
        let ready = world.stage(input, &gpu)?;
        sizes.push((ready.gpu.bytes(), ready.gpu.image_bytes()));
        world.publish(id as u64, ready)?;
    }
    let total: u64 = sizes.iter().map(|s| s.0).sum();
    let images: u64 = sizes.iter().map(|s| s.1).sum();
    let cpu_bytes = world.cpu.live_bytes();
    assert_eq!(world.live(), (total, images));
    let retained = world.current[&0].clone();
    assert!(retained.cpu.bundle_id().is_some());
    assert_eq!(retained.gpu.bytes(), sizes[0].0);
    world.remove(&[0])?;
    assert_eq!(world.live(), (total, images));
    world.byte_budget = total;
    let error = world
        .stage(&inputs[0], &gpu)
        .err()
        .ok_or("Retained GPU reader must prevent reuse")?;
    if !budget_rejection(error.as_ref()) {
        return Err(error);
    }
    assert_eq!(world.live(), (total, images));
    drop(retained);
    assert_eq!(world.live(), (total - sizes[0].0, images - sizes[0].1));
    let ready = world.stage(&inputs[0], &gpu)?;
    world.publish(0, ready)?;
    assert_eq!(world.live(), (total, images));
    world.remove(&[0, 1, 2, 3])?;
    assert_eq!(world.live(), (0, 0));
    assert_eq!(world.cpu.live_bytes(), 0);
    // One-byte-short aggregate buffer+image cap, then independent image cap.
    for (byte_budget, image_budget) in [(total - 1, images), (total, images - 1)] {
        let mut limited = World::new(cpu_bytes, 4, byte_budget, image_budget)?;
        for (id, input) in inputs.iter().enumerate() {
            match limited.stage(input, &gpu) {
                Ok(ready) => limited.publish(id as u64, ready)?,
                Err(error) => {
                    if !budget_rejection(error.as_ref()) {
                        return Err(error);
                    }
                    assert_eq!(id, 3);
                    assert_eq!(limited.current.len(), 3);
                    assert_eq!(limited.cpu.snapshot().epoch(), 3);
                    assert_eq!(
                        limited.live(),
                        (
                            sizes[..3].iter().map(|s| s.0).sum(),
                            sizes[..3].iter().map(|s| s.1).sum()
                        )
                    );
                    break;
                }
            }
        }
        assert_eq!(limited.current.len(), 3);
    }
    println!(
        "World upload ownership verified: {}",
        serde_json::json!({
        "chunks":4,"root_buffers":16,"images":16,"physical_bytes":total,
        "image_bytes":images,"cpu_unique_bytes":cpu_bytes,"per_chunk":sizes,
        "retained_reader_rejection":true,"last_owner_reuse":true,"cancelled_ready_released":true,
        "short_aggregate_rejection":true,"short_image_rejection":true,
        "rendered_chunks":false,"asynchronous_streaming":false})
    );
    Ok(())
}

pub(super) struct PresentationSet {
    pub materials: asset::Buffer,
    pub uvs: asset::Buffer,
    pub table: asset::Buffer,
    pub images: Vec<texture::ResidentTexture>,
}
impl PresentationSet {
    pub fn bytes(&self) -> u64 {
        self.materials.allocated_bytes
            + self.uvs.allocated_bytes
            + self.table.allocated_bytes
            + self
                .images
                .iter()
                .map(|i| i.image.allocated_bytes)
                .sum::<u64>()
    }
}
pub(super) struct Pair {
    pub work: [crate::indirect::IndirectWork; 2],
    pub resources: [PresentationSet; 2],
    pub cpu: WorldResidency,
}
pub(super) fn check_pair_limits(images: u32, gpu: &Gpu<'_>) -> Result<()> {
    if gpu.limits.max_per_stage_descriptor_storage_buffers < 8
        || gpu.limits.max_descriptor_set_storage_buffers < 8
        || gpu.limits.max_per_stage_descriptor_sampled_images < images + 4
        || gpu.limits.max_descriptor_set_sampled_images < images + 4
        || gpu.limits.max_per_stage_descriptor_samplers < 3
        || gpu.limits.max_descriptor_set_samplers < 3
    {
        return Err("Textured world pair exceeds storage/image/sampler descriptor limits".into());
    }
    Ok(())
}
pub(super) fn resident_pair(
    paths: [&Path; 2],
    gpu: Gpu<'_>,
    shaders: &Path,
    cpu_budget: usize,
    mut byte_budget: u64,
    mut image_budget: u64,
) -> Result<Pair> {
    let requested = (byte_budget, image_budget);
    let inputs = paths
        .map(Input::load)
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let images: u32 = inputs.iter().map(|i| i.leaves.len() as u32).sum();
    check_pair_limits(images, &gpu)?;
    let mut cpu = WorldResidency::new(cpu_budget, 2)?;
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
    let mut work = Vec::new();
    let mut resources = Vec::new();
    for input in &inputs {
        let set = upload(input, &gpu, byte_budget, image_budget)?;
        byte_budget = byte_budget
            .checked_sub(set.bytes())
            .ok_or("Pair physical accounting underflow")?;
        image_budget = image_budget
            .checked_sub(set.image_bytes())
            .ok_or("Pair image accounting underflow")?;
        let [geometry, materials, uvs, table] = set.buffers;
        work.push(crate::indirect::IndirectWork::resident(
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
        "Resident textured pair ownership: {}",
        serde_json::json!({
        "chunks":2,"physical_bytes":requested.0-byte_budget,
        "image_bytes":requested.1-image_budget,"cpu_unique_bytes":cpu.live_bytes(),
        "root_buffers":8,"images":images,"cpu_epoch":snapshot.epoch()})
    );

    println!(
        "Resident textured pair ready: 2 complete GPU chunks, {} CPU unique encoded bytes, CPU epoch {}; all transfers retired before graphics generation",
        cpu.live_bytes(),
        snapshot.epoch()
    );
    Ok(Pair {
        work: work.try_into().map_err(|_| "Wrong pair work count")?,
        resources: resources
            .try_into()
            .map_err(|_| "Wrong pair resource count")?,
        cpu,
    })
}
