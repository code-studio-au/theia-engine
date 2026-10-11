//! Synthetic bounded state/ACK loss and reordering, not QUIC qualification.
use std::error::Error;
use theia_protocol::{
    replica::{Error as ReplicaError, Publication, Replica, Snapshot, Source},
    replication::{Acknowledgement, Error as SenderError, Packet, Recipient, Sender},
};
use theia_sim::{ActorState, WorldId};
fn oracle(tick: u64) -> Snapshot {
    Snapshot {
        tick,
        applied_move: Some(tick),
        actors: (1..=8)
            .filter(|id| !(id + tick).is_multiple_of(5))
            .map(|id| ActorState {
                id: WorldId::new(id).unwrap(),
                position_mm: [((tick * id) % 10000) as i32, id as i32 * 100],
                health: ((tick + id) % 101) as u32,
            })
            .collect(),
    }
}
#[derive(Default, Debug)]
struct Counts {
    full: u64,
    delta: u64,
    dropped_state: u64,
    reordered_state: u64,
    repairs: u64,
    ack_attempts: u64,
    dropped_ack: u64,
    reordered_ack: u64,
    stale_ack: u64,
    evicted_ack: u64,
}
struct Lab {
    sender: Sender,
    recipient: Recipient,
    replica: Replica,
    source: Source,
    expected_tick: u64,
    last_confirmed: Option<u64>,
    held_ack: Option<u64>,
    counts: Counts,
}
impl Lab {
    fn ack(&mut self, tick: u64) -> Result<(), Box<dyn Error>> {
        match self.sender.acknowledge(self.recipient, tick) {
            Ok(Acknowledgement::Advanced) => self.last_confirmed = Some(tick),
            Ok(Acknowledgement::Duplicate) => {}
            Ok(Acknowledgement::Stale) => self.counts.stale_ack += 1,
            Err(SenderError::UnknownSnapshot) => self.counts.evicted_ack += 1,
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }
    fn receive(&mut self, packet: &Packet) -> Result<(), Box<dyn Error>> {
        let (tick, result) = match packet {
            Packet::Full(s) => (s.tick, self.replica.publish(self.source, s.clone())),
            Packet::Delta(d) => (d.tick, self.replica.publish_delta(self.source, d)),
        };
        match result {
            Ok(Publication::New | Publication::Duplicate) => {
                self.expected_tick = self.expected_tick.max(tick);
                self.counts.ack_attempts += 1;
                if self.counts.ack_attempts.is_multiple_of(11) {
                    self.counts.dropped_ack += 1;
                } else if self.counts.ack_attempts.is_multiple_of(7) && self.held_ack.is_none() {
                    self.held_ack = Some(tick);
                } else {
                    self.ack(tick)?;
                    if let Some(older) = self.held_ack.take() {
                        self.counts.reordered_ack += 1;
                        self.ack(older)?;
                    }
                }
            }
            Ok(Publication::Stale) => {}
            Err(ReplicaError::MissingBaseline) => {
                self.counts.repairs += 1;
                self.sender.request_full(self.recipient)?;
                self.last_confirmed = None;
            }
            Err(e) => return Err(e.into()),
        }
        assert_eq!(self.replica.latest(), Some(&oracle(self.expected_tick)));
        assert!(self.sender.retained_count() <= 8);
        Ok(())
    }
    fn prepare(&mut self, tick: u64) -> Result<Packet, Box<dyn Error>> {
        let packet = self.sender.prepare(self.recipient, oracle(tick))?;
        match &packet {
            Packet::Full(_) => self.counts.full += 1,
            Packet::Delta(d) => {
                self.counts.delta += 1;
                assert_eq!(Some(d.baseline_tick), self.last_confirmed);
            }
        }
        Ok(packet)
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut sender = Sender::new(8, 8)?;
    let recipient = sender.connect()?;
    let mut replica = Replica::new(8)?;
    let source = replica.connect()?;
    let Packet::Full(initial) = sender.prepare(recipient, oracle(0))? else {
        panic!()
    };
    replica.publish(source, initial)?;
    sender.acknowledge(recipient, 0)?;
    let mut lab = Lab {
        sender,
        recipient,
        replica,
        source,
        expected_tick: 0,
        last_confirmed: Some(0),
        held_ack: None,
        counts: Counts::default(),
    };
    let mut held = None;
    for tick in 1u64..=10000 {
        let packet = lab.prepare(tick)?;
        if tick.is_multiple_of(13) {
            lab.counts.dropped_state += 1;
            continue;
        }
        if tick.is_multiple_of(5) && held.is_none() {
            held = Some(packet);
            continue;
        }
        lab.receive(&packet)?;
        if tick.is_multiple_of(17) {
            lab.receive(&packet)?;
        }
        if let Some(older) = held.take() {
            lab.counts.reordered_state += 1;
            lab.receive(&older)?;
        }
    }
    lab.sender.request_full(recipient)?;
    lab.last_confirmed = None;
    let final_packet = lab.prepare(10001)?;
    assert!(matches!(final_packet, Packet::Full(_)));
    lab.receive(&final_packet)?;
    assert_eq!(lab.replica.latest(), Some(&oracle(10001)));
    assert!(
        lab.counts.full > 0
            && lab.counts.delta > 0
            && lab.counts.repairs > 0
            && lab.counts.dropped_ack > 0
            && lab.counts.stale_ack > 0
    );
    println!(
        "Verified 10000 generated packets against independent complete state with explicit acknowledged sender baselines: {:?}; sender <=8 sets, receiver <=2 sets, <=8 actors each; final full repair tick 10001. Synthetic application messages only; no wire/QUIC or performance claim.",
        lab.counts
    );
    Ok(())
}
