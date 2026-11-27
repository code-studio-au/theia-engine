//! Connection-scoped readonly full/delta replica with explicit application ACK/repair.
use crate::{
    Result, replication_wire,
    wire::{self, Message, Welcome},
};
use quinn::Connection;
use theia_protocol::replica::{Publication, Replica, Snapshot, Source};
#[derive(Clone, Copy, Debug, Default)]
pub struct ReplicationStats {
    pub full: u64,
    pub delta: u64,
    pub acks_sent: u64,
    pub repairs_sent: u64,
    pub withheld_acks: u64,
}
pub struct Client {
    replica: Replica,
    source: Source,
    samples: u64,
    stats: ReplicationStats,
    qualify_repair: bool,
    connection: Option<Connection>,
    welcome: Option<Welcome>,
}
impl Client {
    pub fn new() -> Result<Self> {
        let mut replica = Replica::new(2)?;
        let source = replica.connect()?;
        Ok(Self {
            replica,
            source,
            samples: 0,
            stats: ReplicationStats::default(),
            qualify_repair: false,
            connection: None,
            welcome: None,
        })
    }
    pub fn bind(&mut self, connection: &Connection, welcome: Welcome) -> Result<()> {
        welcome.validate()?;
        self.source = self.replica.connect()?;
        self.connection = Some(connection.clone());
        self.welcome = Some(welcome);
        self.stats = ReplicationStats::default();
        self.qualify_repair = false;
        Ok(())
    }
    pub async fn receive(&mut self, connection: &Connection) -> Result<()> {
        if self.connection.as_ref().map(Connection::stable_id) != Some(connection.stable_id()) {
            return Err("Snapshot delivered from an unbound connection".into());
        }
        let message = replication_wire::decode(&connection.read_datagram().await?)?;
        let full = matches!(message, replication_wire::Message::Full(_));
        if self.apply_state(message)?.is_none() {
            self.qualify_repair = false;
            connection.send_datagram(
                replication_wire::encode(&replication_wire::Message::Repair)?.into(),
            )?;
            self.stats.repairs_sent += 1;
            return Ok(());
        }
        if full {
            self.stats.full += 1;
        } else {
            self.stats.delta += 1;
        }
        let tick = self
            .replica
            .latest()
            .ok_or("Missing complete snapshot")?
            .tick;
        if self.qualify_repair && !full {
            // Explicit qualification injection: hold ACKs after the server proves
            // it selected a baseline, until receiver eviction requests full repair.
            self.stats.withheld_acks += 1;
        } else {
            connection.send_datagram(
                replication_wire::encode(&replication_wire::Message::Ack(tick))?.into(),
            )?;
            self.stats.acks_sent += 1;
        }
        // Qualification screen times are supplied explicitly, independently of
        // receive timing. This is not a network clock synchronization estimator.
        let current = self.replica.latest().ok_or("missing snapshot")?;
        for tick in [0, current.tick.saturating_sub(1), current.tick, u64::MAX] {
            let frame = self
                .replica
                .present(theia_protocol::presentation::Time::new(tick, 0.5)?, 4)?
                .ok_or("missing presentation frame")?;
            assert_eq!(frame.to_tick, current.tick);
            assert_eq!(frame.actors.len(), current.actors.len());
            for (visual, authoritative) in frame.actors.iter().zip(&current.actors) {
                assert_eq!(visual.id, authoritative.id);
                assert_eq!(visual.health, authoritative.health);
                assert!(
                    visual
                        .position_mm
                        .iter()
                        .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
                );
            }
            self.samples += 1;
        }
        Ok(())
    }
    fn apply_state(&mut self, message: replication_wire::Message) -> Result<Option<Publication>> {
        let snapshot = match message {
            replication_wire::Message::Full(snapshot) => snapshot,
            replication_wire::Message::Delta(delta) => {
                delta.validate(2)?;
                if self.replica.latest().is_some_and(|s| delta.tick < s.tick) {
                    return Ok(Some(self.replica.publish_delta(self.source, &delta)?));
                }
                let Some(baseline) = self
                    .replica
                    .latest()
                    .into_iter()
                    .chain(self.replica.previous())
                    .find(|s| s.tick == delta.baseline_tick)
                else {
                    return Ok(None);
                };
                delta.reconstruct(baseline, 2)?
            }
            _ => return Err("Unexpected server replication control".into()),
        };
        let welcome = self.welcome.ok_or("Missing server welcome")?;
        if snapshot.actors.len() != 2
            || !snapshot.actors.iter().any(|v| v.id == welcome.actor)
            || !snapshot.actors.iter().any(|v| v.id == welcome.target)
        {
            return Err("Snapshot roster differs from bootstrap".into());
        }
        Ok(Some(self.replica.publish(self.source, snapshot)?))
    }
    /// Qualification-only application ACK withholding, once until missing baseline.
    pub fn qualify_baseline_eviction(&mut self) -> Result<()> {
        if self.connection.is_none() {
            return Err("Repair qualification needs a bound connection".into());
        }
        self.qualify_repair = true;
        Ok(())
    }
    pub fn replication_stats(&self) -> ReplicationStats {
        self.stats
    }
    pub fn samples(&self) -> u64 {
        self.samples
    }
    pub fn previous(&self) -> Option<&Snapshot> {
        self.replica.previous()
    }
    pub fn latest(&self) -> Option<&Snapshot> {
        self.replica.latest()
    }
    pub fn disconnect(&mut self) -> Result<()> {
        self.replica.disconnect(self.source)?;
        self.connection = None;
        self.qualify_repair = false;
        self.welcome = None;
        Ok(())
    }
}

/// Server-owned identity/profile bootstrap after a complete verified handshake.
pub async fn receive_welcome(connection: &Connection) -> Result<Welcome> {
    let (mut send, mut receive) =
        tokio::time::timeout(std::time::Duration::from_secs(30), connection.accept_bi()).await??;
    let bytes = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        receive.read_to_end(wire::MAX_BYTES),
    )
    .await??;
    let Message::Welcome(welcome) = wire::decode(&bytes)? else {
        return Err("Expected server welcome".into());
    };
    send.finish()?;
    Ok(welcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use theia_protocol::delta::Delta;
    use theia_sim::{ActorState, WorldId};
    fn state(tick: u64) -> Snapshot {
        Snapshot {
            tick,
            applied_move: Some(tick),
            actors: vec![
                ActorState {
                    id: WorldId::new(10).unwrap(),
                    position_mm: [tick as i32, 0],
                    health: 100,
                },
                ActorState {
                    id: WorldId::new(20).unwrap(),
                    position_mm: [100, 0],
                    health: 75,
                },
            ],
        }
    }
    fn bound_fixture() -> Client {
        let mut client = Client::new().unwrap();
        client.welcome = Some(Welcome {
            actor: WorldId::new(10).unwrap(),
            target: WorldId::new(20).unwrap(),
            profile: wire::Profile::Laboratory,
        });
        client
    }
    #[test]
    fn delta_roster_failure_is_atomic_and_missing_baseline_preserves_complete_endpoints() {
        let mut client = bound_fixture();
        client
            .apply_state(replication_wire::Message::Full(state(1)))
            .unwrap();
        let delta = Delta::between(&state(1), &state(2), 2).unwrap();
        assert_eq!(
            client
                .apply_state(replication_wire::Message::Delta(delta))
                .unwrap(),
            Some(Publication::New)
        );
        let mut wrong = state(3);
        wrong.actors[1].id = WorldId::new(30).unwrap();
        let delta = Delta::between(&state(2), &wrong, 2).unwrap();
        assert!(
            client
                .apply_state(replication_wire::Message::Delta(delta))
                .is_err()
        );
        assert_eq!(client.latest(), Some(&state(2)));
        assert_eq!(client.previous(), Some(&state(1)));
        assert!(
            client
                .apply_state(replication_wire::Message::Full(wrong))
                .is_err()
        );
        client
            .apply_state(replication_wire::Message::Full(state(3)))
            .unwrap();
        let missing = Delta::between(&state(1), &state(4), 2).unwrap();
        assert_eq!(
            client
                .apply_state(replication_wire::Message::Delta(missing))
                .unwrap(),
            None
        );
        assert_eq!(client.latest(), Some(&state(3)));
        assert_eq!(client.previous(), Some(&state(2)));
        client
            .apply_state(replication_wire::Message::Full(state(4)))
            .unwrap();
        assert_eq!(client.latest(), Some(&state(4)));
    }
    #[test]
    fn empty_or_partial_bootstrap_roster_and_control_as_state_never_publish() {
        let mut client = bound_fixture();
        for count in [0, 1] {
            let mut snapshot = state(1);
            snapshot.actors.truncate(count);
            assert!(
                client
                    .apply_state(replication_wire::Message::Full(snapshot))
                    .is_err()
            );
        }
        assert!(
            client
                .apply_state(replication_wire::Message::Ack(1))
                .is_err()
        );
        assert!(
            client
                .apply_state(replication_wire::Message::Repair)
                .is_err()
        );
        assert!(client.latest().is_none());
    }
}
