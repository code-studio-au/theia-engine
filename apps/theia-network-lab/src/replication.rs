//! Isolated encrypted TDR1 qualification. Synthetic complete-state source, not
//! gameplay/roster authority or a production service. THN2 labs stay unchanged.
use crate::{Result, connect_at};
use quinn::Connection;
use std::{net::SocketAddr, time::Duration};
use theia_network_fixture::{
    proxy,
    replication_wire::{self as wire, Message},
    tls,
};
use theia_protocol::{
    replica::{self, Publication, Replica, Snapshot},
    replication::{Packet, Sender},
};
use theia_sim::{ActorState, WorldId};
const FINAL: u64 = 120;
fn reference(tick: u64) -> Snapshot {
    Snapshot {
        tick,
        applied_move: Some(tick),
        actors: vec![
            ActorState {
                id: WorldId::new(10).unwrap(),
                position_mm: [-1000 + (tick * 3) as i32, -1000],
                health: 100 - (tick % 5) as u32 * 10,
            },
            ActorState {
                id: WorldId::new(20).unwrap(),
                position_mm: [1000, -1000],
                health: if tick < 60 { 100 } else { 75 },
            },
        ],
    }
}
#[derive(Default, Debug)]
struct ServerReport {
    full: u64,
    delta: u64,
    acks: u64,
    repairs: u64,
    invalid_controls: u64,
}
#[derive(Default, Debug)]
struct ClientReport {
    full: u64,
    delta: u64,
    missing: u64,
    new: u64,
    duplicate: u64,
    stale: u64,
}
async fn server(connections: [Connection; 2]) -> Result<ServerReport> {
    let mut senders = [Sender::new(2, 8)?, Sender::new(2, 8)?];
    let recipients = [senders[0].connect()?, senders[1].connect()?];
    let mut report = ServerReport::default();
    let mut initial = [false; 2];
    let mut final_ack = [false; 2];
    let mut clock = tokio::time::interval(Duration::from_millis(50));
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut tick = 0;
    loop {
        let control = tokio::select! {
            _=clock.tick()=> {
                if tick==0 || (initial.iter().all(|v|*v) && tick<FINAL) {tick+=1;}
                for i in 0..2 {
                    let packet=senders[i].prepare(recipients[i],reference(tick))?;
                    let message=match packet {Packet::Full(s)=>{report.full+=1;Message::Full(s)},Packet::Delta(d)=>{report.delta+=1;Message::Delta(d)}};
                    connections[i].send_datagram(wire::encode(&message)?.into())?;
                    assert!(senders[i].retained_count()<=8);
                }
                continue;
            }
            value=connections[0].read_datagram()=> (0,value?),
            value=connections[1].read_datagram()=> (1,value?),
        };
        let (i, bytes) = control;
        match wire::decode(&bytes) {
            Ok(Message::Ack(ack)) => match senders[i].acknowledge(recipients[i], ack) {
                Ok(_) => {
                    report.acks += 1;
                    if ack == 1 {
                        initial[i] = true;
                    }
                    if ack == FINAL && tick == FINAL {
                        final_ack[i] = true;
                    }
                }
                Err(_) => report.invalid_controls += 1,
            },
            Ok(Message::Repair) => {
                senders[i].request_full(recipients[i])?;
                report.repairs += 1;
            }
            _ => report.invalid_controls += 1,
        }
        if final_ack.iter().all(|v| *v) {
            break;
        }
    }
    assert!(report.full > 0 && report.delta > 0 && report.repairs > 0);
    for connection in connections {
        connection.close(0u8.into(), b"replication verified");
    }
    Ok(report)
}
async fn client(connection: Connection) -> Result<ClientReport> {
    let mut replica = Replica::new(2)?;
    let source = replica.connect()?;
    let mut report = ClientReport::default();
    loop {
        let bytes = match connection.read_datagram().await {
            Ok(bytes) => bytes,
            Err(error) => {
                if replica.latest() != Some(&reference(FINAL)) {
                    return Err(format!("Closed before final complete state: {error}").into());
                }
                break;
            }
        };
        let result = match wire::decode(&bytes)? {
            Message::Full(snapshot) => {
                if snapshot.tick > FINAL || snapshot != reference(snapshot.tick) {
                    return Err("Complete state differs from independent reference".into());
                }
                report.full += 1;
                replica.publish(source, snapshot)
            }
            Message::Delta(delta) => {
                if delta.tick > FINAL {
                    return Err("Delta tick exceeds fixture range".into());
                }
                report.delta += 1;
                replica.publish_delta(source, &delta)
            }
            _ => return Err("Client received a control message as state".into()),
        };
        match result {
            Ok(publication) => {
                match publication {
                    Publication::New => report.new += 1,
                    Publication::Duplicate => report.duplicate += 1,
                    Publication::Stale => report.stale += 1,
                }
                let snapshot = replica.latest().ok_or("No complete replica")?;
                if snapshot != &reference(snapshot.tick) {
                    return Err("Reconstruction differs from independent complete state".into());
                }
                // Deliberately withhold early application ACKs so the receiver's
                // two-set eviction outruns an acknowledged sender baseline.
                if !(2..=6).contains(&snapshot.tick) {
                    connection.send_datagram(wire::encode(&Message::Ack(snapshot.tick))?.into())?;
                }
            }
            Err(replica::Error::MissingBaseline) => {
                report.missing += 1;
                connection.send_datagram(wire::encode(&Message::Repair)?.into())?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    assert!(report.full > 0 && report.delta > 0 && report.missing > 0 && report.new > 0);
    replica.disconnect(source)?;
    assert!(replica.latest().is_none() && replica.previous().is_none());
    Ok(report)
}
pub async fn qualify(ipv6: bool, faults: bool) -> Result<()> {
    let bind: SocketAddr = if ipv6 { "[::1]:0" } else { "127.0.0.1:0" }.parse()?;
    let (server_endpoint, config, _) = tls::endpoints(bind)?;
    let mut clients = [
        quinn::Endpoint::client(bind)?,
        quinn::Endpoint::client(bind)?,
    ];
    for endpoint in &mut clients {
        endpoint.set_default_client_config(config.clone());
    }
    let relays = if faults {
        Some([
            proxy::Relay::new(server_endpoint.local_addr()?, clients[0].local_addr()?).await?,
            proxy::Relay::new(server_endpoint.local_addr()?, clients[1].local_addr()?).await?,
        ])
    } else {
        None
    };
    let destinations = relays
        .as_ref()
        .map_or([server_endpoint.local_addr()?; 2], |r| {
            [r[0].address, r[1].address]
        });
    let (first, server_first) =
        connect_at(&server_endpoint, &clients[0], "localhost", destinations[0]).await?;
    let (second, server_second) =
        connect_at(&server_endpoint, &clients[1], "localhost", destinations[1]).await?;
    if let Some(relays) = &relays {
        for relay in relays {
            relay.enable(true);
        }
    }
    let result=async {
        let (server_report,first_report,second_report)=tokio::try_join!(server([server_first,server_second]),client(first),client(second))?;
        println!("Verified encrypted TDR1 two-client replication through final tick {FINAL}: server {server_report:?}; clients {first_report:?} / {second_report:?}; exact complete-state oracle, explicit ACK baselines, missing-baseline full repair, bounded histories and disconnect clearing.");
        Ok::<(),Box<dyn std::error::Error+Send+Sync>>(())
    }.await;
    if let Some(relays) = &relays {
        for relay in relays {
            relay.enable(false);
        }
    }
    server_endpoint.close(0u8.into(), b"done");
    for endpoint in &clients {
        endpoint.close(0u8.into(), b"done");
    }
    let _ = tokio::time::timeout(Duration::from_secs(1), async {
        tokio::join!(
            server_endpoint.wait_idle(),
            clients[0].wait_idle(),
            clients[1].wait_idle()
        );
    })
    .await;
    if let Some(relays) = relays {
        let mut drops = 0;
        let mut reorders = 0;
        let mut duplicates = 0;
        for relay in relays {
            let r = relay.finish().await?;
            assert!(r.max_pending <= 2);
            drops += r.dropped;
            reorders += r.reordered;
            duplicates += r.duplicated;
            println!("Encrypted replication relay: {r:?}");
        }
        assert!(drops > 0 && reorders > 0 && duplicates > 0);
    }
    result?;
    println!(
        "Synthetic replication fixture only; certificate-verified TLS/ALPN, no account identity, gameplay changes, production feature negotiation, viewer adoption, persistence or performance claim."
    );
    Ok(())
}
