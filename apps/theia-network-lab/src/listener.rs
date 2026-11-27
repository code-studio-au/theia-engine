//! Localhost-only two-client listener fixture. Public certificate export only;
//! signing keys remain in memory. No accounts/persistence/reconnect service.
use crate::Result;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};
use theia_network_fixture::{server, tls, wire::Profile};
pub async fn serve(arguments: &[String]) -> Result<()> {
    let mut address: SocketAddr = "127.0.0.1:0".parse()?;
    let mut certificate = PathBuf::from("build/qualification/m8-listener/localhost.der");
    let mut profile = Profile::LocalDemo;
    let mut args = arguments.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--listen" => address = args.next().ok_or("--listen needs address")?.parse()?,
            "--certificate" => {
                certificate = args
                    .next()
                    .ok_or("--certificate needs public DER path")?
                    .into()
            }
            "--laboratory-profile" => profile = Profile::Laboratory,
            _ => return Err(format!("Unknown listener argument {arg}").into()),
        }
    }
    if !address.ip().is_loopback() {
        return Err("Qualification listener must bind an explicit loopback address".into());
    }
    let (endpoint, public_certificate) = tls::server_endpoint(address)?;
    if let Some(parent) = certificate.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&certificate, public_certificate.as_ref())?;
    let absolute = if certificate.is_absolute() {
        certificate.clone()
    } else {
        std::env::current_dir()?.join(&certificate)
    };
    println!(
        "{}",
        serde_json::json!({"listener":true,"address":endpoint.local_addr()?.to_string(),"certificate":absolute,"profile":profile as u8,"alpn":"theia-lab-3","clients":2})
    );
    let result = async {
        let mut connections = Vec::with_capacity(2);
        while connections.len() < 2 {
            let incoming = if let Some(first) = connections.first() {
                let first: &quinn::Connection = first;
                tokio::select! {
                    incoming = endpoint.accept() => incoming.ok_or("Listener closed")?,
                    _ = first.closed() => return Err("First peer disconnected before the pair was ready".into()),
                }
            } else { endpoint.accept().await.ok_or("Listener closed")? };
            match tokio::time::timeout(std::time::Duration::from_secs(5), incoming).await {
                Ok(Ok(connection)) => connections.push(connection),
                Ok(Err(error)) => eprintln!("Rejected handshake: {error}"),
                Err(_) => eprintln!("Rejected handshake: deadline expired"),
            }
        }
        let [first, second]: [quinn::Connection;2] = connections.try_into().map_err(|_| "Expected exactly two connections")?;
        let (_stop_owner, stop) = tokio::sync::oneshot::channel();
        let report = server::run_profile([first,second], stop, profile).await?;
        println!("Listener pair completed: {report:?}");
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }.await;
    endpoint.close(0u8.into(), b"fixture pair ended");
    endpoint.wait_idle().await;
    result
}
/// Bound certificate reads before constructing any trust config or socket.
pub fn public_der(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?.take(65537);
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > 65536 {
        return Err("Public certificate must be 1..65536 bytes".into());
    }
    Ok(bytes)
}

/// Independent executable client bootstrap/input/snapshot qualification.
pub async fn probe(arguments: &[String]) -> Result<()> {
    use std::time::Duration;
    use theia_network_fixture::{
        client,
        wire::{self, Message},
    };
    use theia_sim::MoveInput;
    if arguments.len() != 2 {
        return Err("--client-probe needs loopback address and public certificate path".into());
    }
    let address: SocketAddr = arguments[0].parse()?;
    if !address.ip().is_loopback() {
        return Err("Client probe must target loopback".into());
    }
    let der = public_der(Path::new(&arguments[1]))?;
    let mut endpoint = quinn::Endpoint::client(SocketAddr::new(address.ip(), 0))?;
    endpoint.set_default_client_config(tls::client_from_der(&der)?);
    let connection = tokio::time::timeout(
        Duration::from_secs(5),
        endpoint.connect(address, "localhost")?,
    )
    .await??;
    let result = async {
        let welcome = client::receive_welcome(&connection).await?;
        let mut replica = client::Client::new()?; replica.bind(&connection, welcome)?;
        let axes = if welcome.actor.value()==10 { [1000,0] } else { [-1000,0] };
        for (sequence, axes) in [(0,axes),(1,[0,0])] {
            tokio::time::timeout(Duration::from_secs(2), async {
                let mut retry = tokio::time::interval(Duration::from_millis(50));
                loop {
                    tokio::select! {
                        _ = retry.tick() => connection.send_datagram(wire::encode(&Message::Move(MoveInput { sequence, axes })).into())?,
                        received = replica.receive(&connection) => {
                            received?;
                            if replica.latest().is_some_and(|v| v.applied_move.is_some_and(|ack| ack>=sequence)) { break; }
                        }
                    }
                }
                Ok::<(), Box<dyn std::error::Error+Send+Sync>>(())
            }).await??;
        }
        let snapshot = replica.latest().ok_or("No probe snapshot")?;
        let actor = snapshot.actors.iter().find(|v| v.id==welcome.actor).ok_or("Assigned actor absent")?;
        if actor.position_mm[0] == if welcome.actor.value()==10 { -1000 } else { 1000 } { return Err("Probe directional input did not move its assigned actor".into()); }
        if (welcome.actor.value()==10 && actor.position_mm[0]<=-1000) || (welcome.actor.value()==20 && actor.position_mm[0]>=1000) { return Err("Probe direction was not bound to its assigned actor".into()); }
        let y = if welcome.profile == Profile::LocalDemo { -1000 } else { 0 };
        if actor.position_mm[1] != y || actor.health != 100 { return Err("Probe changed unrequested authority state".into()); }
        println!("{}",serde_json::json!({"probe":true,"actor":welcome.actor.value(),"target":welcome.target.value(),"profile":welcome.profile as u8,"tick":snapshot.tick,"applied_move":snapshot.applied_move,"position_mm":actor.position_mm,"health":actor.health}));
        // Leave a bounded grace period for the other independent probe to consume its full state.
        tokio::time::sleep(Duration::from_millis(500)).await;
        replica.disconnect()?; Ok::<(),Box<dyn std::error::Error+Send+Sync>>(())
    }.await;
    connection.close(0u8.into(), b"probe complete");
    endpoint.close(0u8.into(), b"probe ended");
    endpoint.wait_idle().await;
    result
}
