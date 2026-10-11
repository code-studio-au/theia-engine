//! Bounded readonly client replica for complete snapshots, not gameplay authority.
//! Local connection contexts are not wire credentials or server authority epochs.
use std::sync::atomic::{AtomicU64, Ordering};
use theia_sim::ActorState;
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    owner: u64,
    connection: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub tick: u64,
    pub applied_move: Option<u64>,
    pub actors: Vec<ActorState>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimit,
    Exhausted,
    UnknownSource,
    ActorLimit,
    InvalidState,
    ConflictingTick,
    RegressingWatermark,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publication {
    New,
    Duplicate,
    Stale,
}
pub struct Replica {
    owner: u64,
    next_connection: u64,
    source: Option<Source>,
    max_actors: usize,
    latest: Option<Snapshot>,
    previous: Option<Snapshot>,
}
impl Replica {
    /// Prototype profile: sorted logical IDs, integer mm within +/-1M, health <=1M.
    /// The caller's decode/input allocation is outside this retained-state budget.
    pub fn new(max_actors: usize) -> Result<Self, Error> {
        if !(1..=128).contains(&max_actors) {
            return Err(Error::InvalidLimit);
        }
        let owner = NEXT_OWNER
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| Error::Exhausted)?;
        Ok(Self {
            owner,
            next_connection: 1,
            source: None,
            max_actors,
            latest: None,
            previous: None,
        })
    }
    /// Bind a new locally owned transport connection and clear its predecessor's state.
    pub fn connect(&mut self) -> Result<Source, Error> {
        let next = self
            .next_connection
            .checked_add(1)
            .ok_or(Error::Exhausted)?;
        let source = Source {
            owner: self.owner,
            connection: self.next_connection,
        };
        self.next_connection = next;
        self.source = Some(source);
        self.latest = None;
        self.previous = None;
        Ok(source)
    }
    pub fn disconnect(&mut self, source: Source) -> Result<(), Error> {
        if self.source != Some(source) {
            return Err(Error::UnknownSource);
        }
        self.source = None;
        self.latest = None;
        self.previous = None;
        Ok(())
    }
    /// Borrowed immutable presentation input. Changing a copied value never changes authority.
    pub fn latest(&self) -> Option<&Snapshot> {
        self.latest.as_ref()
    }
    pub fn previous(&self) -> Option<&Snapshot> {
        self.previous.as_ref()
    }
    /// Full-set publication is atomic after validation. Empty full sets remove every actor.
    /// Missing intermediate ticks need no baseline; no delta messages are accepted here.
    pub fn publish(
        &mut self,
        source: Source,
        mut snapshot: Snapshot,
    ) -> Result<Publication, Error> {
        if self.source != Some(source) {
            return Err(Error::UnknownSource);
        }
        if snapshot.actors.len() > self.max_actors {
            return Err(Error::ActorLimit);
        }
        if snapshot.actors.windows(2).any(|v| v[0].id >= v[1].id)
            || snapshot.actors.iter().any(|v| {
                v.position_mm
                    .iter()
                    .any(|p| !(-1_000_000..=1_000_000).contains(p))
                    || v.health > 1_000_000
            })
        {
            return Err(Error::InvalidState);
        }
        if let Some(current) = &self.latest {
            if snapshot.tick < current.tick {
                return Ok(Publication::Stale);
            }
            if snapshot.tick == current.tick {
                return if &snapshot == current {
                    Ok(Publication::Duplicate)
                } else {
                    Err(Error::ConflictingTick)
                };
            }
            if let Some(applied) = current.applied_move
                && snapshot.applied_move.is_none_or(|v| v < applied)
            {
                return Err(Error::RegressingWatermark);
            }
        }
        // Discard caller spare capacity before retaining a complete set. The input
        // allocation is caller-owned; retained columns have exactly their live count.
        snapshot.actors = snapshot.actors.into_boxed_slice().into_vec();
        self.previous = self.latest.replace(snapshot);
        Ok(Publication::New)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use theia_sim::WorldId;
    fn state(tick: u64) -> Snapshot {
        Snapshot {
            tick,
            applied_move: Some(tick),
            actors: vec![ActorState {
                id: WorldId::new(10).unwrap(),
                position_mm: [tick as i32, 0],
                health: 100,
            }],
        }
    }
    #[test]
    fn newest_complete_set_duplicate_conflict_and_watermark_are_atomic() {
        let mut r = Replica::new(2).unwrap();
        let s = r.connect().unwrap();
        r.publish(s, state(3)).unwrap();
        assert_eq!(r.publish(s, state(1)), Ok(Publication::Stale));
        assert_eq!(r.publish(s, state(3)), Ok(Publication::Duplicate));
        let mut conflict = state(3);
        conflict.actors[0].health = 0;
        assert_eq!(r.publish(s, conflict), Err(Error::ConflictingTick));
        for ack in [None, Some(2)] {
            assert_eq!(
                r.publish(
                    s,
                    Snapshot {
                        applied_move: ack,
                        ..state(4)
                    }
                ),
                Err(Error::RegressingWatermark)
            );
            assert_eq!(r.latest(), Some(&state(3)));
        }
        assert_eq!(
            r.publish(
                s,
                Snapshot {
                    actors: vec![],
                    ..state(8)
                }
            ),
            Ok(Publication::New)
        );
        assert!(r.latest().unwrap().actors.is_empty());
    }
    #[test]
    fn reconnect_and_foreign_contexts_never_republish_old_state() {
        let mut r = Replica::new(2).unwrap();
        let old = r.connect().unwrap();
        r.publish(old, state(30)).unwrap();
        let fresh = r.connect().unwrap();
        assert!(r.latest().is_none());
        assert_eq!(r.publish(old, state(31)), Err(Error::UnknownSource));
        assert_eq!(r.disconnect(old), Err(Error::UnknownSource));
        let mut other = Replica::new(2).unwrap();
        let foreign = other.connect().unwrap();
        assert_eq!(r.publish(foreign, state(99)), Err(Error::UnknownSource));
        r.publish(fresh, state(1)).unwrap();
        r.disconnect(fresh).unwrap();
        assert!(r.latest().is_none());
        assert_eq!(r.publish(fresh, state(2)), Err(Error::UnknownSource));
    }
    #[test]
    fn budgets_invalid_values_and_exhaustion_preserve_state() {
        assert!(matches!(Replica::new(0), Err(Error::InvalidLimit)));
        assert!(matches!(Replica::new(129), Err(Error::InvalidLimit)));
        let mut r = Replica::new(1).unwrap();
        let source = r.connect().unwrap();
        r.publish(source, state(1)).unwrap();
        let mut large = state(2);
        large.actors.push(large.actors[0]);
        assert_eq!(r.publish(source, large), Err(Error::ActorLimit));
        let mut bad = state(2);
        bad.actors[0].position_mm = [i32::MIN, 0];
        assert_eq!(r.publish(source, bad), Err(Error::InvalidState));
        r.next_connection = u64::MAX;
        assert_eq!(r.connect(), Err(Error::Exhausted));
        assert_eq!(r.latest(), Some(&state(1)));
        r.publish(
            source,
            Snapshot {
                tick: u64::MAX,
                ..state(2)
            },
        )
        .unwrap();
        assert_eq!(r.publish(source, state(3)), Ok(Publication::Stale));
    }
    #[test]
    fn canonical_ids_health_and_copied_values_cannot_mutate_published_state() {
        let mut r = Replica::new(2).unwrap();
        let source = r.connect().unwrap();
        r.publish(source, state(1)).unwrap();
        let mut copy = r.latest().unwrap().clone();
        copy.actors[0].health = 0;
        assert_eq!(r.latest(), Some(&state(1)));
        let mut duplicate = state(2);
        duplicate.actors.push(duplicate.actors[0]);
        assert_eq!(r.publish(source, duplicate), Err(Error::InvalidState));
        let mut health = state(2);
        health.actors[0].health = 1_000_001;
        assert_eq!(r.publish(source, health), Err(Error::InvalidState));
        assert_eq!(r.latest(), Some(&state(1)));
        let mut replacement = state(5);
        replacement.actors[0].id = WorldId::new(99).unwrap();
        r.publish(source, replacement.clone()).unwrap();
        assert_eq!(r.latest(), Some(&replacement));
        let mut spare = state(6);
        spare.actors.reserve(10000);
        r.publish(source, spare).unwrap();
        assert_eq!(r.latest().unwrap().actors.capacity(), 1);
    }
    #[test]
    fn randomized_loss_reordering_and_duplicates_match_maximum_tick_oracle() {
        let mut r = Replica::new(2).unwrap();
        let source = r.connect().unwrap();
        let mut seed = 719u64;
        let mut pending = Vec::new();
        let mut expected = None;
        for tick in 0..10000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            if !seed.is_multiple_of(5) {
                pending.push(tick);
            }
            if pending.len() >= 16 || seed.is_multiple_of(3) {
                let index = ((seed >> 32) as usize) % pending.len().max(1);
                if !pending.is_empty() {
                    let delivered = pending.swap_remove(index);
                    expected = Some(expected.map_or(delivered, |v: u64| v.max(delivered)));
                    r.publish(source, state(delivered)).unwrap();
                    r.publish(source, state(delivered)).unwrap();
                }
            }
            assert_eq!(r.latest().map(|v| v.tick), expected);
            if let Some(tick) = expected {
                assert_eq!(r.latest(), Some(&state(tick)));
            }
        }
        // A new full snapshot repairs all missing/out-of-order intermediate states.
        r.publish(source, state(10000)).unwrap();
        assert_eq!(r.latest(), Some(&state(10000)));
    }
}
