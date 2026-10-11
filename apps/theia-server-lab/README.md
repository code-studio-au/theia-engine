# Theia server authority lab

Headless local qualification for `theia-sim`; it opens no sockets and performs
no authentication, replication, persistence or rendering. Run from any working
directory with the Cargo manifest path; no Vulkan/GPU is required.

```bash
cargo run --locked --manifest-path apps/theia-server-lab/Cargo.toml -- --ticks 10000
```

Tick counts 1..100000 are supported. Three seeded actors exercise bounded
server-owned movement and static collision; replay and invalid direction inputs
are deliberately rejected. The test loop advances fixed simulation steps as
fast as possible. Its execution log is correctness evidence, not a server tick
latency/throughput benchmark or an end-to-end playable client.

Use `--combat-test` for the separate 5000-tick health/cooldown oracle and basic
LOS/range/death/order cases. It cannot combine with `--ticks`. Sixteen library
tests pass; networking and rendered client integration remain unimplemented.
