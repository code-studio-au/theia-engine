//! Transport-neutral authority adapter. No sockets, authentication, durable
//! exactly-once effects or wire format. Peer handles come from trusted transport context.
use std::{
    collections::BTreeMap,
    error, fmt,
    sync::atomic::{AtomicU64, Ordering},
};
use theia_sim::{
    ActorState, AttackInput, AttackReject, AttackResult, MoveInput, Rules, Scene, Session, World,
    WorldId,
};
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Peer {
    owner: u64,
    id: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnknownPeer,
    StaleMove,
    RequestStale,
    RequestExhausted,
    RequestConflict,
    RequestWindow,
    OutcomePending,
    OutcomeUnknown,
    SequenceExhausted,
    Capacity,
    InvalidLimits,
    Simulation(theia_sim::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl error::Error for Error {}
impl From<theia_sim::Error> for Error {
    fn from(v: theia_sim::Error) -> Self {
        Self::Simulation(v)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_peers: usize,
    pub request_window: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_peers: 128,
            request_window: 64,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub request: u64,
    pub tick: u64,
    pub attacker: WorldId,
    pub target: WorldId,
    pub result: AttackResult,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    Queued,
    Pending,
    Completed(Outcome),
    Acknowledged,
}
#[derive(Clone, Copy)]
enum Status {
    Pending,
    Completed(Outcome),
    Acknowledged,
}
struct Entry {
    target: WorldId,
    status: Status,
}
struct Client {
    actor: WorldId,
    session: Session,
    next_internal: Option<u64>,
    last_move: Option<u64>,
    applied_move: Option<u64>,
    pending_moves: BTreeMap<u64, u64>,
    pending_requests: BTreeMap<u64, u64>,
    floor: u64,
    exhausted: bool,
    requests: BTreeMap<u64, Entry>,
}
pub struct Frame {
    pub simulation: theia_sim::Frame,
    pub move_acks: Vec<(Peer, u64)>,
    pub outcomes: Vec<(Peer, Outcome)>,
}
pub struct Authority {
    world: World,
    limits: Limits,
    owner: u64,
    next_peer: u64,
    peers: BTreeMap<Peer, Client>,
    actors: BTreeMap<WorldId, Peer>,
}
impl Authority {
    pub fn new(scene: Scene, rules: Rules, limits: Limits) -> Result<Self> {
        if limits.max_peers == 0
            || limits.max_peers > rules.max_actors
            || limits.max_peers > 4096
            || !(1..=256).contains(&limits.request_window)
        {
            return Err(Error::InvalidLimits);
        }
        let world = World::new(scene, rules)?;
        let owner = NEXT_OWNER
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| Error::SequenceExhausted)?;
        Ok(Self {
            world,
            limits,
            owner,
            next_peer: 1,
            peers: BTreeMap::new(),
            actors: BTreeMap::new(),
        })
    }
    /// Server-side attachment at a tick boundary; IDs and spawn are not client commands.
    pub fn attach_actor(&mut self, id: WorldId, position: [i32; 2]) -> Result<Peer> {
        if self.peers.len() >= self.limits.max_peers {
            return Err(Error::Capacity);
        }
        let next = self
            .next_peer
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        let session = self.world.spawn_at_boundary(id, position)?;
        let peer = Peer {
            owner: self.owner,
            id: self.next_peer,
        };
        self.next_peer = next;
        self.peers.insert(
            peer,
            Client {
                actor: id,
                session,
                next_internal: Some(0),
                last_move: None,
                applied_move: None,
                pending_moves: BTreeMap::new(),
                pending_requests: BTreeMap::new(),
                floor: 0,
                exhausted: false,
                requests: BTreeMap::new(),
            },
        );
        self.actors.insert(id, peer);
        Ok(peer)
    }
    pub fn disconnect(&mut self, peer: Peer) -> Result<()> {
        let actor = self.peers.get(&peer).ok_or(Error::UnknownPeer)?.actor;
        self.world.despawn_at_boundary(actor)?;
        self.peers.remove(&peer);
        self.actors.remove(&actor);
        Ok(())
    }
    pub fn actor(&self, id: WorldId) -> Option<ActorState> {
        self.world.actor(id)
    }
    pub fn snapshot(&self) -> Vec<ActorState> {
        self.world.snapshot()
    }
    pub fn applied_move(&self, peer: Peer) -> Result<Option<u64>> {
        Ok(self
            .peers
            .get(&peer)
            .ok_or(Error::UnknownPeer)?
            .applied_move)
    }
    /// Supersedable movement sequence is independent from reliable request identity.
    /// A server applied watermark is produced only after simulation consumes input.
    pub fn movement(&mut self, peer: Peer, input: MoveInput) -> Result<()> {
        let client = self.peers.get_mut(&peer).ok_or(Error::UnknownPeer)?;
        if client.last_move.is_some_and(|v| input.sequence <= v) {
            return Err(Error::StaleMove);
        }
        let internal = client.next_internal.ok_or(Error::SequenceExhausted)?;
        self.world.submit(
            client.session,
            MoveInput {
                sequence: internal,
                axes: input.axes,
            },
        )?;
        client.next_internal = internal.checked_add(1);
        client.last_move = Some(input.sequence);
        client.pending_moves.insert(internal, input.sequence);
        Ok(())
    }
    /// IDs start at zero, can arrive out of order inside a bounded sliding window.
    /// Duplicate payloads reuse pending/completed status. Conflicting reuse fails.
    pub fn attack(&mut self, peer: Peer, input: AttackInput) -> Result<Reply> {
        let client = self.peers.get_mut(&peer).ok_or(Error::UnknownPeer)?;
        if input.sequence < client.floor {
            return Err(Error::RequestStale);
        }
        if client.exhausted {
            return Err(Error::RequestExhausted);
        }
        if input.sequence - client.floor >= self.limits.request_window as u64 {
            return Err(Error::RequestWindow);
        }
        if let Some(entry) = client.requests.get(&input.sequence) {
            if entry.target != input.target {
                return Err(Error::RequestConflict);
            }
            return Ok(match entry.status {
                Status::Pending => Reply::Pending,
                Status::Completed(v) => Reply::Completed(v),
                Status::Acknowledged => Reply::Acknowledged,
            });
        }
        let internal = client.next_internal.ok_or(Error::SequenceExhausted)?;
        match self.world.submit_attack(
            client.session,
            AttackInput {
                sequence: internal,
                target: input.target,
            },
        ) {
            Ok(()) => {
                client.next_internal = internal.checked_add(1);
                client.pending_requests.insert(internal, input.sequence);
                client.requests.insert(
                    input.sequence,
                    Entry {
                        target: input.target,
                        status: Status::Pending,
                    },
                );
                Ok(Reply::Queued)
            }
            Err(theia_sim::Error::DeadActor) => {
                let outcome = Outcome {
                    request: input.sequence,
                    tick: self.world.tick(),
                    attacker: client.actor,
                    target: input.target,
                    result: AttackResult::Rejected(AttackReject::DeadAttacker),
                };
                client.requests.insert(
                    input.sequence,
                    Entry {
                        target: input.target,
                        status: Status::Completed(outcome),
                    },
                );
                Ok(Reply::Completed(outcome))
            }
            Err(error) => Err(error.into()),
        }
    }
    /// Application outcome acknowledgement, never a QUIC packet ACK. Pending outcomes
    /// cannot be freed. Tombstones retain replay protection until the contiguous floor advances.
    pub fn acknowledge(&mut self, peer: Peer, request: u64) -> Result<()> {
        let client = self.peers.get_mut(&peer).ok_or(Error::UnknownPeer)?;
        if request < client.floor || client.exhausted {
            return Ok(());
        }
        let entry = client
            .requests
            .get_mut(&request)
            .ok_or(Error::OutcomeUnknown)?;
        if matches!(entry.status, Status::Pending) {
            return Err(Error::OutcomePending);
        }
        entry.status = Status::Acknowledged;
        while client
            .requests
            .get(&client.floor)
            .is_some_and(|v| matches!(v.status, Status::Acknowledged))
        {
            client.requests.remove(&client.floor);
            if let Some(next) = client.floor.checked_add(1) {
                client.floor = next;
            } else {
                client.exhausted = true;
                break;
            }
        }
        Ok(())
    }
    pub fn retained_outcomes(&self, peer: Peer) -> Result<usize> {
        Ok(self
            .peers
            .get(&peer)
            .ok_or(Error::UnknownPeer)?
            .requests
            .len())
    }
    pub fn step(&mut self) -> Result<Frame> {
        let simulation = self.world.step()?;
        let mut move_acks = BTreeMap::new();
        let mut outcomes = Vec::with_capacity(simulation.attacks.len());
        for applied in &simulation.applied {
            let peer = self.actors[&applied.id];
            let client = self.peers.get_mut(&peer).unwrap();
            if let Some(sequence) = client.pending_moves.remove(&applied.sequence) {
                client.applied_move = Some(sequence);
                move_acks.insert(peer, sequence);
            }
        }
        for attack in &simulation.attacks {
            let peer = self.actors[&attack.attacker];
            let client = self.peers.get_mut(&peer).unwrap();
            let request = client
                .pending_requests
                .remove(&attack.sequence)
                .expect("all adapter attacks have pending request mapping");
            let outcome = Outcome {
                request,
                tick: simulation.tick,
                attacker: attack.attacker,
                target: attack.target,
                result: attack.result,
            };
            client.requests.get_mut(&request).unwrap().status = Status::Completed(outcome);
            outcomes.push((peer, outcome));
        }
        Ok(Frame {
            simulation,
            move_acks: move_acks.into_iter().collect(),
            outcomes,
        })
    }
}

#[cfg(test)]
mod tests;
