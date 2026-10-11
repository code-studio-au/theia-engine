//! Encrypted two-client loopback qualification, not a production server/account service.
mod client;
mod proxy;
mod server;
mod tls;
mod wire;
use quinn::{Connection, Endpoint};
use std::{error::Error, net::SocketAddr, time::Duration};
use theia_protocol::{Outcome, Reply};
use theia_sim::{AttackInput, AttackReject, AttackResult, MoveInput, WorldId};
use tokio::{sync::oneshot, time::timeout};
use wire::{Message, Reject};
type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
async fn connect(
    server: &Endpoint,
    client: &Endpoint,
    hostname: &str,
) -> Result<(Connection, Connection)> {
    connect_at(server, client, hostname, server.local_addr()?).await
}
async fn connect_at(
    server: &Endpoint,
    client: &Endpoint,
    hostname: &str,
    destination: SocketAddr,
) -> Result<(Connection, Connection)> {
    let outgoing = client.connect(destination, hostname)?;
    let incoming = async {
        server
            .accept()
            .await
            .ok_or("server closed")?
            .await
            .map_err(Into::into)
    };
    let (a, b): (
        std::result::Result<Connection, quinn::ConnectionError>,
        Result<Connection>,
    ) = tokio::join!(outgoing, incoming);
    Ok((a?, b?))
}
async fn exchange(connection: &Connection, message: &Message) -> Result<Message> {
    timeout(Duration::from_secs(2), async {
        let (mut send, mut recv) = connection.open_bi().await?;
        send.write_all(&wire::encode(message)).await?;
        send.finish()?;
        Ok(wire::decode(&recv.read_to_end(wire::MAX_BYTES).await?)?)
    })
    .await?
}
async fn complete(connection: &Connection, request: u64, target: WorldId) -> Result<Outcome> {
    timeout(Duration::from_secs(2), async {
        loop {
            match exchange(
                connection,
                &Message::Attack(AttackInput {
                    sequence: request,
                    target,
                }),
            )
            .await?
            {
                Message::Response {
                    request: r,
                    reply: Ok(Reply::Completed(outcome)),
                } if r == request => return Ok(outcome),
                Message::Response {
                    request: r,
                    reply: Ok(Reply::Queued | Reply::Pending),
                } if r == request => tokio::time::sleep(Duration::from_millis(10)).await,
                response => return Err(format!("unexpected attack response {response:?}").into()),
            }
        }
    })
    .await?
}
async fn movement(
    connection: &Connection,
    client: &mut client::Client,
    input: MoveInput,
) -> Result<theia_protocol::replica::Snapshot> {
    timeout(Duration::from_secs(2), async {
        let mut retry = tokio::time::interval(Duration::from_millis(50));
        retry.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = retry.tick() => connection.send_datagram(wire::encode(&Message::Move(input)).into())?,
                received = client.receive(connection) => {
                    received?;
                    let snapshot = client.latest().ok_or("missing published snapshot")?;
                    if snapshot.applied_move.is_some_and(|v| v >= input.sequence) { return Ok(snapshot.clone()); }
                }
            }
        }
    }).await?
}
async fn qualify(ipv6: bool, faults: bool) -> Result<()> {
    let bind: SocketAddr = if ipv6 { "[::1]:0" } else { "127.0.0.1:0" }.parse()?;
    let (server_endpoint, config, negative_configs) = tls::endpoints(bind)?;
    let mut first_endpoint = Endpoint::client(bind)?;
    first_endpoint.set_default_client_config(config.clone());
    let mut second_endpoint = Endpoint::client(bind)?;
    second_endpoint.set_default_client_config(config.clone());
    assert!(
        connect(&server_endpoint, &first_endpoint, "wrong.invalid")
            .await
            .is_err()
    );
    for bad_config in negative_configs {
        first_endpoint.set_default_client_config(bad_config);
        assert!(
            connect(&server_endpoint, &first_endpoint, "localhost")
                .await
                .is_err()
        );
    }
    first_endpoint.set_default_client_config(config.clone());
    let relays = if faults {
        Some([
            proxy::Relay::new(server_endpoint.local_addr()?, first_endpoint.local_addr()?).await?,
            proxy::Relay::new(server_endpoint.local_addr()?, second_endpoint.local_addr()?).await?,
        ])
    } else {
        None
    };
    let destinations = if let Some(relays) = &relays {
        [relays[0].address, relays[1].address]
    } else {
        [server_endpoint.local_addr()?; 2]
    };
    let (first, server_first) = connect_at(
        &server_endpoint,
        &first_endpoint,
        "localhost",
        destinations[0],
    )
    .await?;
    let (second, server_second) = connect_at(
        &server_endpoint,
        &second_endpoint,
        "localhost",
        destinations[1],
    )
    .await?;
    if let Some(relays) = &relays {
        for relay in relays {
            relay.enable(true);
        }
    }
    for connection in [&first, &second] {
        assert!(
            connection
                .max_datagram_size()
                .is_some_and(|v| v >= wire::MAX_BYTES)
        );
        let data = connection
            .handshake_data()
            .ok_or("missing handshake data")?
            .downcast::<quinn::crypto::rustls::HandshakeData>()
            .map_err(|_| "unexpected crypto stack")?;
        assert_eq!(data.protocol.as_deref(), Some(b"theia-lab-1".as_slice()));
    }
    let (stop_tx, stop_rx) = oneshot::channel();
    let server = tokio::spawn(server::run([server_first, server_second], stop_rx));
    let mut first_client = client::Client::new()?;
    let mut second_client = client::Client::new()?;
    let a = WorldId::new(10)?;
    let b = WorldId::new(20)?;
    movement(
        &first,
        &mut first_client,
        MoveInput {
            sequence: 1_000_000,
            axes: [0, 0],
        },
    )
    .await?;
    movement(
        &second,
        &mut second_client,
        MoveInput {
            sequence: 2_000_000,
            axes: [0, 0],
        },
    )
    .await?;
    let (mut stalled_send, mut stalled_recv) = first.open_bi().await?;
    stalled_send.write_all(b"THN1").await?; // No FIN: this stream cannot stall the authority or another peer.
    assert_eq!(
        timeout(Duration::from_millis(600), complete(&second, 0, a))
            .await??
            .result,
        AttackResult::Rejected(AttackReject::Range)
    );
    assert_eq!(
        exchange(&second, &Message::Ack(0)).await?,
        Message::Response {
            request: 0,
            reply: Ok(Reply::Acknowledged)
        }
    );
    assert_eq!(
        complete(&first, 0, b).await?.result,
        AttackResult::Rejected(AttackReject::Range)
    );
    // Held forward motion is refreshed until the authoritative snapshot puts A in range.
    let mut sequence = 1_000_000;
    loop {
        sequence += 1;
        let theia_protocol::replica::Snapshot { actors, .. } = movement(
            &first,
            &mut first_client,
            MoveInput {
                sequence,
                axes: [1000, 0],
            },
        )
        .await?;
        if actors[1].position_mm[0] - actors[0].position_mm[0] <= 1400 {
            break;
        }
    }
    sequence += 1;
    let stop_move = Message::Move(MoveInput {
        sequence,
        axes: [0, 0],
    });
    movement(
        &first,
        &mut first_client,
        MoveInput {
            sequence,
            axes: [0, 0],
        },
    )
    .await?;
    let first_hit = complete(&first, 1, b).await?;
    assert_eq!(first_hit.result, AttackResult::Hit { health_after: 75 });
    assert_eq!(complete(&first, 1, b).await?, first_hit);
    assert_eq!(
        exchange(
            &first,
            &Message::Attack(AttackInput {
                sequence: 1,
                target: a
            })
        )
        .await?,
        Message::Response {
            request: 1,
            reply: Err(Reject::Conflict)
        }
    );
    let other_hit = complete(&second, 1, a).await?;
    assert_eq!(other_hit.attacker, b);
    assert_eq!(other_hit.result, AttackResult::Hit { health_after: 75 });
    assert_eq!(
        exchange(&first, &Message::Ack(1)).await?,
        Message::Response {
            request: 1,
            reply: Ok(Reply::Acknowledged)
        }
    );
    assert_eq!(
        exchange(
            &first,
            &Message::Attack(AttackInput {
                sequence: 4,
                target: b
            })
        )
        .await?,
        Message::Response {
            request: 4,
            reply: Err(Reject::Window)
        }
    );
    exchange(&first, &Message::Ack(0)).await?;
    assert_eq!(
        exchange(
            &first,
            &Message::Attack(AttackInput {
                sequence: 1,
                target: b
            })
        )
        .await?,
        Message::Response {
            request: 1,
            reply: Err(Reject::Stale)
        }
    );
    // Invalid stream gets reset without disrupting another request or DATAGRAM snapshot.
    let (mut send, mut recv) = first.open_bi().await?;
    send.write_all(&[0; wire::MAX_BYTES + 1]).await?;
    send.finish()?;
    assert!(
        timeout(Duration::from_secs(2), recv.read_to_end(wire::MAX_BYTES))
            .await?
            .is_err()
    );
    assert!(
        timeout(
            Duration::from_secs(2),
            stalled_recv.read_to_end(wire::MAX_BYTES)
        )
        .await?
        .is_err()
    );
    first.send_datagram(vec![0; 5].into())?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    for _ in 0..5 {
        first.send_datagram(wire::encode(&stop_move).into())?; // Superseded sequence cannot move again.
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    sequence += 1;
    movement(
        &first,
        &mut first_client,
        MoveInput {
            sequence,
            axes: [0, 0],
        },
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(30)).await;
    for (connection, client) in [(&first, &mut first_client), (&second, &mut second_client)] {
        timeout(Duration::from_secs(2), async {
            loop {
                client.receive(connection).await?;
                if client
                    .latest()
                    .is_some_and(|snapshot| snapshot.actors.iter().all(|v| v.health == 75))
                {
                    return Ok::<_, Box<dyn Error + Send + Sync>>(());
                }
            }
        })
        .await??;
    }
    stop_tx.send(()).map_err(|_| "server stopped early")?;
    let report = server.await??;
    assert_eq!(report.hits, 2);
    assert_eq!(report.health, [75, 75]);
    assert!(report.malformed >= if faults { 2 } else { 3 });
    assert!(report.rejected_moves >= 1);
    println!(
        "Verified encrypted Quinn 0.11.11 {} loopback: two certificate-verified clients, ALPN theia-lab-1, wrong hostname, untrusted certificate and ALPN mismatch rejected; {} fixed ticks, {} admitted movement DATAGRAMs, {} hits, {} malformed messages and {} stale moves rejected; final health {:?}; reliable retry/conflict/ACK-hole/window repair and stalled-stream isolation and monotonic client snapshot publication passed.",
        if ipv6 { "IPv6" } else { "IPv4" },
        report.ticks,
        report.movements,
        report.hits,
        report.malformed,
        report.rejected_moves,
        report.health
    );
    println!(
        "Readonly bounded interpolation: {} / {} explicit screen-time samples; latest logical IDs/health retained, no authority mutation or extrapolation.",
        first_client.samples(),
        second_client.samples()
    );
    first_client.disconnect()?;
    second_client.disconnect()?;
    assert!(first_client.latest().is_none() && second_client.latest().is_none());
    if let Some(relays) = &relays {
        for relay in relays {
            relay.enable(false);
        }
    }
    for connection in [&first, &second] {
        connection.close(0u8.into(), b"qualification complete");
    }
    first_endpoint.close(0u8.into(), b"done");
    second_endpoint.close(0u8.into(), b"done");
    server_endpoint.close(0u8.into(), b"done");
    tokio::join!(
        first_endpoint.wait_idle(),
        second_endpoint.wait_idle(),
        server_endpoint.wait_idle()
    );
    if let Some(relays) = relays {
        let mut dropped = 0;
        let mut reordered = 0;
        let mut duplicated = 0;
        for relay in relays {
            let report = relay.finish().await?;
            dropped += report.dropped;
            reordered += report.reordered;
            duplicated += report.duplicated;
            assert!(report.max_pending <= 2);
            println!("Controlled encrypted packet relay: {report:?}");
        }
        assert!(dropped > 0 && reordered > 0 && duplicated > 0);
    }
    println!(
        "No account authentication, persistence, baseline deltas, renderer integration or scale/performance claim. Keys/certificates exist only in memory; 0-RTT disabled."
    );
    Ok(())
}
#[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut ipv6 = false;
    let mut faults = false;
    for arg in args {
        match arg.as_str() {
            "--ipv6" => ipv6 = true,
            "--faults" => faults = true,
            "--help" => {
                println!(
                    "Theia encrypted two-client loopback lab: [--ipv6] [--faults]; default IPv4; ephemeral localhost ports/certificates only"
                );
                return Ok(());
            }
            _ => return Err("Use --ipv6, --faults or --help".into()),
        }
    }
    timeout(Duration::from_secs(20), qualify(ipv6, faults)).await?
}
