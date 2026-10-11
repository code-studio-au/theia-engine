use crate::{
    Result, replication_wire,
    wire::{self, Message, Profile, Welcome},
};
use quinn::Connection;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use theia_protocol::{
    Authority, Limits, Peer, Reply,
    replica::Snapshot,
    replication::{Packet, Sender},
};
use theia_sim::{Rect, Rules, Scene, WorldId};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
    time::{MissedTickBehavior, timeout},
};
enum Payload {
    Gameplay(Message),
    Replication(replication_wire::Message),
}
struct Event {
    peer: Peer,
    message: Payload,
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
    pub full_snapshots: u64,
    pub delta_snapshots: u64,
    pub snapshot_acks: u64,
    pub full_repairs: u64,
    pub rejected_snapshot_acks: u64,
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
                        events.send(Event { peer, message: Payload::Gameplay(message), response: Some(tx) }).await.map_err(|_| "authority stopped")?;
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
                let payload = if bytes.starts_with(b"TDR1") {
                    match replication_wire::decode(&bytes) {
                        Ok(message @ (replication_wire::Message::Ack(_) | replication_wire::Message::Repair)) => Some(Payload::Replication(message)),
                        _ => None,
                    }
                } else {
                    match wire::decode(&bytes) { Ok(message @ Message::Move(_)) => Some(Payload::Gameplay(message)), _ => None }
                };
                if let Some(message) = payload { let _ = events.try_send(Event { peer, message, response: None }); }
                else { malformed.fetch_add(1, Ordering::Relaxed); }
            }
            _ = streams.join_next(), if !streams.is_empty() => {}
        }
    }
    streams.abort_all();
    while streams.join_next().await.is_some() {}
}
pub async fn run(connections: [Connection; 2], stop: oneshot::Receiver<()>) -> Result<Report> {
    run_profile(connections, stop, Profile::Laboratory).await
}
pub async fn run_profile(
    connections: [Connection; 2],
    mut stop: oneshot::Receiver<()>,
    profile: Profile,
) -> Result<Report> {
    if connections[0].stable_id() == connections[1].stable_id() {
        return Err("Fixture pair requires distinct connections".into());
    }
    let scene = Scene::new(
        Rect {
            min: [-4000; 2],
            max: [4000; 2],
        },
        if profile == Profile::LocalDemo {
            vec![Rect {
                min: [-100; 2],
                max: [100; 2],
            }]
        } else {
            vec![]
        },
    )?;
    let mut authority = Authority::new(
        scene,
        Rules::default(),
        Limits {
            max_peers: 2,
            request_window: 4,
        },
    )?;
    let y = if profile == Profile::LocalDemo {
        -1000
    } else {
        0
    };
    let peers = [
        authority.attach_actor(WorldId::new(10)?, [-1000, y])?,
        authority.attach_actor(WorldId::new(20)?, [1000, y])?,
    ];
    for (i, connection) in connections.iter().enumerate() {
        timeout(Duration::from_secs(2), async {
            let (mut send, mut receive) = connection.open_bi().await?;
            let welcome = Welcome {
                actor: WorldId::new(if i == 0 { 10 } else { 20 })?,
                target: WorldId::new(if i == 0 { 20 } else { 10 })?,
                profile,
            };
            send.write_all(&wire::encode(&Message::Welcome(welcome)))
                .await?;
            send.finish()?;
            if !receive.read_to_end(0).await?.is_empty() {
                return Err("Unexpected welcome acknowledgement payload".into());
            }
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        })
        .await??;
    }
    let mut senders = [Sender::new(2, 8)?, Sender::new(2, 8)?];
    let recipients = [senders[0].connect()?, senders[1].connect()?];
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
        'serving: loop {
            tokio::select! {
                _ = &mut stop => break,
                event = rx.recv() => {
                    let Some(event) = event else { break };
                    let response = match event.message {
                        Payload::Replication(message) => {
                            let index = peers.iter().position(|peer| *peer == event.peer).ok_or("Unknown replication peer")?;
                            match message {
                                replication_wire::Message::Ack(tick) => match senders[index].acknowledge(recipients[index],tick) {Ok(_)=>report.snapshot_acks+=1,Err(_)=>report.rejected_snapshot_acks+=1},
                                replication_wire::Message::Repair => {senders[index].request_full(recipients[index])?;report.full_repairs+=1;},
                                _=>return Err("Reader admitted wrong replication control".into()),
                            }
                            None
                        }
                        Payload::Gameplay(Message::Move(input)) => { match authority.movement(event.peer, input) { Ok(()) => report.movements += 1, Err(_) => report.rejected_moves += 1 }; None }
                        Payload::Gameplay(Message::Attack(input)) => Some(Message::Response { request: input.sequence, reply: authority.attack(event.peer, input).map_err(Into::into) }),
                        Payload::Gameplay(Message::Ack(request)) => Some(Message::Response { request, reply: authority.acknowledge(event.peer, request).map(|()| Reply::Acknowledged).map_err(Into::into) }),
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
                        let state = Snapshot {tick:report.ticks, applied_move:authority.applied_move(peers[i])?, actors:actors.to_vec()};
                        let packet = senders[i].prepare(recipients[i],state)?;
                        let snapshot = match packet {Packet::Full(s)=>{report.full_snapshots+=1;replication_wire::Message::Full(s)},Packet::Delta(d)=>{report.delta_snapshots+=1;replication_wire::Message::Delta(d)}};
                        match connections[i].send_datagram(replication_wire::encode(&snapshot)?.into()) {
                            Ok(()) => {}, Err(quinn::SendDatagramError::ConnectionLost(_)) => break 'serving,
                            Err(error) => return Err(error.into()),
                        }
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
