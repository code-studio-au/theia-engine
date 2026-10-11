# Theia encrypted network lab

Bounded two-client Quinn qualification over ephemeral localhost UDP sockets.
Rustls/ring verifies an in-memory generated certificate; keys are never written.
DATAGRAM carries movement/full snapshots; independent reliable streams carry
combat requests and application outcome ACKs. Server connection contexts bind
actors and all gameplay effects remain in the authoritative simulation.

```sh
cargo run --locked --manifest-path apps/theia-network-lab/Cargo.toml
cargo test --locked --manifest-path apps/theia-network-lab/Cargo.toml
```

`--ipv6` selects IPv6 explicitly and fails when unavailable. IPv4 is qualified
locally; this host disables IPv6. The whole run has a 20-second deadline.
No account authentication, persistence, general replication/deltas, renderer
integration or performance claim. Prototype wire/rules are not engine constants.
See [qualification evidence](../../docs/validation/M8_QUINN_LOOPBACK.md).
