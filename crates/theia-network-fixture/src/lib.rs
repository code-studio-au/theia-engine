//! Shared bounded qualification fixture, not a production networking/server API.
//! Two actors, explicit limits, encrypted Quinn and strict prototype wire messages.
pub mod client;
pub mod proxy;
pub mod replication_wire;
pub mod server;
pub mod tls;
pub mod wire;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
