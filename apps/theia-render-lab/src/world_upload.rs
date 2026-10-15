//! GPU ownership qualification only: complete chunks upload but are not drawn.
//! Deliberately synchronous, with one local pending set and no graphics readers.
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

struct Set {
    // All transfer fences have retired before these resident owners are exposed.
    buffers: Vec<asset::Buffer>,
    images: Vec<texture::ResidentTexture>,
}
impl Set {
    fn bytes(&self) -> u64 {
        self.buffers.iter().map(|b| b.allocated_bytes).sum::<u64>() + self.image_bytes()
    }
    fn image_bytes(&self) -> u64 {
        self.images.iter().map(|i| i.image.allocated_bytes).sum()
    }
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
        if input
            .roots
            .iter()
            .any(|b| b.len() > gpu.limits.max_storage_buffer_range as usize)
        {
            return Err("World upload root exceeds device storage limit".into());
        }
        self.allocations.retain(|w| w.strong_count() != 0);
        let (active, images) = self.live();
        let mut remaining = self
            .byte_budget
            .checked_sub(active)
            .ok_or("World GPU ledger overflow")?;
        let mut image_remaining = self
            .image_budget
            .checked_sub(images)
            .ok_or("World image ledger overflow")?;
        // Pending transfers are declared after staging, so unwind retires them
        // before releasing encoded ownership. Only one call/pending set at a time.
        let mut buffers = Vec::new();
        let mut pending_images = Vec::new();
        for bytes in &input.roots {
            let upload = asset::Upload::begin(
                gpu.device, gpu.memory, bytes, gpu.family, gpu.queue, remaining, None,
            )?;
            remaining = remaining
                .checked_sub(upload.resident_bytes())
                .ok_or("World buffer accounting underflow")?;
            buffers.push(upload);
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
            pending_images.push(upload);
        }
        // finish waits each dedicated transfer fence; this is a blocking ownership
        // probe, not an asynchronous frame-loop or upload/render overlap claim.
        let buffers = buffers
            .into_iter()
            .map(asset::Upload::finish)
            .collect::<Result<Vec<_>>>()?;
        let images = pending_images
            .into_iter()
            .map(texture::Upload::finish)
            .collect::<Result<Vec<_>>>()?;
        let set = Arc::new(Set { buffers, images });
        assert_eq!(set.bytes(), self.byte_budget - remaining - active);
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
fn budget_rejection(error: &(dyn std::error::Error + 'static)) -> bool {
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
