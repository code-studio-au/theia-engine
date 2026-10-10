# M1 bounded CPU oracle

A correctness reference for GPU presentation data. Two fixed archetypes contain
chunked SoA position and optional velocity columns. World IDs, generational handles
and physical locations are distinct. Swap-removal repairs metadata; migration
reserves destination capacity before changing the source.

`publish` processes commands in queue order at an explicit epoch. A failed command
leaves its entity unchanged; prior successes remain committed. Despawned slots are
reclaimed at batch boundaries, after all retained older readers release. Generation
wrap permanently exhausts the slot. Reader data copies are CPU reference artifacts,
not a production full-ECS-copy policy. Call `release` on the originating oracle;
unreleased leases conservatively prevent reuse.

This is not an authoritative/server ECS, general query system, GPU allocator or
performance result. Chunk/slot sizes are qualification parameters, not locked
engine constants. Bulk visual integration is deterministic reference arithmetic;
production time/overflow policies remain to be defined.

```bash
cargo fmt --manifest-path crates/theia-ecs/Cargo.toml -- --check
cargo clippy --locked --manifest-path crates/theia-ecs/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path crates/theia-ecs/Cargo.toml
```

Tests cover relocation, overflow, conflicts, generation wrap, retained epochs,
invalid components and 10,000 deterministic randomized operations compared with
an independent map model. GPU comparison is a separate qualification step.
