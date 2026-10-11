//! Connection-scoped readonly replica for the bounded full-snapshot wire profile.
use crate::{
    Result,
    wire::{self, Message, Welcome},
};
use quinn::Connection;
use theia_protocol::replica::{Replica, Snapshot, Source};
pub struct Client {
    replica: Replica,
    source: Source,
    samples: u64,
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
            connection: None,
            welcome: None,
        })
    }
    pub fn bind(&mut self, connection: &Connection, welcome: Welcome) -> Result<()> {
        welcome.validate()?;
        self.source = self.replica.connect()?;
        self.connection = Some(connection.clone());
        self.welcome = Some(welcome);
        Ok(())
    }
    pub async fn receive(&mut self, connection: &Connection) -> Result<()> {
        if self.connection.as_ref().map(Connection::stable_id) != Some(connection.stable_id()) {
            return Err("Snapshot delivered from an unbound connection".into());
        }
        let message = wire::decode(&connection.read_datagram().await?)?;
        let Message::Snapshot {
            tick,
            applied_move,
            actors,
        } = message
        else {
            return Err("unexpected server DATAGRAM kind".into());
        };
        let welcome = self.welcome.ok_or("Missing server welcome")?;
        if actors.iter().all(|v| v.id != welcome.actor)
            || actors.iter().all(|v| v.id != welcome.target)
        {
            return Err("Snapshot roster differs from bootstrap".into());
        }
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
    pub fn previous(&self) -> Option<&Snapshot> {
        self.replica.previous()
    }
    pub fn latest(&self) -> Option<&Snapshot> {
        self.replica.latest()
    }
    pub fn disconnect(&mut self) -> Result<()> {
        self.replica.disconnect(self.source)?;
        self.connection = None;
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
