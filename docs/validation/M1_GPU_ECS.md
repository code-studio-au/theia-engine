# M1 bounded GPU ECS — 2026-10-10

Reference environment unchanged: RX 7900 XTX, Linux Mint 22.3, RADV Mesa 26.2.4,
Vulkan loader/validation 1.3.275, Rust 1.99.0, ash 0.38, Slang 2026.19.

## Executed commands

```bash
python3 tools/compile_ecs_shaders.py
cargo fmt --manifest-path crates/theia-ecs/Cargo.toml -- --check
cargo clippy --locked --manifest-path crates/theia-ecs/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path crates/theia-ecs/Cargo.toml
cargo fmt --manifest-path apps/theia-ecs-lab/Cargo.toml -- --check
cargo clippy --locked --manifest-path apps/theia-ecs-lab/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path apps/theia-ecs-lab/Cargo.toml
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
  cargo run --locked --manifest-path apps/theia-ecs-lab/Cargo.toml
```

Observed GPU log:

```text
ECS debug callback delivery confirmed (intentional diagnostic excluded from totals)
Khronos layer and synchronization validation enabled for ECS qualification
ECS GPU: AMD Radeon RX 7900 XTX (RADV NAVI31); Vulkan 1.3, Sync2, mesh/task capabilities checked; compute family 0
GPU deterministic qualification: chunk/slot overflow, conflicts, nonfinite input, retained epochs and stale generation passed
GPU generation exhaustion: all 128 slots permanently retired after reader release
GPU ECS verified: 526 batches, 8717 commands, dense state/results vs CPU oracle; parallel integration and retained GPU reader verified
ECS validation totals including teardown: 0 warnings, 0 errors
```

Four entry points independently compile and pass `spirv-val --target-env vulkan1.3`.
CPU tests compare 10,000 randomized operations with an independent map model;
GPU tests compare 512 randomized batches plus deterministic boundary cases with
the dense CPU oracle. Runtime checks cover live identity/location bijection,
world-ID high bits, command results, dense positions/velocities, failed-migration
preservation, compaction repair, stale generations, overflow and conflicts.
Retained device-local storage survives later current-arena mutations and is read
by a GPU kernel. Delayed handle reuse agrees with retained CPU leases. Initializing
generations at `u32::MAX` demonstrates permanent exhaustion of all 128 slots.

Missing `ecs.integrate.spv` after buffers/descriptors/two pipelines exist exits with
an explicit file error and zero teardown diagnostics. A GPU filter with no match
exits with an explicit selection error and zero diagnostics. The original headless
compute and Render Lab remain separate and unchanged by this M1 implementation.
Logs are local ignored artifacts under `build/qualification/`.

## Scope and limits

This qualifies a **bounded M1 reference slice**, not a complete ECS, production
allocator, GPU workload DAG, authoritative server, rendering integration or scale
benchmark. Small structural queues run on one GPU lane; bulk integration is
parallel. Host comparisons/readbacks are diagnostic plumbing. Only selected
position/velocity/metadata streams are versioned in the test; no production
full-ECS-copy policy is established.

One retained version is tested across multiple publication epochs. Actual
concurrent multi-queue/in-flight rendering readers, variable archetype schemas,
large-world mutation pressure and general floating-point overflow remain future
qualification. Publication epochs are explicitly 32-bit and bounded; production
bit budgets remain undecided. Windows/other vendors have not executed this slice.
The local CI configuration includes both new Rust packages and shaders, but no
GitHub push or hosted CI result is claimed. All work remains local as requested.
No locked architectural decision changed.
