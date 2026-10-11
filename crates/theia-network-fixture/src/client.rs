//! Connection-scoped readonly replica for the bounded full-snapshot wire profile.
use crate::{
    Result,
    wire::{self, Message},
};
use quinn::Connection;
use theia_protocol::replica::{Replica, Snapshot, Source};
pub struct Client {
    replica: Replica,
    source: Source,
    samples: u64,
}
impl Client {
    pub fn new() -> Result<Self> {
        let mut replica = Replica::new(2)?;
        let source = replica.connect()?;
        Ok(Self {
            replica,
            source,
            samples: 0,
        })
    }
    pub async fn receive(&mut self, connection: &Connection) -> Result<()> {
        let message = wire::decode(&connection.read_datagram().await?)?;
        let Message::Snapshot {
            tick,
            applied_move,
            actors,
        } = message
        else {
            return Err("unexpected server DATAGRAM kind".into());
        };
        self.replica.publish(
            self.source,
            Snapshot {
                tick,
                applied_move,
                actors: actors.to_vec(),
            },
        )?;
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
    pub fn samples(&self) -> u64 {
        self.samples
    }
    pub fn latest(&self) -> Option<&Snapshot> {
        self.replica.latest()
    }
    pub fn disconnect(&mut self) -> Result<()> {
        Ok(self.replica.disconnect(self.source)?)
    }
}
