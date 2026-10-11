# Shared Theia network qualification fixture

Existing bounded two-actor Quinn/rustls/ring transport, strict prototype wire and
readonly client receiver, extracted from `theia-network-lab` for later reuse.
The executable retains the opaque packet fault relay. No production server API,
accounts, persistence or general replication is implied.

```sh
cargo test --locked --manifest-path crates/theia-network-fixture/Cargo.toml
cargo run --locked --manifest-path apps/theia-network-lab/Cargo.toml -- --faults
```

Extraction qualification logs: `build/qualification/m8-shared-network-*` (ignored).
Three shared codec tests/one app relay test, formatting/Clippy, direct/faulted
runs pass; both clients retain exactly two authoritative hits and health [75,75].
Wire/security/gameplay behavior is unchanged. Windows/hosted CI unexecuted.
