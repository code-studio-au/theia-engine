//! Protected CPU fallback ownership for a bounded outdoor world. No GPU fences,
//! I/O or geometric containment inference: the caller qualifies each asset pair.
use crate::{
    AssetError, Result,
    bundle_residency::{
        ChunkId, MAX_WORLD_CHUNKS, ResourceId, StagedBundle, WorldResidency, WorldSnapshot,
    },
    world_demand::{Catalog, ChunkBounds, Plan, Policy},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug)]
pub struct Region {
    /// The fine chunk ID also identifies the region for demand and draw selection.
    pub fine: ChunkBounds,
    pub fallback: ChunkId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub region: ChunkId,
    pub chunk: ChunkId,
}
/// Selection and all complete encoded owners from the same committed epoch.
/// Retaining this object prevents reuse of resources read by that selection.
#[derive(Clone)]
pub struct Snapshot {
    world: WorldSnapshot,
    selections: Vec<Selection>,
}
impl Snapshot {
    pub fn world(&self) -> &WorldSnapshot {
        &self.world
    }
    /// Ascending region ID; exactly one complete representation per region.
    pub fn selections(&self) -> &[Selection] {
        &self.selections
    }
}

/// Encapsulates the general world store after complete fallback bootstrap.
/// Protected chunk IDs cannot be replaced or removed through this API. Fine
/// publication remains explicit and atomic; pending reservations never select.
/// Completeness means admitted CPU graphs, not GPU readiness or geometric cover.
pub struct ProtectedWorld {
    world: WorldResidency,
    fallbacks: BTreeMap<ChunkId, ChunkId>,
    demand: Catalog,
}
impl ProtectedWorld {
    pub fn new(world: WorldResidency, regions: &[Region]) -> Result<Self> {
        if regions.is_empty() || regions.len() > MAX_WORLD_CHUNKS / 2 {
            return Err(AssetError("invalid protected region count"));
        }
        let mut known = BTreeSet::new();
        let mut fallbacks = BTreeMap::new();
        for region in regions {
            if !known.insert(region.fine.id) || !known.insert(region.fallback) {
                return Err(AssetError("overlapping protected region chunk IDs"));
            }
            fallbacks.insert(region.fine.id, region.fallback);
        }
        let current = world.snapshot();
        if fallbacks.values().any(|id| current.chunk(*id).is_none())
            || current.chunk_ids().any(|id| !known.contains(&id))
        {
            return Err(AssetError("missing fallback or unrelated published chunk"));
        }
        let demand = Catalog::new(&regions.iter().map(|r| r.fine).collect::<Vec<_>>())?;
        Ok(Self {
            world,
            fallbacks,
            demand,
        })
    }
    pub fn live_bytes(&self) -> usize {
        self.world.live_bytes()
    }
    pub fn snapshot(&self) -> Snapshot {
        let world = self.world.snapshot();
        let selections = self
            .fallbacks
            .iter()
            .map(|(&fine, &fallback)| Selection {
                region: fine,
                chunk: if world.chunk(fine).is_some() {
                    fine
                } else {
                    fallback
                },
            })
            .collect();
        Snapshot { world, selections }
    }
    /// Load/retain bands apply only to actual committed fine chunks. Coarse
    /// owners do not consume the fine policy's slots and never enter removals.
    /// Total world slots/bytes and CPU/GPU coexistence still gate publication.
    pub fn plan(&self, eye: [f32; 2], policy: Policy) -> Result<Plan> {
        let current = self.world.snapshot();
        let fine: Vec<_> = current
            .chunk_ids()
            .filter(|id| self.fallbacks.contains_key(id))
            .collect();
        self.demand.plan(eye, &fine, policy)
    }
    pub fn stage(
        &mut self,
        manifest: &[u8],
        expected: ResourceId,
        roots: [&[u8]; 4],
        leaves: &[&[u8]],
    ) -> Result<StagedBundle> {
        self.world.stage(manifest, expected, roots, leaves)
    }
    /// Caller retires any GPU users and builds a complete next GPU generation
    /// before mirroring its publication here. This API cannot infer those fences.
    pub fn publish_fine(
        &mut self,
        additions: Vec<(ChunkId, StagedBundle)>,
        removals: &[ChunkId],
    ) -> Result<Snapshot> {
        if additions
            .iter()
            .map(|(id, _)| id)
            .chain(removals)
            .any(|id| !self.fallbacks.contains_key(id))
        {
            return Err(AssetError("publication targets protected or unknown chunk"));
        }
        self.world.publish(additions, removals)?;
        Ok(self.snapshot())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_id;
    struct Input {
        manifest: Vec<u8>,
        roots: [Vec<u8>; 4],
    }
    impl Input {
        fn new(tag: u8) -> Self {
            let (_, mut roots) = crate::bundle::tests::fixture();
            let at = roots[3].len() - 16;
            roots[3][at] = tag;
            let mut manifest = b"THEIABND".to_vec();
            manifest.extend(1u32.to_le_bytes());
            manifest.extend(4u32.to_le_bytes());
            for root in &roots {
                manifest.extend(content_id(root));
                manifest.extend((root.len() as u64).to_le_bytes());
            }
            Self { manifest, roots }
        }
        fn stage(&self, world: &mut ProtectedWorld) -> StagedBundle {
            world
                .stage(
                    &self.manifest,
                    content_id(&self.manifest),
                    std::array::from_fn(|i| self.roots[i].as_slice()),
                    &[],
                )
                .unwrap()
        }
    }
    fn regions() -> [Region; 2] {
        [
            Region {
                fine: ChunkBounds {
                    id: 1,
                    min: [-2., -2.],
                    max: [0., 2.],
                },
                fallback: 10,
            },
            Region {
                fine: ChunkBounds {
                    id: 2,
                    min: [0., -2.],
                    max: [2., 2.],
                },
                fallback: 20,
            },
        ]
    }
    fn boot(capacity: usize) -> WorldResidency {
        boot_budget(capacity, 100_000)
    }
    fn boot_budget(capacity: usize, budget: usize) -> WorldResidency {
        let input = Input::new(10);
        let mut world = WorldResidency::new(budget, capacity).unwrap();
        let mut staged = Vec::new();
        for id in [10, 20] {
            staged.push((
                id,
                world
                    .stage(
                        &input.manifest,
                        content_id(&input.manifest),
                        std::array::from_fn(|i| input.roots[i].as_slice()),
                        &[],
                    )
                    .unwrap(),
            ));
        }
        world.publish(staged, &[]).unwrap();
        world
    }
    fn chosen(world: &Snapshot) -> Vec<ChunkId> {
        world.selections().iter().map(|s| s.chunk).collect()
    }
    #[test]
    fn bootstrap_rejects_missing_aliases_unknown_and_bad_bounds() {
        assert!(ProtectedWorld::new(boot(4), &[]).is_err());
        let mut r = regions();
        r[1].fallback = 10;
        assert!(ProtectedWorld::new(boot(4), &r).is_err());
        let mut r = regions();
        r[1].fallback = 1;
        assert!(ProtectedWorld::new(boot(4), &r).is_err());
        let mut r = regions();
        r[1].fallback = 99;
        assert!(ProtectedWorld::new(boot(4), &r).is_err());
        let mut r = regions();
        r[1].fine.min[0] = f32::NAN;
        assert!(ProtectedWorld::new(boot(4), &r).is_err());
        assert!(ProtectedWorld::new(boot(4), &regions()[..1]).is_err());
    }
    #[test]
    fn pending_cancel_publication_and_retained_selection_are_separate() {
        let mut world = ProtectedWorld::new(boot(4), &regions()).unwrap();
        let base = world.live_bytes();
        let before = world.snapshot();
        let input = Input::new(44);
        let pending = input.stage(&mut world);
        assert_eq!(chosen(&world.snapshot()), [10, 20]);
        assert!(world.live_bytes() > base);
        drop(pending);
        assert_eq!(world.live_bytes(), base);
        let pending = input.stage(&mut world);
        let fine = world.publish_fine(vec![(1, pending)], &[]).unwrap();
        assert_eq!(chosen(&fine), [1, 20]);
        assert_eq!(fine.world().epoch(), 2);
        let coarse = world.publish_fine(vec![], &[1]).unwrap();
        assert_eq!(chosen(&coarse), [10, 20]);
        assert_eq!(chosen(&before), [10, 20]);
        assert_eq!(chosen(&fine), [1, 20]);
        assert!(fine.world().chunk(1).is_some());
        assert!(world.live_bytes() > base);
        drop(fine);
        assert_eq!(world.live_bytes(), base);
    }
    #[test]
    fn protected_unknown_foreign_and_capacity_failures_preserve_coverage() {
        let mut world = ProtectedWorld::new(boot(3), &regions()).unwrap();
        let input = Input::new(45);
        for id in [10, 20, 99] {
            let pending = input.stage(&mut world);
            assert!(world.publish_fine(vec![(id, pending)], &[]).is_err());
            assert!(world.publish_fine(vec![], &[id]).is_err());
            assert_eq!(world.snapshot().world().epoch(), 1);
        }
        let pending = input.stage(&mut world);
        world.publish_fine(vec![(1, pending)], &[]).unwrap();
        let pending = input.stage(&mut world);
        assert!(world.publish_fine(vec![(2, pending)], &[]).is_err());
        assert_eq!(chosen(&world.snapshot()), [1, 20]);
        let pending = input.stage(&mut world);
        world.publish_fine(vec![(2, pending)], &[1]).unwrap();
        assert_eq!(chosen(&world.snapshot()), [10, 2]);
        let mut foreign = ProtectedWorld::new(boot(4), &regions()).unwrap();
        let pending = input.stage(&mut foreign);
        assert!(world.publish_fine(vec![(1, pending)], &[2]).is_err());
        assert_eq!(chosen(&world.snapshot()), [10, 2]);
    }
    #[test]
    fn demand_observes_committed_fine_only_and_never_evicts_fallbacks() {
        let mut world = ProtectedWorld::new(boot(4), &regions()).unwrap();
        let policy = Policy {
            load_radius: 0.,
            retain_radius: 4.,
            slots: 1,
        };
        let plan = world.plan([-1., 0.], policy).unwrap();
        assert_eq!(plan.loads, [1]);
        assert!(plan.removals.is_empty());
        // Planning or holding a reservation does not activate retention or select fine.
        let input = Input::new(46);
        let pending = input.stage(&mut world);
        assert!(world.plan([-3., 0.], policy).unwrap().desired.is_empty());
        world.publish_fine(vec![(1, pending)], &[]).unwrap();
        assert_eq!(world.plan([-3., 0.], policy).unwrap().desired, [1]);
        let plan = world.plan([1., 0.], policy).unwrap();
        assert_eq!(plan.loads, [2]);
        assert_eq!(plan.removals, [1]);
        world.publish_fine(vec![], &plan.removals).unwrap();
        assert_eq!(chosen(&world.snapshot()), [10, 20]);
    }

    #[test]
    fn coarse_only_byte_budget_defers_fine_without_changing_selection_or_epoch() {
        let bytes = boot(4).live_bytes();
        let mut world = ProtectedWorld::new(boot_budget(4, bytes), &regions()).unwrap();
        let input = Input::new(90);
        assert!(
            world
                .stage(
                    &input.manifest,
                    content_id(&input.manifest),
                    std::array::from_fn(|i| input.roots[i].as_slice()),
                    &[]
                )
                .is_err()
        );
        assert_eq!(chosen(&world.snapshot()), [10, 20]);
        assert_eq!(world.snapshot().world().epoch(), 1);
        assert_eq!(world.live_bytes(), bytes);
    }

    #[test]
    fn randomized_publications_keep_pinned_owners_and_retained_epoch_selections() {
        let mut world = ProtectedWorld::new(boot(4), &regions()).unwrap();
        let input = Input::new(55);
        let mut actual = BTreeSet::new();
        let mut held = std::collections::VecDeque::new();
        let mut epoch = 1;
        let mut seed = 0x37ac_8201u32;
        for step in 0..2000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let id = 1 + u64::from((seed >> 28) & 1);
            if step % 17 == 0 {
                let staged = input.stage(&mut world);
                assert!(world.publish_fine(vec![(10, staged)], &[]).is_err());
            } else if seed & 1 == 0 || !actual.contains(&id) {
                let staged = input.stage(&mut world);
                world.publish_fine(vec![(id, staged)], &[]).unwrap();
                actual.insert(id);
                epoch += 1;
            } else {
                world.publish_fine(vec![], &[id]).unwrap();
                actual.remove(&id);
                epoch += 1;
            }
            let expected = [
                if actual.contains(&1) { 1 } else { 10 },
                if actual.contains(&2) { 2 } else { 20 },
            ];
            let current = world.snapshot();
            assert_eq!(current.world().epoch(), epoch);
            assert_eq!(chosen(&current), expected);
            for id in [10, 20] {
                assert_eq!(current.world().chunk(id).unwrap().epoch(), 1);
            }
            held.push_back((current, expected, epoch));
            if held.len() > 8 {
                held.pop_front();
            }
            for (snapshot, expected, epoch) in &held {
                assert_eq!(chosen(snapshot), *expected);
                assert_eq!(snapshot.world().epoch(), *epoch);
                assert!(
                    snapshot
                        .selections()
                        .iter()
                        .all(|s| snapshot.world().chunk(s.chunk).is_some())
                );
            }
        }
    }
}
