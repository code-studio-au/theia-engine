# Theia immutable asset reader — M4 prototype

Pure Rust reader for the bounded `THEIAMES` version-1 meshlet container. Validates
content/payload digests, lengths, byte budget, finite positions, padding and
meshlet/local/global reference ranges before exposing decoded data. No native
meshoptimizer dependency. Format/limits are qualification choices, not a locked
production schema. See [the contract](../../docs/M4_ASSET_CONTRACT.md).

```bash
cargo fmt --manifest-path crates/theia-assets/Cargo.toml -- --check
cargo clippy --locked --manifest-path crates/theia-assets/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path crates/theia-assets/Cargo.toml
cargo run --locked --manifest-path crates/theia-assets/Cargo.toml --example inspect -- <cooked-file>
```

The lab additionally anchors the full-file digest to the filename. The inspect
example accepts any filename and reports successful container validation. Tests
cover valid data, digest/version/length/budget corruption, every truncated prefix,
and correctly rehashed but invalid geometry/padding.

`residency` adds a bounded immutable cell-manifest reader and CPU page lifetime
oracle. Snapshots and staged tokens retain encoded allocations across eviction;
the byte budget counts all unique live pages. This does not model Vulkan upload
completion or physical GPU memory. See the contract for schema and exclusions.
The `residency` example runs the retained-reader sequence on two real cooked files.

`arena` provides aligned host-backed range leases with last-owner release and
free-range coalescing. Retained clones prevent reuse. Tests cover fragmentation,
overflow/ticket exhaustion and 5,000 randomized operations against an independent
occupancy bitmap. Renderer use still needs explicit GPU upload/frame completion;
the CPU allocator does not infer GPU completion from a dropped lease.

`bundle_residency::WorldResidency` admits several independently named complete
bundles against one unique encoded-byte pool and a separate chunk-slot capacity.
Atomic add/replace/remove transactions preserve retained world/chunk snapshots.
No camera scheduling, I/O, Vulkan upload or GPU publication is implied. Run
`--example bundle_residency -- --world <bundle0> <bundle1> ...` for real-file
exact/short-budget, shared ownership and retained-reader qualification.

`world_demand` provides a stateless, bounded outdoor XY-distance demand oracle with
load/retention bands and deterministic slot pressure. It returns planned IDs only;
byte admission, fallback coverage, pending work and GPU retirement stay separate.
The bundle example's `--world-route <load-radius> <retain-radius> <slots>
<byte-budget> <xy-route.txt> <bundle> ...` exercises CPU publication on a bounded
route. `tools/qualify_world_demand.py` compares it with an independent model using
the benchmark camera and checked quadrant fixtures. No GPU renderer is changed.
