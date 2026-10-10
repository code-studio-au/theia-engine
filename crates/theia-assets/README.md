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
