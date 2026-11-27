//! Readonly bounded positional interpolation. Explicit screen time never advances
//! authority. No extrapolation/prediction/network clock estimator is implemented.
use crate::replica::Replica;
use theia_sim::WorldId;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Time {
    tick: u64,
    sub_tick: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidTime,
    InvalidGap,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl Time {
    /// Fraction is finite in [0,1). Keep the integer tick separate to preserve
    /// adjacent-frame differences even when the server tick exceeds 2^53.
    pub fn new(tick: u64, sub_tick: f64) -> Result<Self, Error> {
        if !sub_tick.is_finite() || !(0.0..1.0).contains(&sub_tick) {
            return Err(Error::InvalidTime);
        }
        Ok(Self { tick, sub_tick })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Single,
    Before,
    Between,
    After,
    Gap,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Actor {
    pub id: WorldId,
    pub position_mm: [f64; 2],
    pub health: u32,
}
pub struct Frame {
    pub from_tick: u64,
    pub to_tick: u64,
    pub alpha: f64,
    pub mode: Mode,
    pub actors: Vec<Actor>,
}
impl Replica {
    /// Clamp to retained endpoints; a gap beyond the prototype limit snaps to
    /// latest. Membership/health are latest truth; only matching actor positions
    /// blend. New actors and observed dead->alive transitions snap to current.
    pub fn present(&self, time: Time, max_gap_ticks: u64) -> Result<Option<Frame>, Error> {
        if !(1..=120).contains(&max_gap_ticks) {
            return Err(Error::InvalidGap);
        }
        let Some(current) = self.latest() else {
            return Ok(None);
        };
        let previous = self.previous();
        let (from_tick, alpha, mode) = match previous {
            None => (current.tick, 1.0, Mode::Single),
            Some(previous) => {
                let gap = current.tick - previous.tick;
                let (alpha, mode) = if gap > max_gap_ticks {
                    (1.0, Mode::Gap)
                } else if time.tick < previous.tick {
                    (0.0, Mode::Before)
                } else if time.tick >= current.tick {
                    (1.0, Mode::After)
                } else {
                    (
                        ((time.tick - previous.tick) as f64 + time.sub_tick) / gap as f64,
                        Mode::Between,
                    )
                };
                (previous.tick, alpha, mode)
            }
        };
        let actors = current
            .actors
            .iter()
            .map(|actor| {
                let mut position_mm = actor.position_mm.map(f64::from);
                if alpha < 1.0
                    && let Some(previous) = previous
                    && let Ok(index) = previous.actors.binary_search_by_key(&actor.id, |v| v.id)
                {
                    let old = previous.actors[index];
                    if old.health != 0 || actor.health == 0 {
                        position_mm = std::array::from_fn(|i| {
                            f64::from(old.position_mm[i])
                                + alpha
                                    * (f64::from(actor.position_mm[i])
                                        - f64::from(old.position_mm[i]))
                        });
                    }
                }
                Actor {
                    id: actor.id,
                    position_mm,
                    health: actor.health,
                }
            })
            .collect();
        Ok(Some(Frame {
            from_tick,
            to_tick: current.tick,
            alpha,
            mode,
            actors,
        }))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::replica::Snapshot;
    use theia_sim::ActorState;
    fn actor(id: u64, x: i32, health: u32) -> ActorState {
        ActorState {
            id: WorldId::new(id).unwrap(),
            position_mm: [x, -x],
            health,
        }
    }
    fn snapshot(tick: u64, actors: Vec<ActorState>) -> Snapshot {
        Snapshot {
            tick,
            actors,
            applied_move: None,
        }
    }
    #[test]
    fn endpoints_midpoints_and_separate_clock_are_bounded() {
        let mut r = Replica::new(2).unwrap();
        let source = r.connect().unwrap();
        assert!(r.present(Time::new(0, 0.0).unwrap(), 4).unwrap().is_none());
        r.publish(source, snapshot(10, vec![actor(1, 100, 100)]))
            .unwrap();
        assert_eq!(
            r.present(Time::new(0, 0.0).unwrap(), 4)
                .unwrap()
                .unwrap()
                .mode,
            Mode::Single
        );
        r.publish(source, snapshot(12, vec![actor(1, 300, 75)]))
            .unwrap();
        for (tick, fraction, x, mode) in [
            (9, 0.5, 100.0, Mode::Before),
            (10, 0.0, 100.0, Mode::Between),
            (11, 0.5, 250.0, Mode::Between),
            (12, 0.0, 300.0, Mode::After),
            (99, 0.0, 300.0, Mode::After),
        ] {
            let frame = r
                .present(Time::new(tick, fraction).unwrap(), 4)
                .unwrap()
                .unwrap();
            assert_eq!(frame.mode, mode);
            assert_eq!(frame.actors[0].position_mm, [x, -x]);
            assert_eq!(frame.actors[0].health, 75);
        }
        assert_eq!(r.latest().unwrap().actors[0].position_mm, [300, -300]);
    }
    #[test]
    fn identity_membership_respawn_gap_and_reset_cannot_blend_unrelated_state() {
        let mut r = Replica::new(3).unwrap();
        let source = r.connect().unwrap();
        r.publish(source, snapshot(1, vec![actor(1, 0, 0), actor(2, 0, 100)]))
            .unwrap();
        r.publish(
            source,
            snapshot(2, vec![actor(1, 1000, 100), actor(3, 500, 100)]),
        )
        .unwrap();
        let frame = r.present(Time::new(1, 0.5).unwrap(), 4).unwrap().unwrap();
        assert_eq!(frame.actors.len(), 2);
        assert_eq!(frame.actors[0].position_mm, [1000.0, -1000.0]);
        assert_eq!(frame.actors[1].id, WorldId::new(3).unwrap());
        assert_eq!(frame.actors[1].position_mm, [500.0, -500.0]);
        r.publish(source, snapshot(100, vec![actor(1, 2000, 75)]))
            .unwrap();
        let frame = r.present(Time::new(2, 0.5).unwrap(), 4).unwrap().unwrap();
        assert_eq!(frame.mode, Mode::Gap);
        assert_eq!(frame.alpha, 1.0);
        r.connect().unwrap();
        assert!(
            r.present(Time::new(100, 0.0).unwrap(), 4)
                .unwrap()
                .is_none()
        );
        assert!(r.previous().is_none());
    }
    #[test]
    fn huge_ticks_and_invalid_policy_do_not_lose_local_time_precision() {
        for fraction in [f64::NAN, f64::INFINITY, -0.1, 1.0] {
            assert_eq!(Time::new(1, fraction), Err(Error::InvalidTime));
        }
        let mut r = Replica::new(1).unwrap();
        let source = r.connect().unwrap();
        r.publish(
            source,
            snapshot(u64::MAX - 2, vec![actor(1, -1_000_000, 100)]),
        )
        .unwrap();
        r.publish(source, snapshot(u64::MAX, vec![actor(1, 1_000_000, 100)]))
            .unwrap();
        let time = Time::new(u64::MAX - 1, 0.5).unwrap();
        let frame = r.present(time, 4).unwrap().unwrap();
        assert_eq!(frame.alpha, 0.75);
        assert_eq!(frame.actors[0].position_mm, [500_000.0, -500_000.0]);
        for gap in [0, 121] {
            assert!(matches!(r.present(time, gap), Err(Error::InvalidGap)));
        }
    }
    #[test]
    fn randomized_samples_match_independent_convex_weights_and_latest_health() {
        let mut r = Replica::new(1).unwrap();
        let source = r.connect().unwrap();
        let mut seed = 7919u64;
        for tick in 1..=3000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let start = ((seed >> 32) % 2_000_001) as i32 - 1_000_000;
            let end = (seed % 2_000_001) as i32 - 1_000_000;
            r.publish(source, snapshot(tick * 2, vec![actor(1, start, 100)]))
                .unwrap();
            r.publish(source, snapshot(tick * 2 + 1, vec![actor(1, end, 75)]))
                .unwrap();
            let fraction = (seed % 1024) as f64 / 1024.0;
            let frame = r
                .present(Time::new(tick * 2, fraction).unwrap(), 4)
                .unwrap()
                .unwrap();
            let expected = (1.0 - fraction) * f64::from(start) + fraction * f64::from(end);
            assert!((frame.actors[0].position_mm[0] - expected).abs() < 1e-8);
            assert!(
                frame.actors[0]
                    .position_mm
                    .iter()
                    .all(|v| v.abs() <= 1_000_000.0)
            );
            assert_eq!(frame.actors[0].health, 75);
        }
    }
}
