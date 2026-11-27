//! Synthetic bounded application-message loss/reordering and explicit full repair.
//! No sockets or authority mutation; independent complete-state formula is oracle.
use std::error::Error;
use theia_protocol::{
    delta::Delta,
    replica::{Error as ReplicaError, Publication, Replica, Snapshot, Source},
};
use theia_sim::{ActorState, WorldId};
#[derive(Clone)]
enum Packet {
    Full(Snapshot),
    Delta(Delta),
}
#[derive(Default, Debug)]
struct Counts {
    new: u64,
    duplicate: u64,
    stale: u64,
    missing: u64,
    dropped: u64,
    reordered: u64,
}
fn oracle(tick: u64) -> Snapshot {
    Snapshot {
        tick,
        applied_move: Some(tick),
        actors: (1..=16)
            .filter(|id| !(id + tick).is_multiple_of(7))
            .map(|id| ActorState {
                id: WorldId::new(id).unwrap(),
                position_mm: [((id * tick) % 10000) as i32, -(((tick + id) % 1000) as i32)],
                health: ((tick * 3 + id) % 101) as u32,
            })
            .collect(),
    }
}
fn deliver(
    replica: &mut Replica,
    source: Source,
    packet: &Packet,
    now: u64,
    expected_tick: &mut u64,
    counts: &mut Counts,
) -> Result<(), Box<dyn Error>> {
    let tick = match packet {
        Packet::Full(s) => s.tick,
        Packet::Delta(d) => d.tick,
    };
    let result = match packet {
        Packet::Full(s) => replica.publish(source, s.clone()),
        Packet::Delta(d) => replica.publish_delta(source, d),
    };
    match result {
        Ok(Publication::New) => {
            counts.new += 1;
            *expected_tick = (*expected_tick).max(tick);
        }
        Ok(Publication::Duplicate) => counts.duplicate += 1,
        Ok(Publication::Stale) => counts.stale += 1,
        Err(ReplicaError::MissingBaseline) => {
            counts.missing += 1;
            // Caller explicitly requests a complete set, not guesses absent actor values.
            replica.publish(source, oracle(now))?;
            *expected_tick = now;
        }
        Err(error) => return Err(error.into()),
    }
    assert_eq!(replica.latest(), Some(&oracle(*expected_tick)));
    assert!(replica.latest().unwrap().actors.len() <= 16);
    assert!(replica.previous().is_none_or(|s| s.actors.len() <= 16));
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut replica = Replica::new(16)?;
    let source = replica.connect()?;
    replica.publish(source, oracle(0))?;
    let mut expected_tick = 0;
    let mut counts = Counts::default();
    let mut held = None;
    for tick in 1u64..=10000 {
        let packet = if tick.is_multiple_of(11) {
            Packet::Full(oracle(tick))
        } else {
            // Sender baselines can lag the receiver's two retained endpoints.
            let base = tick.saturating_sub(1 + tick % 3);
            Packet::Delta(Delta::between(&oracle(base), &oracle(tick), 16)?)
        };
        if tick.is_multiple_of(7) {
            counts.dropped += 1;
            continue;
        }
        if tick.is_multiple_of(5) && held.is_none() {
            held = Some(packet);
            continue;
        }
        deliver(
            &mut replica,
            source,
            &packet,
            tick,
            &mut expected_tick,
            &mut counts,
        )?;
        if tick.is_multiple_of(13) {
            deliver(
                &mut replica,
                source,
                &packet,
                tick,
                &mut expected_tick,
                &mut counts,
            )?;
        }
        if let Some(older) = held.take() {
            counts.reordered += 1;
            deliver(
                &mut replica,
                source,
                &older,
                tick,
                &mut expected_tick,
                &mut counts,
            )?;
        }
    }
    // Full repair is independent of missing/dropped final delta or held history.
    replica.publish(source, oracle(10001))?;
    assert_eq!(replica.latest(), Some(&oracle(10001)));
    assert!(
        counts.new > 0
            && counts.duplicate > 0
            && counts.stale > 0
            && counts.missing > 0
            && counts.dropped > 0
            && counts.reordered > 0
    );
    let fresh = replica.connect()?;
    let old = Delta::between(&oracle(10000), &oracle(10001), 16)?;
    assert_eq!(
        replica.publish_delta(source, &old),
        Err(ReplicaError::UnknownSource)
    );
    assert_eq!(
        replica.publish_delta(fresh, &old),
        Err(ReplicaError::MissingBaseline)
    );
    assert!(replica.latest().is_none());
    println!(
        "Verified 10000 generated synthetic delta/full packets with a complete-state delivery oracle: {counts:?}; final full repair tick 10001; at most two retained sets/16 actors each; stale source and missing baseline preserve state. No transport, baseline ACK policy or performance claim."
    );
    Ok(())
}
