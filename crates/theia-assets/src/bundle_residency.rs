//! Bounded complete-bundle CPU ownership. GPU completion/publication is separate.
use crate::{
    AssetError, Result,
    bundle::{Bundle, BundleProfile},
    content_id,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

pub type ResourceId = [u8; 32];
static NEXT_STORE: AtomicU64 = AtomicU64::new(1);

/// Immutable complete set, including its manifest and every texture-table leaf.
/// Retaining a snapshot retains its encoded resources across later publications.
#[derive(Clone, Default)]
pub struct Snapshot {
    epoch: u64,
    bundle: Option<ResourceId>,
    resources: BTreeMap<ResourceId, Arc<[u8]>>,
}
impl Snapshot {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn bundle_id(&self) -> Option<ResourceId> {
        self.bundle
    }
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }
    pub fn resource(&self, id: &ResourceId) -> Option<&[u8]> {
        self.resources.get(id).map(AsRef::as_ref)
    }
}
/// Opaque complete reservation. Dropping it cancels staging without publication.
pub struct StagedBundle {
    owner: u64,
    bundle: ResourceId,
    resources: BTreeMap<ResourceId, Arc<[u8]>>,
}

pub struct Residency {
    owner: u64,
    budget: usize,
    allocations: BTreeMap<ResourceId, Weak<[u8]>>,
    current: Snapshot,
}
impl Residency {
    pub fn new(budget: usize) -> Result<Self> {
        let owner = NEXT_STORE
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| AssetError("bundle residency identity exhausted"))?;
        Ok(Self {
            owner,
            budget,
            allocations: BTreeMap::new(),
            current: Snapshot::default(),
        })
    }
    /// Unique live encoded bytes across staging, current and retained snapshots.
    /// Includes manifests; excludes input/decoded temporaries, Arc/map overhead and VRAM.
    pub fn live_bytes(&self) -> usize {
        self.allocations
            .values()
            .filter_map(Weak::upgrade)
            .map(|v| v.len())
            .sum()
    }
    pub fn snapshot(&self) -> Snapshot {
        self.current.clone()
    }

    /// Admit the full bounded graph before reserving any encoded ownership.
    /// v1 has no additional leaves; v2 leaves follow the table's canonical order.
    /// Caller owns I/O inputs, which are outside this residency budget.
    pub fn stage(
        &mut self,
        manifest: &[u8],
        expected: ResourceId,
        roots: [&[u8]; 4],
        leaves: &[&[u8]],
    ) -> Result<StagedBundle> {
        let bundle = Bundle::decode(manifest, Some(expected))?;
        match bundle.profile {
            BundleProfile::SingleTexture => {
                if !leaves.is_empty() {
                    return Err(AssetError("single-texture bundle has unexpected leaves"));
                }
                bundle.validate(roots)?;
            }
            BundleProfile::TextureTable => {
                let admitted = bundle.validate_table_roots(roots)?;
                admitted.table.validate_textures(leaves)?;
            }
        }
        let mut inputs = BTreeMap::new();
        for bytes in std::iter::once(manifest)
            .chain(roots)
            .chain(leaves.iter().copied())
        {
            let id = content_id(bytes);
            if let Some(previous) = inputs.insert(id, bytes)
                && previous != bytes
            {
                return Err(AssetError("resource digest collision"));
            }
        }
        self.allocations
            .retain(|_, value| value.strong_count() != 0);
        // Hold deduplicated owners while checking the entire reservation. No partial
        // allocation enters the store on admission, collision or budget failure.
        let mut resources = BTreeMap::new();
        let mut required = self.live_bytes();
        for (id, bytes) in &inputs {
            if let Some(existing) = self.allocations.get(id).and_then(Weak::upgrade) {
                if existing.as_ref() != *bytes {
                    return Err(AssetError("live resource digest collision"));
                }
                resources.insert(*id, existing);
            } else {
                required = required
                    .checked_add(bytes.len())
                    .ok_or(AssetError("bundle residency size overflow"))?;
            }
        }
        if required > self.budget {
            return Err(AssetError("live bundle byte budget exhausted"));
        }
        for (id, bytes) in inputs {
            resources.entry(id).or_insert_with(|| {
                let allocation: Arc<[u8]> = Arc::from(bytes);
                self.allocations.insert(id, Arc::downgrade(&allocation));
                allocation
            });
        }
        Ok(StagedBundle {
            owner: self.owner,
            bundle: expected,
            resources,
        })
    }
    /// Replace the whole CPU set atomically, or publish an empty set with None.
    /// This method does not prove GPU upload completion or update descriptors.
    pub fn publish(&mut self, staged: Option<StagedBundle>) -> Result<Snapshot> {
        if staged.as_ref().is_some_and(|v| v.owner != self.owner) {
            return Err(AssetError("foreign staged bundle"));
        }
        let epoch = self
            .current
            .epoch
            .checked_add(1)
            .ok_or(AssetError("bundle residency epoch exhausted"))?;
        self.current = if let Some(staged) = staged {
            Snapshot {
                epoch,
                bundle: Some(staged.bundle),
                resources: staged.resources,
            }
        } else {
            Snapshot {
                epoch,
                ..Snapshot::default()
            }
        };
        Ok(self.snapshot())
    }
}

/// Application-supplied stable chunk identity; distinct chunks may share a bundle.
pub type ChunkId = u64;
pub const MAX_WORLD_CHUNKS: usize = 4096;

/// Atomic CPU view of independently published complete bundles.
/// Each chunk's epoch records its last publication; this epoch records the world
/// transaction. A retained world or chunk snapshot keeps its immutable bytes alive.
#[derive(Clone, Default)]
pub struct WorldSnapshot {
    epoch: u64,
    chunks: BTreeMap<ChunkId, Snapshot>,
}
impl WorldSnapshot {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
    pub fn chunk_ids(&self) -> impl Iterator<Item = ChunkId> + '_ {
        self.chunks.keys().copied()
    }
    pub fn chunk(&self, id: ChunkId) -> Option<&Snapshot> {
        self.chunks.get(&id)
    }
}

/// Bounded CPU ownership prerequisite for independent world-chunk residency.
/// No I/O, camera demand, eviction policy, GPU completion or descriptor publication
/// is implied. The shared pool charges unique encoded bytes across all chunks,
/// staged reservations and retained readers, including removed/replaced chunks.
pub struct WorldResidency {
    pool: Residency,
    capacity: usize,
    current: WorldSnapshot,
}
impl WorldResidency {
    pub fn new(byte_budget: usize, chunk_capacity: usize) -> Result<Self> {
        if !(1..=MAX_WORLD_CHUNKS).contains(&chunk_capacity) {
            return Err(AssetError("invalid world chunk capacity"));
        }
        Ok(Self {
            pool: Residency::new(byte_budget)?,
            capacity: chunk_capacity,
            current: WorldSnapshot::default(),
        })
    }
    pub fn live_bytes(&self) -> usize {
        self.pool.live_bytes()
    }
    pub fn snapshot(&self) -> WorldSnapshot {
        self.current.clone()
    }
    /// Complete admission and reservation use the same contract as Residency.
    /// A staged reservation is not yet a published chunk and consumes no slot.
    pub fn stage(
        &mut self,
        manifest: &[u8],
        expected: ResourceId,
        roots: [&[u8]; 4],
        leaves: &[&[u8]],
    ) -> Result<StagedBundle> {
        self.pool.stage(manifest, expected, roots, leaves)
    }
    /// Atomically add/replace named chunks and remove existing chunks. Duplicate
    /// IDs, unknown removals, add/remove conflicts, foreign tokens, capacity and
    /// epoch exhaustion fail before changing the current world. Failure consumes
    /// the supplied tokens, releasing reservations when their last owner drops.
    /// Empty transactions are rejected. Replacing an existing ID needs no removal.
    pub fn publish(
        &mut self,
        additions: Vec<(ChunkId, StagedBundle)>,
        removals: &[ChunkId],
    ) -> Result<WorldSnapshot> {
        if additions.len() > self.capacity
            || removals.len() > self.capacity
            || (additions.is_empty() && removals.is_empty())
        {
            return Err(AssetError("invalid world transaction size"));
        }
        let mut removed = std::collections::BTreeSet::new();
        for id in removals {
            if !removed.insert(*id) || !self.current.chunks.contains_key(id) {
                return Err(AssetError("duplicate or unknown world removal"));
            }
        }
        let mut added = std::collections::BTreeSet::new();
        for (id, staged) in &additions {
            if !added.insert(*id) || removed.contains(id) {
                return Err(AssetError("duplicate or conflicting world addition"));
            }
            if staged.owner != self.pool.owner {
                return Err(AssetError("foreign staged world bundle"));
            }
        }
        let new_slots = added
            .iter()
            .filter(|id| !self.current.chunks.contains_key(id))
            .count();
        if self.current.chunks.len() - removed.len() + new_slots > self.capacity {
            return Err(AssetError("world chunk capacity exhausted"));
        }
        let epoch = self
            .current
            .epoch
            .checked_add(1)
            .ok_or(AssetError("world residency epoch exhausted"))?;
        let mut next = self.current.clone();
        next.epoch = epoch;
        for id in removals {
            next.chunks.remove(id);
        }
        for (id, staged) in additions {
            next.chunks.insert(
                id,
                Snapshot {
                    epoch,
                    bundle: Some(staged.bundle),
                    resources: staged.resources,
                },
            );
        }
        self.current = next;
        Ok(self.snapshot())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Input {
        manifest: Vec<u8>,
        roots: [Vec<u8>; 4],
        leaves: Vec<Vec<u8>>,
    }
    impl Input {
        fn fixture(tag: u8, indexed: bool) -> Self {
            let (_, mut roots) = crate::bundle::tests::fixture();
            let offset = roots[3].len() - 16;
            roots[3][offset] = tag;
            let mut leaves = Vec::new();
            if indexed {
                let texture = std::mem::take(&mut roots[3]);
                let mut payload = b"table\0\0\0".to_vec();
                payload.extend(content_id(&texture));
                payload.extend((texture.len() as u64).to_le_bytes());
                payload.extend(0u32.to_le_bytes());
                let mut table = b"THEIATEX".to_vec();
                for value in [1u32, 5, 1, 1] {
                    table.extend(value.to_le_bytes());
                }
                table.extend(content_id(&roots[1]));
                table.extend(content_id(&payload));
                table.extend(payload);
                roots[3] = table;
                leaves.push(texture);
            }
            let mut result = Self {
                manifest: Vec::new(),
                roots,
                leaves,
            };
            result.rebind(indexed);
            result
        }
        fn rebind(&mut self, indexed: bool) {
            self.manifest = b"THEIABND".to_vec();
            self.manifest
                .extend((if indexed { 2u32 } else { 1u32 }).to_le_bytes());
            self.manifest.extend(4u32.to_le_bytes());
            for bytes in &self.roots {
                self.manifest.extend(content_id(bytes));
                self.manifest.extend((bytes.len() as u64).to_le_bytes());
            }
        }
        fn stage(&self, store: &mut Residency) -> Result<StagedBundle> {
            store.stage(
                &self.manifest,
                content_id(&self.manifest),
                std::array::from_fn(|i| self.roots[i].as_slice()),
                &self.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            )
        }
        fn stage_world(&self, store: &mut WorldResidency) -> Result<StagedBundle> {
            store.stage(
                &self.manifest,
                content_id(&self.manifest),
                std::array::from_fn(|i| self.roots[i].as_slice()),
                &self.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            )
        }
        fn bytes(&self) -> BTreeMap<ResourceId, usize> {
            std::iter::once(&self.manifest)
                .chain(&self.roots)
                .chain(&self.leaves)
                .map(|v| (content_id(v), v.len()))
                .collect()
        }
    }
    #[test]
    fn whole_set_reservations_deduplicate_cancel_and_count_manifests() {
        let single = Input::fixture(42, false);
        let indexed = Input::fixture(42, true);
        let mut union = single.bytes();
        union.extend(indexed.bytes());
        let total: usize = union.values().sum();
        let mut store = Residency::new(total).unwrap();
        let a = single.stage(&mut store).unwrap();
        let b = indexed.stage(&mut store).unwrap();
        let duplicate = indexed.stage(&mut store).unwrap();
        assert_eq!(store.live_bytes(), total);
        assert_eq!(a.resources.len(), 5);
        assert_eq!(b.resources.len(), 6);
        // The texture is shared across different bundle profiles as well as stages.
        let tex = content_id(&single.roots[3]);
        assert!(Arc::ptr_eq(&a.resources[&tex], &b.resources[&tex]));
        drop(b);
        drop(duplicate);
        assert_eq!(store.live_bytes(), single.bytes().values().sum());
        drop(a);
        assert_eq!(store.live_bytes(), 0);
    }
    #[test]
    fn admission_and_budget_failure_leave_current_and_reservations_unchanged() {
        let a = Input::fixture(42, false);
        let b = Input::fixture(43, true);
        let size = a.bytes().values().sum();
        let mut store = Residency::new(size).unwrap();
        let staged = a.stage(&mut store).unwrap();
        store.publish(Some(staged)).unwrap();
        assert!(b.stage(&mut store).is_err());
        assert_eq!(store.live_bytes(), size);
        let mut missing = Input::fixture(42, true);
        missing.leaves.clear();
        assert!(missing.stage(&mut store).is_err());
        let mut corrupt = Input::fixture(42, true);
        corrupt.leaves[0][0] ^= 1;
        assert!(corrupt.stage(&mut store).is_err());
        let mut foreign_uv = Input::fixture(42, false);
        foreign_uv.roots[2][24] ^= 1;
        foreign_uv.rebind(false);
        assert!(foreign_uv.stage(&mut store).is_err());
        let roots = std::array::from_fn(|i| a.roots[i].as_slice());
        assert!(store.stage(&a.manifest, [0; 32], roots, &[]).is_err());
        assert!(
            store
                .stage(&a.manifest, content_id(&a.manifest), roots, &[&a.roots[3]])
                .is_err()
        );
        assert_eq!(store.snapshot().epoch(), 1);
        assert_eq!(store.snapshot().bundle_id(), Some(content_id(&a.manifest)));
        assert_eq!(store.live_bytes(), size);
        // A one-byte-short empty store rejects the entire set without partial charging.
        let mut short = Residency::new(size - 1).unwrap();
        assert!(a.stage(&mut short).is_err());
        assert_eq!(short.live_bytes(), 0);
        assert_eq!(short.snapshot().epoch(), 0);
    }
    #[test]
    fn readers_block_reuse_and_snapshot_access_remains_complete() {
        let a = Input::fixture(42, true);
        let b = Input::fixture(43, true);
        let mut store = Residency::new(a.bytes().values().sum()).unwrap();
        let staged = a.stage(&mut store).unwrap();
        let reader = store.publish(Some(staged)).unwrap();
        let empty = store.publish(None).unwrap();
        assert_eq!(empty.resource_count(), 0);
        for (id, length) in a.bytes() {
            assert_eq!(reader.resource(&id).unwrap().len(), length);
        }
        assert!(b.stage(&mut store).is_err());
        drop(reader);
        assert_eq!(store.live_bytes(), 0);
        let staged = b.stage(&mut store).unwrap();
        assert_eq!(store.publish(Some(staged)).unwrap().epoch(), 3);
    }
    #[test]
    fn foreign_staging_and_exhausted_epoch_cannot_replace_current() {
        let a = Input::fixture(42, false);
        let b = Input::fixture(43, false);
        let mut store = Residency::new(usize::MAX).unwrap();
        let mut other = Residency::new(usize::MAX).unwrap();
        let staged = a.stage(&mut store).unwrap();
        store.publish(Some(staged)).unwrap();
        assert!(store.publish(Some(b.stage(&mut other).unwrap())).is_err());
        assert_eq!(store.snapshot().bundle_id(), Some(content_id(&a.manifest)));
        store.current.epoch = u64::MAX;
        let before = store.live_bytes();
        let staged = b.stage(&mut store).unwrap();
        assert!(store.publish(Some(staged)).is_err());
        assert_eq!(store.live_bytes(), before);
        assert!(store.publish(None).is_err());
        assert_eq!(store.snapshot().bundle_id(), Some(content_id(&a.manifest)));
    }
    #[test]
    fn randomized_stages_publications_readers_match_independent_byte_union() {
        let inputs: Vec<_> = (0..4).map(|i| Input::fixture(i, i % 2 == 0)).collect();
        let budget = inputs[0].bytes().values().sum::<usize>() + 500;
        let mut store = Residency::new(budget).unwrap();
        let mut current = BTreeMap::new();
        let mut readers: Vec<(Snapshot, BTreeMap<ResourceId, usize>)> = Vec::new();
        let mut stages: Vec<(StagedBundle, BTreeMap<ResourceId, usize>)> = Vec::new();
        let mut seed = 7919u64;
        for _ in 0..3000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let mut live = current.clone();
            for (_, ids) in &readers {
                live.extend(ids);
            }
            for (_, ids) in &stages {
                live.extend(ids);
            }
            match (seed >> 32) % 6 {
                0 => {
                    let input = &inputs[seed as usize % inputs.len()];
                    let ids = input.bytes();
                    let mut expected = live.clone();
                    expected.extend(&ids);
                    let staged = input.stage(&mut store);
                    assert_eq!(staged.is_ok(), expected.values().sum::<usize>() <= budget);
                    if let Ok(staged) = staged {
                        stages.push((staged, ids));
                    }
                }
                1 if !stages.is_empty() => {
                    let (staged, ids) = stages.swap_remove(seed as usize % stages.len());
                    store.publish(Some(staged)).unwrap();
                    current = ids;
                }
                2 => {
                    store.publish(None).unwrap();
                    current.clear();
                }
                3 => readers.push((store.snapshot(), current.clone())),
                4 if !readers.is_empty() => {
                    readers.swap_remove(seed as usize % readers.len());
                }
                5 if !stages.is_empty() => {
                    stages.swap_remove(seed as usize % stages.len());
                }
                _ => {}
            }
            let mut expected = current.clone();
            for (snapshot, ids) in &readers {
                expected.extend(ids);
                assert_eq!(snapshot.resource_count(), ids.len());
                for (id, size) in ids {
                    assert_eq!(snapshot.resource(id).unwrap().len(), *size);
                }
            }
            for (_, ids) in &stages {
                expected.extend(ids);
            }
            assert_eq!(store.live_bytes(), expected.values().sum());
            assert!(store.live_bytes() <= budget);
        }
    }
    #[test]
    fn world_chunks_share_owners_and_publish_independently() {
        let a = Input::fixture(42, true);
        let b = Input::fixture(43, true);
        let mut union = a.bytes();
        union.extend(b.bytes());
        let mut store = WorldResidency::new(union.values().sum(), 3).unwrap();
        let sa = a.stage_world(&mut store).unwrap();
        let duplicate = a.stage_world(&mut store).unwrap();
        let sb = b.stage_world(&mut store).unwrap();
        let first = store
            .publish(vec![(8, sa), (2, sb), (99, duplicate)], &[])
            .unwrap();
        assert_eq!(first.chunk_ids().collect::<Vec<_>>(), [2, 8, 99]);
        assert_eq!(first.epoch(), 1);
        assert_eq!(store.live_bytes(), union.values().sum());
        for id in a.bytes().keys() {
            assert!(Arc::ptr_eq(
                &first.chunks[&8].resources[id],
                &first.chunks[&99].resources[id]
            ));
        }
        let replacement = b.stage_world(&mut store).unwrap();
        let second = store.publish(vec![(8, replacement)], &[2, 99]).unwrap();
        assert_eq!(second.chunk_count(), 1);
        assert_eq!(second.chunk(8).unwrap().epoch(), 2);
        assert_eq!(first.chunk_count(), 3);
        assert_eq!(store.live_bytes(), union.values().sum());
        drop(first);
        assert_eq!(store.live_bytes(), b.bytes().values().sum());
        let retained_chunk = second.chunk(8).unwrap().clone();
        drop(second);
        store.publish(vec![], &[8]).unwrap();
        assert_eq!(store.live_bytes(), b.bytes().values().sum());
        drop(retained_chunk);
        assert_eq!(store.live_bytes(), 0);
    }

    #[test]
    fn world_transactions_fail_atomically_and_enforce_slots() {
        let a = Input::fixture(42, true);
        assert!(WorldResidency::new(0, 0).is_err());
        assert!(WorldResidency::new(0, MAX_WORLD_CHUNKS + 1).is_err());
        let mut store = WorldResidency::new(usize::MAX, 2).unwrap();
        let first = a.stage_world(&mut store).unwrap();
        store.publish(vec![(0, first)], &[]).unwrap();
        let mut other = WorldResidency::new(usize::MAX, 2).unwrap();
        let foreign = a.stage_world(&mut other).unwrap();
        let valid = a.stage_world(&mut store).unwrap();
        assert!(store.publish(vec![(1, valid), (2, foreign)], &[]).is_err());
        assert!(store.publish(vec![], &[]).is_err());
        assert!(store.publish(vec![], &[123]).is_err());
        assert!(store.publish(vec![], &[0, 0]).is_err());
        let staged = a.stage_world(&mut store).unwrap();
        assert!(store.publish(vec![(0, staged)], &[0]).is_err());
        let sa = a.stage_world(&mut store).unwrap();
        let sb = a.stage_world(&mut store).unwrap();
        assert!(store.publish(vec![(1, sa), (1, sb)], &[]).is_err());
        let sa = a.stage_world(&mut store).unwrap();
        let sb = a.stage_world(&mut store).unwrap();
        assert!(store.publish(vec![(1, sa), (2, sb)], &[]).is_err());
        assert_eq!(store.snapshot().epoch(), 1);
        assert_eq!(store.snapshot().chunk_ids().collect::<Vec<_>>(), [0]);
        assert_eq!(store.live_bytes(), a.bytes().values().sum());
        let staged = a.stage_world(&mut store).unwrap();
        store.publish(vec![(u64::MAX, staged)], &[0]).unwrap();
        store.current.epoch = u64::MAX;
        let staged = a.stage_world(&mut store).unwrap();
        assert!(store.publish(vec![(0, staged)], &[u64::MAX]).is_err());
        assert_eq!(store.snapshot().chunk_ids().collect::<Vec<_>>(), [u64::MAX]);
    }

    #[test]
    fn world_exact_budget_short_budget_and_retained_eviction() {
        let a = Input::fixture(42, true);
        let b = Input::fixture(43, true);
        let size = a.bytes().values().sum::<usize>();
        let mut short = WorldResidency::new(size - 1, 2).unwrap();
        assert!(a.stage_world(&mut short).is_err());
        assert_eq!(short.live_bytes(), 0);
        let mut store = WorldResidency::new(size, 2).unwrap();
        let staged = a.stage_world(&mut store).unwrap();
        let reader = store.publish(vec![(1, staged)], &[]).unwrap();
        assert!(b.stage_world(&mut store).is_err());
        store.publish(vec![], &[1]).unwrap();
        assert!(b.stage_world(&mut store).is_err());
        assert_eq!(store.snapshot().epoch(), 2);
        drop(reader);
        assert_eq!(store.live_bytes(), 0);
        let staged = b.stage_world(&mut store).unwrap();
        assert_eq!(store.publish(vec![(2, staged)], &[]).unwrap().epoch(), 3);
    }

    #[test]
    fn randomized_world_transactions_match_independent_retained_resource_union() {
        let inputs: Vec<_> = (0..4).map(|i| Input::fixture(i, i % 2 == 0)).collect();
        let budget = inputs[0].bytes().values().sum::<usize>() + 800;
        let mut store = WorldResidency::new(budget, 3).unwrap();
        let mut current = BTreeMap::<u64, usize>::new();
        let mut readers = Vec::<(WorldSnapshot, BTreeMap<u64, usize>)>::new();
        let mut stages = Vec::<(StagedBundle, usize)>::new();
        let mut seed = 12347u64;
        for _ in 0..4000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let live = |current: &BTreeMap<u64, usize>,
                        readers: &[(WorldSnapshot, BTreeMap<u64, usize>)],
                        stages: &[(StagedBundle, usize)]| {
                let mut union = BTreeMap::new();
                for index in current
                    .values()
                    .chain(readers.iter().flat_map(|(_, c)| c.values()))
                    .copied()
                    .chain(stages.iter().map(|(_, i)| *i))
                {
                    union.extend(inputs[index].bytes());
                }
                union
            };
            match (seed >> 32) % 6 {
                0 => {
                    let index = seed as usize % inputs.len();
                    let mut expected = live(&current, &readers, &stages);
                    expected.extend(inputs[index].bytes());
                    let result = inputs[index].stage_world(&mut store);
                    assert_eq!(result.is_ok(), expected.values().sum::<usize>() <= budget);
                    if let Ok(staged) = result {
                        stages.push((staged, index));
                    }
                }
                1 if !stages.is_empty() => {
                    let (staged, index) = stages.swap_remove(seed as usize % stages.len());
                    let id = (seed >> 16) % 5;
                    let fits = current.contains_key(&id) || current.len() < 3;
                    assert_eq!(store.publish(vec![(id, staged)], &[]).is_ok(), fits);
                    if fits {
                        current.insert(id, index);
                    }
                }
                2 if !current.is_empty() => {
                    let id = *current.keys().nth(seed as usize % current.len()).unwrap();
                    store.publish(vec![], &[id]).unwrap();
                    current.remove(&id);
                }
                3 => readers.push((store.snapshot(), current.clone())),
                4 if !readers.is_empty() => {
                    readers.swap_remove(seed as usize % readers.len());
                }
                5 if !stages.is_empty() => {
                    stages.swap_remove(seed as usize % stages.len());
                }
                _ => {}
            }
            assert_eq!(
                store.live_bytes(),
                live(&current, &readers, &stages).values().sum()
            );
            assert!(store.live_bytes() <= budget);
            assert_eq!(
                store.snapshot().chunk_ids().collect::<Vec<_>>(),
                current.keys().copied().collect::<Vec<_>>()
            );
            for (snapshot, model) in &readers {
                for (chunk, index) in model {
                    let actual = snapshot.chunk(*chunk).unwrap();
                    assert_eq!(
                        actual.bundle_id(),
                        Some(content_id(&inputs[*index].manifest))
                    );
                    for (id, size) in inputs[*index].bytes() {
                        assert_eq!(actual.resource(&id).unwrap().len(), size);
                    }
                }
            }
        }
    }
}
