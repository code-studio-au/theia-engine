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
}
impl Client {
    pub fn new() -> Result<Self> {
        let mut replica = Replica::new(2)?;
        let source = replica.connect()?;
        Ok(Self { replica, source })
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
        Ok(())
    }
    pub fn latest(&self) -> Option<&Snapshot> {
        self.replica.latest()
    }
    pub fn disconnect(&mut self) -> Result<()> {
        Ok(self.replica.disconnect(self.source)?)
    }
}
