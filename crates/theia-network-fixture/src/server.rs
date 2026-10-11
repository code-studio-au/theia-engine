use crate::{
    Result,
    wire::{self, Message},
};
use quinn::Connection;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use theia_protocol::{Authority, Limits, Peer, Reply};
use theia_sim::{Rect, Rules, Scene, WorldId};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
    time::{MissedTickBehavior, timeout},
};
struct Event {
    peer: Peer,
    message: Message,
    response: Option<oneshot::Sender<Message>>,
}
#[derive(Debug, Default)]
pub struct Report {
    pub ticks: u64,
    pub movements: u64,
    pub hits: u64,
    pub malformed: u64,
    pub rejected_moves: u64,
    pub health: [u32; 2],
}
async fn read_peer(
    connection: Connection,
    peer: Peer,
    events: mpsc::Sender<Event>,
    malformed: Arc<AtomicU64>,
) {
    let mut streams = JoinSet::new();
    loop {
        tokio::select! {
            incoming = connection.accept_bi(), if streams.len() < 4 => {
                let Ok((mut send, mut recv)) = incoming else { break };
                let events = events.clone(); let malformed = malformed.clone();
                streams.spawn(async move {
                    let operation = async {
                        let bytes = recv.read_to_end(wire::MAX_BYTES).await?;
                        let message = wire::decode(&bytes)?;
                        if !matches!(message, Message::Attack(_) | Message::Ack(_)) { return Err("wrong reliable message kind".into()); }
                        let (tx, rx) = oneshot::channel();
                        events.send(Event { peer, message, response: Some(tx) }).await.map_err(|_| "authority stopped")?;
                        let response = rx.await?;
                        send.write_all(&wire::encode(&response)).await?; send.finish()?;
                        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                    };
                    let result = timeout(Duration::from_secs(1), operation).await;
                    if !matches!(result, Ok(Ok(()))) { malformed.fetch_add(1, Ordering::Relaxed); let _ = recv.stop(1u8.into()); let _ = send.reset(1u8.into()); }
                });
            }
            datagram = connection.read_datagram() => {
                let Ok(bytes) = datagram else { break };
                match wire::decode(&bytes) {
                    Ok(message @ Message::Move(_)) => { let _ = events.try_send(Event { peer, message, response: None }); }
                    _ => { malformed.fetch_add(1, Ordering::Relaxed); }
                }
            }
            _ = streams.join_next(), if !streams.is_empty() => {}
        }
    }
    streams.abort_all();
    while streams.join_next().await.is_some() {}
}
pub async fn run(connections: [Connection; 2], mut stop: oneshot::Receiver<()>) -> Result<Report> {
    let scene = Scene::new(
        Rect {
            min: [-4000; 2],
            max: [4000; 2],
        },
        vec![],
    )?;
    let mut authority = Authority::new(
        scene,
        Rules::default(),
        Limits {
            max_peers: 2,
            request_window: 4,
        },
    )?;
    let peers = [
        authority.attach_actor(WorldId::new(10)?, [-1000, 0])?,
        authority.attach_actor(WorldId::new(20)?, [1000, 0])?,
    ];
    let (tx, mut rx) = mpsc::channel(32);
    let malformed = Arc::new(AtomicU64::new(0));
    let mut workers = JoinSet::new();
    for i in 0..2 {
        workers.spawn(read_peer(
            connections[i].clone(),
            peers[i],
            tx.clone(),
            malformed.clone(),
        ));
    }
    drop(tx);
    let mut clock = tokio::time::interval(Duration::from_millis(50));
    clock.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut report = Report::default();
    let result: Result<()> = async {
        loop {
            tokio::select! {
                _ = &mut stop => break,
                event = rx.recv() => {
                    let Some(event) = event else { return Err("all network readers exited".into()) };
                    let response = match event.message {
                        Message::Move(input) => { match authority.movement(event.peer, input) { Ok(()) => report.movements += 1, Err(_) => report.rejected_moves += 1 }; None }
                        Message::Attack(input) => Some(Message::Response { request: input.sequence, reply: authority.attack(event.peer, input).map_err(Into::into) }),
                        Message::Ack(request) => Some(Message::Response { request, reply: authority.acknowledge(event.peer, request).map(|()| Reply::Acknowledged).map_err(Into::into) }),
                        _ => return Err("reader admitted wrong message".into()),
                    };
                    if let (Some(tx), Some(response)) = (event.response, response) { let _ = tx.send(response); }
                }
                _ = clock.tick() => {
                    let frame = authority.step()?; report.ticks = frame.simulation.tick;
                    report.hits += frame.outcomes.iter().filter(|(_, v)| matches!(v.result, theia_sim::AttackResult::Hit { .. })).count() as u64;
                    let mut actors = authority.snapshot(); actors.sort_by_key(|v| v.id);
                    let actors = [actors[0], actors[1]];
                    for i in 0..2 {
                        let snapshot = Message::Snapshot { tick: report.ticks, applied_move: authority.applied_move(peers[i])?, actors };
                        connections[i].send_datagram(wire::encode(&snapshot).into())?;
                    }
                }
            }
        }
        Ok(())
    }.await;
    workers.abort_all();
    while workers.join_next().await.is_some() {}
    report.malformed = malformed.load(Ordering::Relaxed);
    report.health = [
        authority.actor(WorldId::new(10)?).unwrap().health,
        authority.actor(WorldId::new(20)?).unwrap().health,
    ];
    result?;
    Ok(report)
}
