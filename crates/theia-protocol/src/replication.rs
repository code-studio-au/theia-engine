//! Bounded per-recipient sender prototype. Application ACKs name complete sets;
//! no wire protocol, authentication, transport ACK inference or durable state.
use crate::{
    delta::Delta,
    replica::{self, Snapshot, validate_actors},
};
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipient {
    owner: u64,
    connection: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    Full(Snapshot),
    Delta(Delta),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acknowledgement {
    Advanced,
    Duplicate,
    Stale,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimit,
    Exhausted,
    UnknownRecipient,
    UnknownSnapshot,
    State(replica::Error),
}
impl From<replica::Error> for Error {
    fn from(value: replica::Error) -> Self {
        Self::State(value)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub struct Sender {
    owner: u64,
    next_connection: u64,
    recipient: Option<Recipient>,
    max_actors: usize,
    history_limit: usize,
    history: VecDeque<Snapshot>,
    acknowledged: Option<u64>,
    repair_after: Option<u64>,
}
impl Sender {
    pub fn new(max_actors: usize, history_limit: usize) -> Result<Self, Error> {
        if !(1..=128).contains(&max_actors) || !(2..=16).contains(&history_limit) {
            return Err(Error::InvalidLimit);
        }
        let owner = NEXT_OWNER
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| Error::Exhausted)?;
        Ok(Self {
            owner,
            next_connection: 1,
            recipient: None,
            max_actors,
            history_limit,
            history: VecDeque::with_capacity(history_limit),
            acknowledged: None,
            repair_after: None,
        })
    }
    /// Caller binds this locally issued context to its exact trusted connection.
    pub fn connect(&mut self) -> Result<Recipient, Error> {
        let next = self
            .next_connection
            .checked_add(1)
            .ok_or(Error::Exhausted)?;
        let recipient = Recipient {
            owner: self.owner,
            connection: self.next_connection,
        };
        self.next_connection = next;
        self.recipient = Some(recipient);
        self.history.clear();
        self.acknowledged = None;
        self.repair_after = None;
        Ok(recipient)
    }
    fn check(&self, recipient: Recipient) -> Result<(), Error> {
        if self.recipient == Some(recipient) {
            Ok(())
        } else {
            Err(Error::UnknownRecipient)
        }
    }
    pub fn disconnect(&mut self, recipient: Recipient) -> Result<(), Error> {
        self.check(recipient)?;
        self.recipient = None;
        self.history.clear();
        self.acknowledged = None;
        self.repair_after = None;
        Ok(())
    }
    /// ACK only a complete set that this sender prepared for this live recipient.
    /// Claiming a tick is not proof of gameplay authority or account identity.
    pub fn acknowledge(
        &mut self,
        recipient: Recipient,
        tick: u64,
    ) -> Result<Acknowledgement, Error> {
        self.check(recipient)?;
        if self.repair_after.is_some_and(|floor| tick <= floor)
            || self.acknowledged.is_some_and(|v| tick < v)
        {
            return Ok(Acknowledgement::Stale);
        }
        if !self.history.iter().any(|s| s.tick == tick) {
            return Err(Error::UnknownSnapshot);
        }
        if self.acknowledged == Some(tick) {
            return Ok(Acknowledgement::Duplicate);
        }
        self.acknowledged = Some(tick);
        self.repair_after = None;
        Ok(Acknowledgement::Advanced)
    }
    /// Full repair continues until a NEWER complete set is explicitly acknowledged.
    /// Delayed pre-repair ACKs cannot immediately reactivate a lost baseline.
    pub fn request_full(&mut self, recipient: Recipient) -> Result<(), Error> {
        self.check(recipient)?;
        self.repair_after = self.history.back().map(|s| s.tick);
        self.acknowledged = None;
        Ok(())
    }
    pub fn prepare(
        &mut self,
        recipient: Recipient,
        mut snapshot: Snapshot,
    ) -> Result<Packet, Error> {
        self.check(recipient)?;
        validate_actors(&snapshot.actors, self.max_actors)?;
        if let Some(last) = self.history.back() {
            if snapshot.tick < last.tick {
                return Err(replica::Error::InvalidState.into());
            }
            if snapshot.tick == last.tick && snapshot != *last {
                return Err(replica::Error::ConflictingTick.into());
            }
            if last
                .applied_move
                .is_some_and(|v| snapshot.applied_move.is_none_or(|n| n < v))
            {
                return Err(replica::Error::RegressingWatermark.into());
            }
        }
        snapshot.actors = snapshot.actors.into_boxed_slice().into_vec();
        if self.history.back().is_none_or(|s| s.tick != snapshot.tick) {
            if self.history.len() == self.history_limit {
                self.history.pop_front();
            }
            self.history.push_back(snapshot.clone());
        }
        if self
            .acknowledged
            .is_some_and(|tick| !self.history.iter().any(|s| s.tick == tick))
        {
            self.acknowledged = None;
        }
        match self
            .acknowledged
            .and_then(|tick| self.history.iter().find(|s| s.tick == tick))
        {
            Some(baseline) if baseline.tick < snapshot.tick => Ok(Packet::Delta(Delta::between(
                baseline,
                &snapshot,
                self.max_actors,
            )?)),
            _ => Ok(Packet::Full(snapshot)),
        }
    }
    pub fn retained_count(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replica::{Publication, Replica};
    use theia_sim::{ActorState, WorldId};
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
    fn only_exact_prepared_application_acknowledgements_enable_deltas() {
        let mut sender = Sender::new(2, 4).unwrap();
        let recipient = sender.connect().unwrap();
        assert_eq!(
            sender.acknowledge(recipient, 0),
            Err(Error::UnknownSnapshot)
        );
        assert!(matches!(
            sender.prepare(recipient, state(1)),
            Ok(Packet::Full(_))
        ));
        assert!(matches!(
            sender.prepare(recipient, state(2)),
            Ok(Packet::Full(_))
        ));
        assert_eq!(
            sender.acknowledge(recipient, 3),
            Err(Error::UnknownSnapshot)
        );
        assert_eq!(
            sender.acknowledge(recipient, 1),
            Ok(Acknowledgement::Advanced)
        );
        let Packet::Delta(delta) = sender.prepare(recipient, state(3)).unwrap() else {
            panic!("Expected acknowledged delta")
        };
        assert_eq!(delta.baseline_tick, 1);
        assert_eq!(delta.reconstruct(&state(1), 2), Ok(state(3)));
        assert_eq!(
            sender.acknowledge(recipient, 1),
            Ok(Acknowledgement::Duplicate)
        );
        assert_eq!(
            sender.acknowledge(recipient, 3),
            Ok(Acknowledgement::Advanced)
        );
        assert_eq!(sender.acknowledge(recipient, 2), Ok(Acknowledgement::Stale));
        assert!(matches!(
            sender.prepare(recipient, state(3)),
            Ok(Packet::Full(_))
        ));
        let Packet::Delta(delta) = sender.prepare(recipient, state(4)).unwrap() else {
            panic!("Expected newer acknowledged baseline")
        };
        assert_eq!(delta.baseline_tick, 3);
    }
    #[test]
    fn eviction_and_repair_ignore_delayed_ack_until_new_full_state_is_confirmed() {
        let mut sender = Sender::new(2, 2).unwrap();
        let recipient = sender.connect().unwrap();
        sender.prepare(recipient, state(1)).unwrap();
        sender.acknowledge(recipient, 1).unwrap();
        assert!(matches!(
            sender.prepare(recipient, state(2)),
            Ok(Packet::Delta(_))
        ));
        assert!(matches!(
            sender.prepare(recipient, state(3)),
            Ok(Packet::Full(_))
        ));
        assert_eq!(sender.retained_count(), 2);
        assert_eq!(
            sender.acknowledge(recipient, 1),
            Err(Error::UnknownSnapshot)
        );
        sender.acknowledge(recipient, 3).unwrap();
        sender.request_full(recipient).unwrap();
        assert_eq!(sender.acknowledge(recipient, 3), Ok(Acknowledgement::Stale));
        assert!(matches!(
            sender.prepare(recipient, state(3)),
            Ok(Packet::Full(_))
        ));
        assert!(matches!(
            sender.prepare(recipient, state(4)),
            Ok(Packet::Full(_))
        ));
        assert_eq!(
            sender.acknowledge(recipient, 4),
            Ok(Acknowledgement::Advanced)
        );
        assert!(matches!(
            sender.prepare(recipient, state(5)),
            Ok(Packet::Delta(_))
        ));
        let mut replica = Replica::new(2).unwrap();
        let source = replica.connect().unwrap();
        assert_eq!(replica.publish(source, state(4)), Ok(Publication::New));
        let Packet::Delta(delta) = sender.prepare(recipient, state(5)).unwrap() else {
            panic!()
        };
        assert_eq!(replica.publish_delta(source, &delta), Ok(Publication::New));
        assert_eq!(replica.latest(), Some(&state(5)));
    }
    #[test]
    fn invalid_states_limits_and_old_foreign_contexts_cannot_change_baselines() {
        assert!(matches!(Sender::new(0, 2), Err(Error::InvalidLimit)));
        assert!(matches!(Sender::new(2, 17), Err(Error::InvalidLimit)));
        let mut sender = Sender::new(2, 4).unwrap();
        let old = sender.connect().unwrap();
        sender.prepare(old, state(2)).unwrap();
        sender.acknowledge(old, 2).unwrap();
        let mut conflict = state(2);
        conflict.actors[0].health = 0;
        assert_eq!(
            sender.prepare(old, conflict),
            Err(Error::State(replica::Error::ConflictingTick))
        );
        let mut regress = state(3);
        regress.applied_move = Some(1);
        assert_eq!(
            sender.prepare(old, regress),
            Err(Error::State(replica::Error::RegressingWatermark))
        );
        assert_eq!(
            sender.prepare(old, state(1)),
            Err(Error::State(replica::Error::InvalidState))
        );
        assert_eq!(sender.retained_count(), 1);
        let mut other = Sender::new(2, 4).unwrap();
        let foreign = other.connect().unwrap();
        assert_eq!(sender.acknowledge(foreign, 2), Err(Error::UnknownRecipient));
        let fresh = sender.connect().unwrap();
        assert_eq!(sender.retained_count(), 0);
        assert_eq!(sender.acknowledge(old, 2), Err(Error::UnknownRecipient));
        assert_eq!(sender.request_full(old), Err(Error::UnknownRecipient));
        assert!(matches!(
            sender.prepare(fresh, state(1)),
            Ok(Packet::Full(_))
        ));
        sender.disconnect(fresh).unwrap();
        assert_eq!(sender.retained_count(), 0);
        assert_eq!(
            sender.prepare(fresh, state(2)),
            Err(Error::UnknownRecipient)
        );
        sender.next_connection = u64::MAX;
        assert_eq!(sender.connect(), Err(Error::Exhausted));
        assert!(sender.recipient.is_none());
    }
}
