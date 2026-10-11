//! Bounded qualification-only UDP relay. Never decrypts/rewrites QUIC packets.
use crate::Result;
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    net::UdpSocket,
    sync::oneshot,
    task::JoinHandle,
    time::{Instant, MissedTickBehavior},
};
const MAX_PACKET: usize = 2048;
#[derive(Debug, Default)]
pub struct Report {
    pub forwarded: u64,
    pub dropped: u64,
    pub delayed: u64,
    pub reordered: u64,
    pub duplicated: u64,
    pub max_pending: usize,
}
pub struct Relay {
    pub address: SocketAddr,
    enabled: Arc<AtomicBool>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<Result<Report>>>,
}
impl Relay {
    pub async fn new(server: SocketAddr, client: SocketAddr) -> Result<Self> {
        let bind = SocketAddr::new(server.ip(), 0);
        let socket = UdpSocket::bind(bind).await?;
        let address = socket.local_addr()?;
        let enabled = Arc::new(AtomicBool::new(false));
        let (tx, rx) = oneshot::channel();
        let task = tokio::spawn(run(socket, server, client, enabled.clone(), rx));
        Ok(Self {
            address,
            enabled,
            stop: Some(tx),
            task: Some(task),
        })
    }
    pub fn enable(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
    pub async fn finish(mut self) -> Result<Report> {
        if let Some(tx) = self.stop.take() {
            let _ = tx.send(());
        }
        self.task.take().ok_or("relay task already taken")?.await?
    }
}
impl Drop for Relay {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
struct Pending {
    bytes: Vec<u8>,
    destination: SocketAddr,
    deadline: Instant,
}
async fn run(
    socket: UdpSocket,
    server: SocketAddr,
    client: SocketAddr,
    enabled: Arc<AtomicBool>,
    mut stop: oneshot::Receiver<()>,
) -> Result<Report> {
    let mut buffer = [0u8; MAX_PACKET + 1];
    let mut counts = [0u64; 2];
    let mut pending: [Option<Pending>; 2] = [None, None];
    let mut report = Report::default();
    let mut clock = tokio::time::interval(Duration::from_millis(2));
    clock.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = &mut stop => break,
            packet = socket.recv_from(&mut buffer) => {
                let (length, source) = packet?;
                let (direction, destination) = if source == client { (0, server) } else if source == server { (1, client) } else { continue };
                if length > MAX_PACKET { report.dropped += 1; continue; }
                counts[direction] = counts[direction].checked_add(1).ok_or("packet count exhausted")?;
                let count = counts[direction]; let faults = enabled.load(Ordering::Relaxed);
                if faults && count % 11 == 0 { report.dropped += 1; continue; }
                if faults && pending[direction].is_none() && count % 7 == 0 {
                    pending[direction] = Some(Pending { bytes: buffer[..length].to_vec(), destination, deadline: Instant::now() + Duration::from_millis(10) });
                    report.delayed += 1; report.max_pending = report.max_pending.max(pending.iter().filter(|p| p.is_some()).count()); continue;
                }
                socket.send_to(&buffer[..length], destination).await?; report.forwarded += 1;
                if faults && count % 13 == 0 { socket.send_to(&buffer[..length], destination).await?; report.duplicated += 1; report.forwarded += 1; }
                if let Some(held) = pending[direction].take() { socket.send_to(&held.bytes, held.destination).await?; report.forwarded += 1; report.reordered += 1; }
            }
            _ = clock.tick() => {
                let now = Instant::now();
                for held in &mut pending {
                    if held.as_ref().is_some_and(|v| v.deadline <= now) {
                        let packet = held.take().unwrap(); socket.send_to(&packet.bytes, packet.destination).await?; report.forwarded += 1;
                    }
                }
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn transparent_routes_reject_unknown_sources_and_oversize_without_growth() {
        let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let foreign = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let relay = Relay::new(server.local_addr().unwrap(), client.local_addr().unwrap())
            .await
            .unwrap();
        foreign
            .send_to(b"untrusted source", relay.address)
            .await
            .unwrap();
        client
            .send_to(&[0; MAX_PACKET + 2], relay.address)
            .await
            .unwrap();
        client
            .send_to(b"client bytes", relay.address)
            .await
            .unwrap();
        let mut bytes = [0; MAX_PACKET + 2];
        let (length, source) =
            tokio::time::timeout(Duration::from_secs(1), server.recv_from(&mut bytes))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(&bytes[..length], b"client bytes");
        assert_eq!(source, relay.address);
        server
            .send_to(b"server bytes", relay.address)
            .await
            .unwrap();
        let (length, source) =
            tokio::time::timeout(Duration::from_secs(1), client.recv_from(&mut bytes))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(&bytes[..length], b"server bytes");
        assert_eq!(source, relay.address);
        let report = relay.finish().await.unwrap();
        assert_eq!(report.forwarded, 2);
        assert_eq!(report.dropped, 1);
        assert_eq!(report.max_pending, 0);
    }
}
