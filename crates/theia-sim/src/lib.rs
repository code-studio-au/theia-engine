//! Bounded single-writer authoritative movement/combat slice. No networking or GPU truth.
//! Positions are integer millimetres on an analytic Z=0 floor. These prototype
//! precision/tick/rules are not approved global engine or wire-format constants.
mod collision;
mod combat;
pub use collision::{Rect, Scene};
pub use combat::{AttackInput, AttackOutcome, AttackReject, AttackResult, HealthChange};
use std::{
    collections::BTreeMap,
    error, fmt,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_WORLD: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidScene,
    InvalidRules,
    InvalidWorldId,
    DuplicateActor,
    Capacity,
    IllegalSpawn,
    UnknownActor,
    UnknownSession,
    DeadActor,
    Sequence,
    InvalidIntent,
    InputBudget,
    CounterExhausted,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldId(u64);
impl WorldId {
    /// Supplied by the server's logical identity registry, never a dense row/GPU handle.
    pub fn new(value: u64) -> Result<Self> {
        if value == 0 {
            Err(Error::InvalidWorldId)
        } else {
            Ok(Self(value))
        }
    }
    pub fn value(self) -> u64 {
        self.0
    }
}
/// Trusted server connection context, not a credential or client packet field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Session {
    world: u64,
    id: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    pub tick_ms: u32,
    pub speed_mm_per_second: u32,
    pub half_extent_mm: u32,
    pub intent_hold_ticks: u64,
    pub max_actors: usize,
    pub max_inputs: usize,
    pub max_inputs_per_session: usize,
    pub max_health: u32,
    pub attack_damage: u32,
    pub attack_range_mm: u32,
    pub attack_cooldown_ticks: u64,
}
impl Default for Rules {
    fn default() -> Self {
        Self {
            tick_ms: 50,
            speed_mm_per_second: 3000,
            half_extent_mm: 250,
            intent_hold_ticks: 3,
            max_actors: 128,
            max_inputs: 256,
            max_inputs_per_session: 4,
            max_health: 100,
            attack_damage: 25,
            attack_range_mm: 1500,
            attack_cooldown_ticks: 4,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct MoveInput {
    pub sequence: u64,
    pub axes: [i16; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorState {
    pub id: WorldId,
    pub position_mm: [i32; 2],
    pub health: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    pub id: WorldId,
    pub position_mm: [i32; 2],
}
#[derive(Clone, Copy, Debug)]
/// Authoritative processing watermark; a blocked/rejected action is not a successful effect.
pub struct Applied {
    pub id: WorldId,
    pub sequence: u64,
}
pub struct Frame {
    pub tick: u64,
    pub changes: Vec<Change>,
    pub applied: Vec<Applied>,
    pub blocked: Vec<WorldId>,
    pub attacks: Vec<AttackOutcome>,
    pub health_changes: Vec<HealthChange>,
}
#[derive(Clone, Copy)]
enum Action {
    Move([i16; 2]),
    Attack(WorldId),
}
struct Queued {
    session: Session,
    sequence: u64,
    action: Action,
}
struct Client {
    actor: WorldId,
    last_sequence: Option<u64>,
    queued: usize,
}
pub struct World {
    owner: u64,
    tick: u64,
    next_session: u64,
    rules: Rules,
    step_mm: i32,
    collision: collision::Collision,
    // Dense authoritative SoA. Map/lifecycle ownership is host-side; row placement is not identity.
    ids: Vec<WorldId>,
    positions: Vec<[i32; 2]>,
    intents: Vec<[i16; 2]>,
    expires: Vec<u64>,
    controllers: Vec<Session>,
    health: Vec<u32>,
    next_attack: Vec<u64>,
    rows: BTreeMap<WorldId, usize>,
    clients: BTreeMap<Session, Client>,
    pending: Vec<Queued>,
}
impl World {
    pub fn new(scene: Scene, rules: Rules) -> Result<Self> {
        let step = u64::from(rules.speed_mm_per_second) * u64::from(rules.tick_ms) / 1000;
        if !(1..=1000).contains(&rules.tick_ms)
            || !(1..=100_000).contains(&rules.speed_mm_per_second)
            || !(1..=10000).contains(&rules.half_extent_mm)
            || !(1..=60).contains(&rules.intent_hold_ticks)
            || !(1..=4096).contains(&rules.max_actors)
            || !(1..=4096).contains(&rules.max_inputs)
            || !(1..=16).contains(&rules.max_inputs_per_session)
            || !(1..=1_000_000).contains(&rules.max_health)
            || !(1..=1_000_000).contains(&rules.attack_damage)
            || !(1..=100_000).contains(&rules.attack_range_mm)
            || !(1..=600).contains(&rules.attack_cooldown_ticks)
            || step == 0
        {
            return Err(Error::InvalidRules);
        }
        let collision = scene.prepare(rules.half_extent_mm as i32)?;
        let owner = NEXT_WORLD
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| Error::CounterExhausted)?;
        Ok(Self {
            owner,
            tick: 0,
            next_session: 1,
            rules,
            step_mm: step as i32,
            collision,
            ids: Vec::with_capacity(rules.max_actors),
            positions: Vec::with_capacity(rules.max_actors),
            intents: Vec::with_capacity(rules.max_actors),
            expires: Vec::with_capacity(rules.max_actors),
            controllers: Vec::with_capacity(rules.max_actors),
            health: Vec::with_capacity(rules.max_actors),
            next_attack: Vec::with_capacity(rules.max_actors),
            rows: BTreeMap::new(),
            clients: BTreeMap::new(),
            pending: Vec::with_capacity(rules.max_inputs),
        })
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn actor(&self, id: WorldId) -> Option<ActorState> {
        self.rows.get(&id).map(|&row| ActorState {
            id,
            position_mm: self.positions[row],
            health: self.health[row],
        })
    }
    pub fn snapshot(&self) -> Vec<ActorState> {
        self.ids
            .iter()
            .enumerate()
            .map(|(i, &id)| ActorState {
                id,
                position_mm: self.positions[i],
                health: self.health[i],
            })
            .collect()
    }
    /// Administrative lifecycle operation at a tick boundary, not a client action.
    pub fn spawn_at_boundary(&mut self, id: WorldId, position_mm: [i32; 2]) -> Result<Session> {
        if self.rows.contains_key(&id) {
            return Err(Error::DuplicateActor);
        }
        if self.ids.len() == self.rules.max_actors {
            return Err(Error::Capacity);
        }
        if !self.collision.legal(position_mm) {
            return Err(Error::IllegalSpawn);
        }
        let next = self
            .next_session
            .checked_add(1)
            .ok_or(Error::CounterExhausted)?;
        let session = Session {
            world: self.owner,
            id: self.next_session,
        };
        self.next_session = next;
        self.rows.insert(id, self.ids.len());
        self.ids.push(id);
        self.positions.push(position_mm);
        self.intents.push([0; 2]);
        self.expires.push(0);
        self.controllers.push(session);
        self.health.push(self.rules.max_health);
        self.next_attack.push(0);
        self.clients.insert(
            session,
            Client {
                actor: id,
                last_sequence: None,
                queued: 0,
            },
        );
        Ok(session)
    }
    pub fn despawn_at_boundary(&mut self, id: WorldId) -> Result<()> {
        let row = self.rows.remove(&id).ok_or(Error::UnknownActor)?;
        let session = self.controllers[row];
        self.clients.remove(&session);
        self.pending.retain(|input| input.session != session);
        self.ids.swap_remove(row);
        self.positions.swap_remove(row);
        self.intents.swap_remove(row);
        self.expires.swap_remove(row);
        self.controllers.swap_remove(row);
        self.health.swap_remove(row);
        self.next_attack.swap_remove(row);
        if row < self.ids.len() {
            self.rows.insert(self.ids[row], row);
        }
        Ok(())
    }
    /// Bind session from trusted server transport context. Client submits direction/sequence only.
    /// Admission is queued, not an acknowledgement that movement has been applied.
    pub fn submit(&mut self, session: Session, input: MoveInput) -> Result<()> {
        self.admit(session, input.sequence, Action::Move(input.axes))
    }
    pub fn submit_attack(&mut self, session: Session, input: AttackInput) -> Result<()> {
        self.admit(session, input.sequence, Action::Attack(input.target))
    }
    fn admit(&mut self, session: Session, sequence: u64, action: Action) -> Result<()> {
        let client = self
            .clients
            .get_mut(&session)
            .ok_or(Error::UnknownSession)?;
        if self.health[self.rows[&client.actor]] == 0 {
            return Err(Error::DeadActor);
        }
        if let Action::Move(axes) = action
            && axes.iter().any(|v| !(-1000..=1000).contains(v))
        {
            return Err(Error::InvalidIntent);
        }
        if client.last_sequence.is_some_and(|v| sequence <= v) {
            return Err(Error::Sequence);
        }
        if client.queued >= self.rules.max_inputs_per_session
            || self.pending.len() >= self.rules.max_inputs
        {
            return Err(Error::InputBudget);
        }
        client.last_sequence = Some(sequence);
        client.queued += 1;
        self.pending.push(Queued {
            session,
            sequence,
            action,
        });
        Ok(())
    }
    /// Fixed simulation step, independent of client time/render/network cadence.
    /// Every actor moves at most once, regardless of inputs admitted this tick.
    pub fn step(&mut self) -> Result<Frame> {
        let tick = self.tick.checked_add(1).ok_or(Error::CounterExhausted)?;
        let expiry = if !self
            .pending
            .iter()
            .any(|v| matches!(v.action, Action::Move(_)))
        {
            0
        } else {
            tick.checked_add(self.rules.intent_hold_ticks - 1)
                .ok_or(Error::CounterExhausted)?
        };
        let ready_tick = if self
            .pending
            .iter()
            .any(|v| matches!(v.action, Action::Attack(_)))
        {
            tick.checked_add(self.rules.attack_cooldown_ticks)
                .ok_or(Error::CounterExhausted)?
        } else {
            0
        };
        let mut applied = Vec::with_capacity(self.pending.len());
        let mut requests = Vec::with_capacity(self.pending.len());
        for input in self.pending.drain(..) {
            let client = self
                .clients
                .get_mut(&input.session)
                .expect("lifecycle cancels stale queued inputs");
            let row = self.rows[&client.actor];
            client.queued = 0;
            match input.action {
                Action::Move(axes) => {
                    self.intents[row] = axes;
                    self.expires[row] = expiry;
                }
                Action::Attack(target) => requests.push((client.actor, target, input.sequence)),
            }
            applied.push(Applied {
                id: client.actor,
                sequence: input.sequence,
            });
        }
        let mut changes = Vec::with_capacity(self.ids.len());
        let mut blocked = Vec::with_capacity(self.ids.len());
        for row in 0..self.ids.len() {
            if self.health[row] == 0 {
                continue;
            }
            if tick > self.expires[row] {
                self.intents[row] = [0; 2];
            }
            let delta = movement(self.intents[row], self.step_mm);
            let start = self.positions[row];
            let end = std::array::from_fn(|i| start[i] + delta[i]);
            if end == start {
                continue;
            }
            if self.collision.allows(start, end) {
                self.positions[row] = end;
                changes.push(Change {
                    id: self.ids[row],
                    position_mm: end,
                });
            } else {
                blocked.push(self.ids[row]);
            }
        }
        let mut attacks = Vec::with_capacity(requests.len());
        let mut dirty = vec![false; self.ids.len()];
        for (attacker, target, sequence) in requests {
            let result = self.resolve_attack(attacker, target, tick, ready_tick, &mut dirty);
            attacks.push(AttackOutcome {
                attacker,
                target,
                sequence,
                result,
            });
        }
        let health_changes = dirty
            .iter()
            .enumerate()
            .filter(|(_, v)| **v)
            .map(|(row, _)| HealthChange {
                id: self.ids[row],
                health: self.health[row],
            })
            .collect();
        self.tick = tick;
        Ok(Frame {
            tick,
            changes,
            applied,
            blocked,
            attacks,
            health_changes,
        })
    }
}
fn movement(axes: [i16; 2], step: i32) -> [i32; 2] {
    let magnitude = axes
        .iter()
        .map(|v| i64::from(*v) * i64::from(*v))
        .sum::<i64>() as u64;
    let root = magnitude.isqrt();
    let ceil = root + u64::from(root * root < magnitude);
    let denominator = ceil.max(1000) as i64;
    axes.map(|v| (i64::from(v) * i64::from(step) / denominator) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(n: u64) -> WorldId {
        WorldId::new(n).unwrap()
    }
    fn scene() -> Scene {
        Scene::new(
            Rect {
                min: [-4000, -4000],
                max: [4000, 4000],
            },
            vec![Rect {
                min: [-100, -100],
                max: [100, 100],
            }],
        )
        .unwrap()
    }
    fn world() -> World {
        World::new(scene(), Rules::default()).unwrap()
    }
    fn input(sequence: u64, axes: [i16; 2]) -> MoveInput {
        MoveInput { sequence, axes }
    }
    fn distance(a: [i32; 2], b: [i32; 2]) -> i64 {
        (0..2)
            .map(|i| {
                let d = i64::from(a[i]) - i64::from(b[i]);
                d * d
            })
            .sum()
    }
    #[test]
    fn fixed_tick_latest_intent_speed_bound_and_expiration() {
        let mut w = world();
        let s = w.spawn_at_boundary(id(100), [-1000, -1000]).unwrap();
        w.submit(s, input(0, [1000, 0])).unwrap();
        w.submit(s, input(1, [1000, 1000])).unwrap();
        let frame = w.step().unwrap();
        assert_eq!(frame.tick, 1);
        assert_eq!(frame.applied.len(), 2);
        assert_eq!(frame.changes.len(), 1);
        assert_eq!(w.actor(id(100)).unwrap().position_mm, [-894, -894]);
        assert!(distance([-1000, -1000], [-894, -894]) <= 150 * 150);
        w.step().unwrap();
        w.step().unwrap();
        let position = w.actor(id(100)).unwrap().position_mm;
        let expired = w.step().unwrap();
        assert!(expired.changes.is_empty());
        assert_eq!(w.actor(id(100)).unwrap().position_mm, position);
        assert_eq!(w.submit(s, input(1, [0, 0])), Err(Error::Sequence));
    }
    #[test]
    fn ordinary_actors_overlap_and_client_cannot_mutate_a_snapshot() {
        let mut w = world();
        let a = w.spawn_at_boundary(id(1), [-1000, -1000]).unwrap();
        let b = w.spawn_at_boundary(id(2), [-1000, -1000]).unwrap();
        for s in [a, b] {
            w.submit(s, input(0, [1000, 0])).unwrap();
        }
        w.step().unwrap();
        assert_eq!(
            w.actor(id(1)).unwrap().position_mm,
            w.actor(id(2)).unwrap().position_mm
        );
        let mut copy = w.snapshot();
        copy[0].position_mm = [i32::MAX; 2];
        assert_eq!(w.actor(id(1)).unwrap().position_mm, [-850, -1000]);
    }
    #[test]
    fn static_swept_collision_blocks_tunneling_and_world_escape() {
        let scene = Scene::new(
            Rect {
                min: [-200000, -200000],
                max: [200000, 200000],
            },
            vec![Rect {
                min: [0, -1000],
                max: [100, 1000],
            }],
        )
        .unwrap();
        let mut w = World::new(
            scene,
            Rules {
                tick_ms: 1000,
                speed_mm_per_second: 100000,
                half_extent_mm: 100,
                ..Rules::default()
            },
        )
        .unwrap();
        let s = w.spawn_at_boundary(id(1), [-2000, 0]).unwrap();
        w.submit(s, input(0, [1000, 0])).unwrap();
        let blocked = w.step().unwrap();
        assert_eq!(blocked.blocked, vec![id(1)]);
        assert!(blocked.changes.is_empty());
        assert_eq!(w.actor(id(1)).unwrap().position_mm, [-2000, 0]);
        w.submit(s, input(1, [0, 1000])).unwrap();
        w.step().unwrap();
        assert_eq!(w.actor(id(1)).unwrap().position_mm, [-2000, 100000]);
        assert_eq!(w.step().unwrap().blocked, vec![id(1)]);
    }
    #[test]
    fn compaction_respawn_and_foreign_world_sessions_preserve_authority() {
        let mut w = world();
        let mut other = world();
        let foreign = other.spawn_at_boundary(id(99), [-1000, -1000]).unwrap();
        assert_eq!(
            w.submit(foreign, input(0, [1000, 0])),
            Err(Error::UnknownSession)
        );
        w.spawn_at_boundary(id(100), [-1000, -1000]).unwrap();
        let old = w.spawn_at_boundary(id(200), [1000, -1000]).unwrap();
        let moved = w.spawn_at_boundary(id(300), [1000, 1000]).unwrap();
        w.submit(old, input(0, [1000, 0])).unwrap();
        w.despawn_at_boundary(id(200)).unwrap();
        assert_eq!(w.rows[&id(300)], 1);
        let fresh = w.spawn_at_boundary(id(200), [1000, -1000]).unwrap();
        assert_ne!(fresh, old);
        assert_eq!(
            w.submit(old, input(1, [1000, 0])),
            Err(Error::UnknownSession)
        );
        w.submit(moved, input(0, [1000, 0])).unwrap();
        let frame = w.step().unwrap();
        assert_eq!(frame.applied.len(), 1);
        assert_eq!(frame.changes[0].id, id(300));
        assert_eq!(w.actor(id(200)).unwrap().position_mm, [1000, -1000]);
        assert_eq!(w.actor(id(300)).unwrap().position_mm, [1150, 1000]);
    }
    #[test]
    fn bounded_inputs_reject_without_consuming_retry_sequence() {
        let mut w = World::new(
            scene(),
            Rules {
                max_inputs: 1,
                max_inputs_per_session: 1,
                ..Rules::default()
            },
        )
        .unwrap();
        let s = w.spawn_at_boundary(id(1), [-1000, -1000]).unwrap();
        assert_eq!(w.submit(s, input(0, [1001, 0])), Err(Error::InvalidIntent));
        w.submit(s, input(0, [1000, 0])).unwrap();
        assert_eq!(w.submit(s, input(1, [0, 0])), Err(Error::InputBudget));
        w.step().unwrap();
        w.submit(s, input(1, [0, 0])).unwrap();
        w.step().unwrap();
        assert_eq!(w.actor(id(1)).unwrap().position_mm, [-850, -1000]);
        w.submit(s, input(u64::MAX, [0, 0])).unwrap();
        w.step().unwrap();
        assert_eq!(w.submit(s, input(0, [1000, 0])), Err(Error::Sequence));
    }
    #[test]
    fn invalid_configuration_capacity_and_exhausted_tick_fail_atomically() {
        assert_eq!(WorldId::new(0), Err(Error::InvalidWorldId));
        for rules in [
            Rules {
                tick_ms: 0,
                ..Rules::default()
            },
            Rules {
                speed_mm_per_second: 0,
                ..Rules::default()
            },
            Rules {
                max_actors: 0,
                ..Rules::default()
            },
            Rules {
                intent_hold_ticks: 0,
                ..Rules::default()
            },
        ] {
            assert!(matches!(
                World::new(scene(), rules),
                Err(Error::InvalidRules)
            ));
        }
        assert!(
            Scene::new(
                Rect {
                    min: [0, 0],
                    max: [100, 100]
                },
                vec![Rect {
                    min: [0, 0],
                    max: [1000, 1000]
                }]
            )
            .is_err()
        );
        let mut w = World::new(
            scene(),
            Rules {
                max_actors: 1,
                ..Rules::default()
            },
        )
        .unwrap();
        let s = w.spawn_at_boundary(id(1), [-1000, -1000]).unwrap();
        assert_eq!(
            w.spawn_at_boundary(id(1), [-1000, -1000]),
            Err(Error::DuplicateActor)
        );
        assert_eq!(
            w.spawn_at_boundary(id(2), [-1000, -1000]),
            Err(Error::Capacity)
        );
        w.submit(s, input(0, [1000, 0])).unwrap();
        w.tick = u64::MAX - 1;
        let before = w.snapshot();
        assert!(matches!(w.step(), Err(Error::CounterExhausted)));
        assert_eq!(w.snapshot(), before);
        assert_eq!(w.pending.len(), 1);
        w.tick = u64::MAX;
        assert!(matches!(w.step(), Err(Error::CounterExhausted)));
        assert_eq!(w.snapshot(), before);
    }
    #[test]
    fn randomized_movement_keeps_speed_collision_and_change_stream_invariants() {
        let mut w = world();
        let ids = [id(100), id(200), id(300)];
        let positions = [[-1000, -1000], [1000, -1000], [1000, 1000]];
        let sessions: Vec<_> = ids
            .iter()
            .zip(positions)
            .map(|(&id, p)| w.spawn_at_boundary(id, p).unwrap())
            .collect();
        let mut seed = 719u64;
        for sequence in 0..5000 {
            let before = w.snapshot();
            for &session in &sessions {
                let axes = std::array::from_fn(|_| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    ((seed >> 32) % 2001) as i16 - 1000
                });
                w.submit(session, input(sequence, axes)).unwrap();
                assert_eq!(
                    w.submit(session, input(sequence, axes)),
                    Err(Error::Sequence)
                );
            }
            let frame = w.step().unwrap();
            assert_eq!(frame.tick, sequence + 1);
            assert_eq!(frame.applied.len(), 3);
            for old in before {
                let p = w.actor(old.id).unwrap().position_mm;
                assert!(distance(old.position_mm, p) <= 150 * 150);
                assert!(p.iter().all(|v| (-3750..=3750).contains(v)));
                assert!(p[0] < -350 || p[0] > 350 || p[1] < -350 || p[1] > 350);
                let changes: Vec<_> = frame.changes.iter().filter(|v| v.id == old.id).collect();
                assert_eq!(changes.len(), usize::from(p != old.position_mm));
                if let Some(change) = changes.first() {
                    assert_eq!(change.position_mm, p);
                }
            }
        }
    }
}
