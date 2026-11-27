//! Bounded transport-neutral full-actor delta prototype. Not a wire schema or
//! baseline ACK policy. Exact named complete baselines are required; no guessing.
use crate::replica::{Error, Snapshot, validate_actors};
use theia_sim::{ActorState, WorldId};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delta {
    pub baseline_tick: u64,
    pub tick: u64,
    pub applied_move: Option<u64>,
    /// Complete replacement values for changed/new logical actors, sorted unique.
    pub upserts: Vec<ActorState>,
    /// Existing baseline IDs to remove, sorted unique, disjoint from upserts.
    pub removed: Vec<WorldId>,
}
impl Delta {
    pub fn between(
        baseline: &Snapshot,
        current: &Snapshot,
        max_actors: usize,
    ) -> Result<Self, Error> {
        validate_actors(&baseline.actors, max_actors)?;
        validate_actors(&current.actors, max_actors)?;
        if current.tick <= baseline.tick {
            return Err(Error::InvalidState);
        }
        check_watermark(baseline.applied_move, current.applied_move)?;
        let upserts = current
            .actors
            .iter()
            .filter(|actor| {
                baseline
                    .actors
                    .binary_search_by_key(&actor.id, |a| a.id)
                    .map_or(true, |i| baseline.actors[i] != **actor)
            })
            .copied()
            .collect();
        let removed = baseline
            .actors
            .iter()
            .filter(|actor| {
                current
                    .actors
                    .binary_search_by_key(&actor.id, |a| a.id)
                    .is_err()
            })
            .map(|a| a.id)
            .collect();
        Ok(Self {
            baseline_tick: baseline.tick,
            tick: current.tick,
            applied_move: current.applied_move,
            upserts,
            removed,
        })
    }
    /// Validate the bounded envelope; removal membership needs reconstruction.
    pub fn validate(&self, max_actors: usize) -> Result<(), Error> {
        validate_actors(&self.upserts, max_actors)?;
        if self.removed.len() > max_actors {
            return Err(Error::ActorLimit);
        }
        if self.tick <= self.baseline_tick
            || self.removed.windows(2).any(|v| v[0] >= v[1])
            || self
                .removed
                .iter()
                .any(|id| self.upserts.binary_search_by_key(id, |a| a.id).is_ok())
        {
            return Err(Error::InvalidState);
        }
        Ok(())
    }
    pub fn reconstruct(&self, baseline: &Snapshot, max_actors: usize) -> Result<Snapshot, Error> {
        self.validate(max_actors)?;
        if baseline.tick != self.baseline_tick {
            return Err(Error::MissingBaseline);
        }
        validate_actors(&baseline.actors, max_actors)?;
        check_watermark(baseline.applied_move, self.applied_move)?;
        if self
            .removed
            .iter()
            .any(|id| baseline.actors.binary_search_by_key(id, |a| a.id).is_err())
        {
            return Err(Error::InvalidState);
        }
        let mut actors: Vec<_> = baseline
            .actors
            .iter()
            .filter(|actor| self.removed.binary_search(&actor.id).is_err())
            .copied()
            .collect();
        for &actor in &self.upserts {
            match actors.binary_search_by_key(&actor.id, |a| a.id) {
                Ok(i) => actors[i] = actor,
                Err(i) => {
                    if actors.len() == max_actors {
                        return Err(Error::ActorLimit);
                    }
                    actors.insert(i, actor);
                }
            }
        }
        Ok(Snapshot {
            tick: self.tick,
            applied_move: self.applied_move,
            actors,
        })
    }
}
fn check_watermark(previous: Option<u64>, current: Option<u64>) -> Result<(), Error> {
    if previous.is_some_and(|v| current.is_none_or(|c| c < v)) {
        Err(Error::RegressingWatermark)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replica::{Publication, Replica};
    fn actor(id: u64, x: i32, health: u32) -> ActorState {
        ActorState {
            id: WorldId::new(id).unwrap(),
            position_mm: [x, -x],
            health,
        }
    }
    fn state(tick: u64, actors: Vec<ActorState>) -> Snapshot {
        Snapshot {
            tick,
            applied_move: Some(tick),
            actors,
        }
    }
    #[test]
    fn complete_replacement_lifecycle_and_current_health_reconstruct_exactly() {
        let base = state(
            1,
            vec![actor(1, 10, 100), actor(2, 20, 100), actor(3, 30, 0)],
        );
        let next = state(
            2,
            vec![actor(1, 11, 75), actor(3, 30, 100), actor(4, 40, 100)],
        );
        let delta = Delta::between(&base, &next, 3).unwrap();
        assert_eq!(delta.removed, [WorldId::new(2).unwrap()]);
        assert_eq!(delta.upserts, next.actors);
        assert_eq!(delta.reconstruct(&base, 3), Ok(next.clone()));
        let unchanged = state(3, next.actors.clone());
        let delta = Delta::between(&next, &unchanged, 3).unwrap();
        assert!(delta.upserts.is_empty() && delta.removed.is_empty());
        assert_eq!(delta.reconstruct(&next, 3), Ok(unchanged));
        let empty = state(4, vec![]);
        let delta = Delta::between(&next, &empty, 3).unwrap();
        assert_eq!(delta.reconstruct(&next, 3), Ok(empty));
    }
    #[test]
    fn missing_evicted_or_foreign_baseline_preserves_state_until_full_repair() {
        let mut replica = Replica::new(2).unwrap();
        let source = replica.connect().unwrap();
        let first = state(1, vec![actor(1, 10, 100)]);
        let second = state(2, vec![actor(1, 20, 75)]);
        let third = state(3, vec![actor(2, 30, 100)]);
        let d2 = Delta::between(&first, &second, 2).unwrap();
        assert_eq!(
            replica.publish_delta(source, &d2),
            Err(Error::MissingBaseline)
        );
        assert!(replica.latest().is_none());
        replica.publish(source, first.clone()).unwrap();
        assert_eq!(replica.publish_delta(source, &d2), Ok(Publication::New));
        assert_eq!(
            replica.publish_delta(source, &d2),
            Ok(Publication::Duplicate)
        );
        let d3 = Delta::between(&first, &third, 2).unwrap();
        assert_eq!(replica.publish_delta(source, &d3), Ok(Publication::New));
        assert_eq!(replica.latest(), Some(&third));
        let fourth = state(4, vec![]);
        let d4 = Delta::between(&first, &fourth, 2).unwrap();
        assert_eq!(
            replica.publish_delta(source, &d4),
            Err(Error::MissingBaseline)
        );
        assert_eq!(replica.latest(), Some(&third));
        assert_eq!(replica.previous(), Some(&second));
        replica.publish(source, fourth.clone()).unwrap();
        assert_eq!(replica.latest(), Some(&fourth));
        assert_eq!(replica.publish_delta(source, &d2), Ok(Publication::Stale));
        let fresh = replica.connect().unwrap();
        assert_eq!(
            replica.publish_delta(source, &d4),
            Err(Error::UnknownSource)
        );
        assert_eq!(
            replica.publish_delta(fresh, &d4),
            Err(Error::MissingBaseline)
        );
        assert!(replica.latest().is_none());
    }
    #[test]
    fn malformed_conflicting_and_regressing_deltas_never_partially_publish() {
        let base = state(10, vec![actor(1, 0, 100), actor(2, 100, 75)]);
        let next = state(11, vec![actor(1, 10, 75), actor(3, 200, 100)]);
        let good = Delta::between(&base, &next, 2).unwrap();
        let mut replica = Replica::new(2).unwrap();
        let source = replica.connect().unwrap();
        replica.publish(source, base.clone()).unwrap();
        let mut cases = vec![];
        let mut d = good.clone();
        d.upserts.push(d.upserts[0]);
        cases.push(d);
        let mut d = good.clone();
        d.removed.push(d.removed[0]);
        cases.push(d);
        let mut d = good.clone();
        d.removed = vec![WorldId::new(1).unwrap()];
        cases.push(d);
        let mut d = good.clone();
        d.removed = vec![WorldId::new(4).unwrap()];
        cases.push(d);
        let mut d = good.clone();
        d.upserts[0].health = 1_000_001;
        cases.push(d);
        let mut d = good.clone();
        d.upserts[0].position_mm[0] = -1_000_001;
        cases.push(d);
        let mut d = good.clone();
        d.removed.clear();
        cases.push(d); // Union exceeds capacity.
        let mut d = good.clone();
        d.applied_move = None;
        cases.push(d);
        let mut d = good.clone();
        d.tick = d.baseline_tick;
        cases.push(d);
        for delta in cases {
            assert!(replica.publish_delta(source, &delta).is_err());
            assert_eq!(replica.latest(), Some(&base));
            assert!(replica.previous().is_none());
        }
        replica.publish_delta(source, &good).unwrap();
        let mut conflict = good.clone();
        conflict.upserts[0].health = 50;
        assert_eq!(
            replica.publish_delta(source, &conflict),
            Err(Error::ConflictingTick)
        );
        assert_eq!(replica.latest(), Some(&next));
        // A delta based on an older retained set still cannot regress CURRENT ACK.
        let mut regress = good;
        regress.tick = 12;
        regress.applied_move = Some(10);
        assert_eq!(
            replica.publish_delta(source, &regress),
            Err(Error::RegressingWatermark)
        );
        assert_eq!(replica.latest(), Some(&next));
    }
    #[test]
    fn independent_sparse_membership_oracle_and_extreme_ticks_round_trip() {
        use std::collections::BTreeMap;
        let mut values = BTreeMap::new();
        let mut base = Snapshot {
            tick: u64::MAX - 3001,
            applied_move: None,
            actors: vec![],
        };
        let mut seed = 0x1a25_8891u64;
        for step in 0..3000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let id = seed % 16 + 1;
            if seed & 32 == 0 {
                values.remove(&id);
            } else {
                values.insert(id, actor(id, step, (seed % 101) as u32));
            }
            let next = Snapshot {
                tick: base.tick + 1,
                applied_move: Some(step as u64),
                actors: values.values().copied().collect(),
            };
            let delta = Delta::between(&base, &next, 16).unwrap();
            assert_eq!(delta.reconstruct(&base, 16), Ok(next.clone()));
            assert!(delta.upserts.len() <= 1 && delta.removed.len() <= 1);
            base = next;
        }
        let last = Snapshot {
            tick: u64::MAX,
            ..base.clone()
        };
        assert_eq!(
            Delta::between(&base, &last, 16)
                .unwrap()
                .reconstruct(&base, 16),
            Ok(last)
        );
    }
}
