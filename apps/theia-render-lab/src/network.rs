//! Bounded viewer transport worker. No local World/health/movement authority.
use crate::{Result, authority};
use quinn::{Connection, Endpoint};
use std::{
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use theia_network_fixture::{
    client, proxy, tls,
    wire::{self, Message, Profile, Reject, Welcome},
};
use theia_protocol::{Outcome, Reply};
use theia_sim::{ActorState, AttackInput, MoveInput};
use tokio::{sync::oneshot, task::JoinSet};
#[derive(Default)]
struct Controls {
    keys: [bool; 8],
    attack: bool,
}
struct View {
    status: String,
    welcome: Option<Welcome>,
    previous: Vec<ActorState>,
    current: Vec<ActorState>,
    previous_tick: Option<u64>,
    tick: u64,
    received: Instant,
    snapshots: u64,
    outcomes: u64,
    replication: client::ReplicationStats,
}
impl View {
    fn new() -> Self {
        Self {
            status: "Connecting".into(),
            welcome: None,
            previous: vec![],
            current: vec![],
            previous_tick: None,
            tick: 0,
            received: Instant::now(),
            snapshots: 0,
            outcomes: 0,
            replication: client::ReplicationStats::default(),
        }
    }
    fn closed(&mut self, message: &str) {
        self.status = message.chars().take(512).collect();
        self.welcome = None;
        self.previous.clear();
        self.current.clear();
        self.previous_tick = None;
    }
}
pub struct Viewer {
    controls: Arc<Mutex<Controls>>,
    view: Arc<Mutex<View>>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Viewer {
    pub fn new(
        address: SocketAddr,
        certificate: &Path,
        faults: bool,
        repair_test: bool,
    ) -> Result<Self> {
        use std::io::Read;
        if !address.ip().is_loopback() {
            return Err("Network viewer fixture requires loopback".into());
        }
        let mut der = Vec::new();
        std::fs::File::open(certificate)?
            .take(65537)
            .read_to_end(&mut der)?;
        let config = tls::client_from_der(&der).map_err(|e| e.to_string())?;
        let controls = Arc::new(Mutex::new(Controls::default()));
        let view = Arc::new(Mutex::new(View::new()));
        let (stop_tx, stop_rx) = oneshot::channel();
        let worker_controls = controls.clone();
        let worker_view = view.clone();
        let thread = thread::Builder::new()
            .name("theia-viewer-network".into())
            .spawn(move || {
                let result: theia_network_fixture::Result<()> = (|| {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?;
                    runtime.block_on(async {
                        let mut endpoint = Endpoint::client(SocketAddr::new(address.ip(), 0))?;
                        endpoint.set_default_client_config(config);
                        run(
                            &endpoint,
                            address,
                            worker_controls,
                            worker_view.clone(),
                            faults,
                            repair_test,
                            stop_rx,
                        )
                        .await
                    })
                })();
                let message = match result {
                    Ok(()) => "Connection closed".into(),
                    Err(e) => format!("Connection closed: {e}"),
                };
                if let Ok(mut view) = worker_view.lock() {
                    view.closed(&message);
                }
                println!("{message}");
            })?;
        Ok(Self {
            controls,
            view,
            stop: Some(stop_tx),
            thread: Some(thread),
        })
    }
    pub fn direction(&self, binding: usize, pressed: bool) {
        if let Ok(mut controls) = self.controls.lock()
            && let Some(key) = controls.keys.get_mut(binding)
        {
            *key = pressed;
        }
    }
    pub fn attack(&self) {
        if let Ok(mut controls) = self.controls.lock() {
            controls.attack = true;
        }
    }
    pub fn release_input(&self) {
        if let Ok(mut controls) = self.controls.lock() {
            *controls = Controls::default();
        }
    }
    pub fn push(&self) -> Result<([u8; 64], String)> {
        let view = self.view.lock().map_err(|_| "Viewer state poisoned")?;
        let mut bytes = [0; 64];
        // An unknown/closed connection supplies no actor identity; shader drops zero-ID groups.
        if !view.current.is_empty() {
            let phase = (view.received.elapsed().as_secs_f64() / 0.05).clamp(0.0, 1.0) as f32;
            let alpha = match view.previous_tick {
                Some(previous) if view.tick > previous && view.tick - previous <= 4 => {
                    ((view.tick - previous - 1) as f32 + phase) / (view.tick - previous) as f32
                }
                _ => 1.0,
            };
            bytes = authority::pack(&view.previous, &view.current, alpha)?;
        }
        if view
            .welcome
            .is_none_or(|v| v.profile == Profile::Laboratory)
        {
            let flags = u32::from_ne_bytes(bytes[60..64].try_into().unwrap()) | (1 << 31);
            bytes[60..64].copy_from_slice(&flags.to_ne_bytes());
        }
        Ok((bytes, view.status.clone()))
    }
    pub fn verify(&self) -> Result<()> {
        let view = self.view.lock().map_err(|_| "Viewer state poisoned")?;
        if view.snapshots == 0 {
            return Err(format!("No verified network snapshots received: {}", view.status).into());
        }
        println!(
            "Network viewer: {} readonly snapshots, {} acknowledged outcomes; latest observed server tick {}; no local authority",
            view.snapshots, view.outcomes, view.tick
        );
        println!("Live replication: {:?}", view.replication);
        Ok(())
    }
}
impl Drop for Viewer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
async fn exchange(
    connection: &Connection,
    message: &Message,
) -> theia_network_fixture::Result<Message> {
    tokio::time::timeout(Duration::from_secs(2), async {
        let (mut send, mut receive) = connection.open_bi().await?;
        send.write_all(&wire::encode(message)).await?;
        send.finish()?;
        Ok(wire::decode(&receive.read_to_end(wire::MAX_BYTES).await?)?)
    })
    .await?
}
async fn request(
    connection: Connection,
    welcome: Welcome,
    sequence: u64,
) -> theia_network_fixture::Result<Outcome> {
    tokio::time::timeout(Duration::from_secs(3), async {
        let outcome = loop {
            let reply = exchange(
                &connection,
                &Message::Attack(AttackInput {
                    sequence,
                    target: welcome.target,
                }),
            )
            .await?;
            match reply {
                Message::Response {
                    request,
                    reply: Ok(Reply::Completed(outcome)),
                } if request == sequence => break outcome,
                Message::Response {
                    request,
                    reply: Ok(Reply::Queued | Reply::Pending) | Err(Reject::Budget),
                } if request == sequence => tokio::time::sleep(Duration::from_millis(10)).await,
                other => return Err(format!("Unexpected gameplay response {other:?}").into()),
            }
        };
        if outcome.attacker != welcome.actor || outcome.target != welcome.target {
            return Err("Outcome identity differs from bootstrap".into());
        }
        if exchange(&connection, &Message::Ack(sequence)).await?
            != (Message::Response {
                request: sequence,
                reply: Ok(Reply::Acknowledged),
            })
        {
            return Err("Outcome acknowledgement failed".into());
        }
        Ok(outcome)
    })
    .await?
}
async fn run(
    endpoint: &Endpoint,
    address: SocketAddr,
    controls: Arc<Mutex<Controls>>,
    view: Arc<Mutex<View>>,
    faults: bool,
    repair_test: bool,
    mut stop: oneshot::Receiver<()>,
) -> theia_network_fixture::Result<()> {
    let relay = if faults {
        Some(proxy::Relay::new(address, endpoint.local_addr()?).await?)
    } else {
        None
    };
    let destination = relay.as_ref().map_or(address, |relay| relay.address);
    let result = tokio::select! {
        result = connected(endpoint, destination, controls, view.clone(), relay.as_ref(), repair_test) => result,
        _ = &mut stop => Ok(()),
    };
    if let Ok(mut view) = view.lock() {
        view.closed("Connection closed");
    }
    if let Some(relay) = &relay {
        relay.enable(false);
    }
    endpoint.close(0u8.into(), b"viewer ended");
    // Keep the transparent relay alive during bounded QUIC close draining.
    let _ = tokio::time::timeout(Duration::from_secs(1), endpoint.wait_idle()).await;
    if let Some(relay) = relay {
        println!("Network fault relay: {:?}", relay.finish().await?);
    }
    result
}
async fn connected(
    endpoint: &Endpoint,
    destination: SocketAddr,
    controls: Arc<Mutex<Controls>>,
    view: Arc<Mutex<View>>,
    relay: Option<&proxy::Relay>,
    repair_test: bool,
) -> theia_network_fixture::Result<()> {
    let connection = tokio::time::timeout(
        Duration::from_secs(5),
        endpoint.connect(destination, "localhost")?,
    )
    .await??;
    view.lock().map_err(|_| "Viewer state poisoned")?.status = "Waiting for second client".into();
    let welcome = client::receive_welcome(&connection).await?;
    let mut replica = client::Client::new()?;
    replica.bind(&connection, welcome)?;
    if repair_test {
        replica.qualify_baseline_eviction()?;
        println!("Qualification: withholding snapshot ACKs until missing-baseline full repair");
    }
    {
        let mut view = view.lock().map_err(|_| "Viewer state poisoned")?;
        view.welcome = Some(welcome);
        view.status = format!(
            "Player {} — WASD/arrows, Space attack",
            welcome.actor.value()
        );
    }
    println!(
        "Verified server assigned actor {}, target {}, scene {:?}",
        welcome.actor.value(),
        welcome.target.value(),
        welcome.profile
    );
    if let Some(relay) = &relay {
        relay.enable(true);
        println!("Network faults enabled after verified bootstrap");
    }
    let mut clock = tokio::time::interval(Duration::from_millis(50));
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut movement = 0u64;
    let mut next_request = 0u64;
    let mut requests = JoinSet::new();
    let result: theia_network_fixture::Result<()>=async {
        loop {
            tokio::select! {
                _=clock.tick()=> {
                    let (axes,attack)={
                        let mut controls=controls.lock().map_err(|_| "Viewer controls poisoned")?;
                        let axes=[(i16::from(controls.keys[1]||controls.keys[5])-i16::from(controls.keys[0]||controls.keys[4]))*1000,
                            (i16::from(controls.keys[3]||controls.keys[7])-i16::from(controls.keys[2]||controls.keys[6]))*1000];
                        let attack=controls.attack && requests.is_empty();if attack { controls.attack=false; } (axes,attack)
                    };
                    let alive=replica.latest().and_then(|v| v.actors.iter().find(|a|a.id==welcome.actor)).is_none_or(|v|v.health!=0);
                    if alive {
                        let next=movement.checked_add(1).ok_or("Movement sequence exhausted")?;
                        connection.send_datagram(wire::encode(&Message::Move(MoveInput {sequence:movement,axes})).into())?;movement=next;
                    }
                    if attack { let next=next_request.checked_add(1).ok_or("Request sequence exhausted")?;requests.spawn(request(connection.clone(),welcome,next_request));next_request=next; }
                }
                received=replica.receive(&connection)=> {
                    received?;
                    let Some(current)=replica.latest() else {continue;};
                    let mut view=view.lock().map_err(|_| "Viewer state poisoned")?;
                    view.replication=replica.replication_stats();
                    if view.current.is_empty() || current.tick>view.tick {
                        view.previous=replica.previous().map_or_else(Vec::new,|v|v.actors.clone());
                        view.previous_tick=replica.previous().map(|v|v.tick);view.current=current.actors.clone();view.tick=current.tick;
                        view.received=Instant::now();view.snapshots+=1;
                    }
                }
                result=requests.join_next(),if !requests.is_empty()=> {
                    let outcome=result.ok_or("Missing request task")???;
                    view.lock().map_err(|_| "Viewer state poisoned")?.outcomes+=1;
                    println!("Server outcome request {} tick {}: {:?}",outcome.request,outcome.tick,outcome.result);
                }
            }
        }
    }.await;
    requests.abort_all();
    while requests.join_next().await.is_some() {}
    println!("Live replication: {:?}", replica.replication_stats());
    replica.disconnect()?;
    connection.close(0u8.into(), b"viewer disconnected");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use theia_sim::WorldId;
    fn viewer() -> Viewer {
        Viewer {
            controls: Arc::new(Mutex::new(Controls::default())),
            view: Arc::new(Mutex::new(View::new())),
            stop: None,
            thread: None,
        }
    }
    #[test]
    fn empty_closed_and_profile_state_never_fabricate_actor_or_obstacle_identity() {
        let viewer = viewer();
        let (empty, _) = viewer.push().unwrap();
        assert!(empty[32..48].iter().all(|v| *v == 0));
        assert_eq!(
            u32::from_ne_bytes(empty[60..64].try_into().unwrap()),
            1 << 31
        );
        let mut view = viewer.view.lock().unwrap();
        let a = WorldId::new(10).unwrap();
        let b = WorldId::new(20).unwrap();
        view.welcome = Some(Welcome {
            actor: a,
            target: b,
            profile: Profile::LocalDemo,
        });
        view.current = vec![
            ActorState {
                id: a,
                position_mm: [-1000, 0],
                health: 100,
            },
            ActorState {
                id: b,
                position_mm: [1000, 0],
                health: 75,
            },
        ];
        view.previous = view.current.clone();
        view.previous_tick = Some(1);
        view.tick = 10; // Long gap snaps latest.
        drop(view);
        let (bytes, _) = viewer.push().unwrap();
        assert_eq!(f32::from_ne_bytes(bytes[56..60].try_into().unwrap()), 1.0);
        assert_eq!(
            u32::from_ne_bytes(bytes[60..64].try_into().unwrap()) >> 31,
            0
        );
        viewer.view.lock().unwrap().closed("fixture closed");
        let (bytes, status) = viewer.push().unwrap();
        assert!(bytes[32..48].iter().all(|v| *v == 0));
        assert_eq!(status, "fixture closed");
    }
    #[test]
    fn pending_attack_is_bounded_and_focus_release_clears_all_aliases() {
        let viewer = viewer();
        viewer.direction(1, true);
        viewer.direction(5, true);
        viewer.attack();
        viewer.attack();
        assert!(viewer.controls.lock().unwrap().attack);
        viewer.direction(1, false);
        assert!(viewer.controls.lock().unwrap().keys[5]);
        viewer.release_input();
        let controls = viewer.controls.lock().unwrap();
        assert!(!controls.attack);
        assert!(controls.keys.iter().all(|v| !*v));
    }
}
