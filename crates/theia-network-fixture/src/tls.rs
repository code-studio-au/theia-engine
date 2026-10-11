use crate::Result;
use quinn::{
    ClientConfig, Endpoint, ServerConfig, TransportConfig,
    crypto::rustls::{QuicClientConfig, QuicServerConfig},
    rustls,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
const ALPN: &[u8] = b"theia-lab-2";
fn transport() -> Arc<TransportConfig> {
    let mut t = TransportConfig::default();
    t.max_concurrent_bidi_streams(4u8.into())
        .max_concurrent_uni_streams(0u8.into())
        .stream_receive_window(256u32.into())
        .receive_window(4096u32.into())
        .send_window(4096)
        .datagram_receive_buffer_size(Some(1024))
        .datagram_send_buffer_size(128)
        .max_idle_timeout(Some(Duration::from_secs(5).try_into().unwrap()))
        .keep_alive_interval(Some(Duration::from_secs(1)));
    Arc::new(t)
}
pub fn server_endpoint(
    bind: SocketAddr,
) -> Result<(Endpoint, rustls::pki_types::CertificateDer<'static>)> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let der = rustls::pki_types::CertificateDer::from(cert.cert);
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
    let mut crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![der.clone()], key.into())?;
    crypto.alpn_protocols = vec![ALPN.to_vec()];
    crypto.max_early_data_size = 0; // Never accept state-changing 0-RTT requests.
    let mut server = ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(crypto)?));
    server
        .transport_config(transport())
        .max_incoming(2)
        .incoming_buffer_size(4096)
        .incoming_buffer_size_total(8192);
    let endpoint = Endpoint::server(server, bind)?;
    Ok((endpoint, der))
}
/// Trust exactly the caller-provided public localhost certificate, never a bypass.
pub fn client_from_der(bytes: &[u8]) -> Result<ClientConfig> {
    if bytes.is_empty() || bytes.len() > 65536 {
        return Err("Certificate must be 1..65536 public DER bytes".into());
    }
    let mut roots = rustls::RootCertStore::empty();
    roots.add(rustls::pki_types::CertificateDer::from(bytes.to_vec()))?;
    client_config(roots, ALPN)
}
pub fn endpoints(bind: SocketAddr) -> Result<(Endpoint, ClientConfig, [ClientConfig; 2])> {
    let (endpoint, der) = server_endpoint(bind)?;
    let mut roots = rustls::RootCertStore::empty();
    roots.add(der)?;
    let good = client_config(roots.clone(), ALPN)?;
    let untrusted = client_config(rustls::RootCertStore::empty(), ALPN)?;
    let wrong_protocol = client_config(roots, b"wrong-lab")?;
    Ok((endpoint, good, [untrusted, wrong_protocol]))
}
fn client_config(roots: rustls::RootCertStore, alpn: &[u8]) -> Result<ClientConfig> {
    let mut crypto = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    crypto.alpn_protocols = vec![alpn.to_vec()];
    crypto.enable_early_data = false;
    let mut client = ClientConfig::new(Arc::new(QuicClientConfig::try_from(crypto)?));
    client.transport_config(transport());
    Ok(client)
}
