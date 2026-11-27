# Theia Engine

Theia is a custom Rust engine experiment for GPU-driven rendering, streamed worlds
and authoritative multiplayer simulation. Its native renderer uses Vulkan directly
through `ash`, with Slang compute, task and mesh shaders. The project develops
small executable slices of an MMO engine and checks them against independent CPU
models, captured images and actual GPU execution.

The repository now contains substantially more than a triangle demo: a bounded
GPU ECS, GPU visibility and material resolve, cooked asset streaming, a terrain
and crowd benchmark with 10,000 visual proxies, server-owned movement and combat,
and encrypted two-client replication with loss and baseline-repair tests.
These are implemented experimental systems with documented qualification limits;
a complete production MMO engine is still a development goal.

This is a self-contained engine development repository. Tests use synthetic scenes
and fixtures; game stories, worldbuilding and conversation exports belong elsewhere.

## Technical achievements

The results below summarize the recorded qualification runs in
[CURRENT_STATUS](docs/CURRENT_STATUS.md) and the linked evidence documents. They
are specific to the tested revisions, fixtures and environments. The primary GPU
reference is an AMD Radeon RX 7900 XTX running Mesa RADV on Linux Mint.

### GPU-generated rendering and work submission

The renderer can generate triangles directly in mesh shaders without conventional
vertex/index buffers or a vertex shader. Task shaders amplify work and pass shared
payloads to mesh groups. In the indirect path, a compute shader writes task-group
dimensions, draw count, transforms and downstream mesh counts into device-local
storage; `vkCmdDrawMeshTasksIndirectCountEXT` consumes those results without CPU
count readback. Synchronization2 barriers connect compute writes to indirect and
shader reads.

This establishes a working route from GPU decisions to GPU rendering. The initial
qualification exercised zero, one and two generated triangles, preserved direct
mesh/task paths, and passed 120-frame resize tests with zero validation diagnostics
including teardown. The original headless compute regression independently checks
all 256 expected output values.

Evidence: [indirect work](docs/validation/M0D_INDIRECT_WORK.md),
[task amplification](docs/validation/M0D_TASK_AMPLIFICATION.md),
[Render Lab closeout](docs/validation/M0D_CLOSEOUT.md).

### GPU ECS with checked identity and resource lifetime

The ECS slice implements dense position/velocity storage, two archetypes,
generational handles, 64-bit world identities, spawn/despawn, migration and row
compaction. Publication epochs and retained readers keep older data valid while
current state changes. Bulk component integration runs in parallel on the GPU;
the bounded structural command queue is deliberately ordered on one lane.

The recorded GPU run matched a CPU oracle across **526 batches and 8,717 commands**.
Checks cover stale/conflicting handles, exhausted chunks and slots, failed migration,
compaction repair, retained GPU readers and permanent generation exhaustion.
CPU tests also compare **10,000 randomized operations** against an independent map
model. These checks establish identity and lifetime correctness before attempting
larger-scale mutation or overlapping rendering workloads.

Evidence: [GPU ECS qualification](docs/validation/M1_GPU_ECS.md).

### GPU visibility and deferred material reconstruction

A direct-addressed XY grid builds contiguous cell membership ranges. GPU work
gathers candidates, performs height-aware bounds culling, compacts visible IDs and
feeds indirect task/mesh rendering. Empty, filtered and full test views produce
exactly **0, 8 and 32 triangles**; CPU tests include **1,000 randomized view
comparisons** against brute force.

The visibility path writes integer instance/primitive identity and D32 depth,
then reconstructs surface attributes for material resolve. Subsequent slices add
perspective-correct authored UVs, opaque material factors, nonuniform texture
selection and analytic UV footprints for mip selection. This separates geometry
visibility from surface shading while keeping results comparable to independent
pixel, depth and attribute references.

Evidence: [spatial visibility](docs/validation/M2_SPATIAL_VISIBILITY.md),
[visibility and materials](docs/validation/M3_VISIBILITY_MATERIALS.md),
[automatic mip selection](docs/validation/M4_GPU_AUTOMATIC_MIPS.md).

### Cooked assets, bounded streaming and protected coarse coverage

The content pipeline supports bounded static glTF import, offline meshlet cooking,
authored material/UV/image data, immutable bundles and explicit residency budgets.
The camera-driven world keeps four coarse regions available while loading and
retiring finer representations. Pending loads become selectable only after upload
readiness and publication; cancellation or insufficient capacity preserves coarse
coverage. CPU encoded bytes, GPU allocations and image allocations are accounted
for separately, including resources that coexist during replacement.

A repeated **1,320-frame route** recorded **88 fine publications and 86 retirements**
with bounded history. Coarse/fine quadrant assets preserve their shared perimeter:
all **16 combinations** pass coverage/edge checks, with continuous vertical error
below **0.12 m** for that fixture. Independent 16-image replacement tests show that
exact budgets admit replacement while limits **one byte smaller** defer it and
retain the current complete set. This exercises the difficult transition and
failure paths as well as successful loading.

Evidence: [camera-driven residency](docs/validation/M4_CAMERA_WORLD_DEMAND.md),
[fallback geometry](docs/validation/M4_CHUNK_FALLBACK_GEOMETRY.md),
[texture replacement capacity](docs/validation/M4_TEXTURE_STREAMING_CAPACITY.md).

### Texture quality and GPU LOD decisions

The renderer has qualified explicit mip selection from analytic perspective UV
gradients, bilinear sampling, coarse mip tails and asynchronous mip-tail refinement.
Offline filtered mip cooking checks linear-light sRGB averages against independent
references. GPU-owned LOD history adds hysteresis to reduce coarse/fine switching
while preserving the refinement limit.

For one measured horizontal texture fixture, physical image allocations fell from
**9,472 bytes** for the full chain to **1,792 / 1,536 / 256 bytes** for progressively
smaller tails. Five filtered sources across **31 levels** matched the independent
per-step RGB reference exactly. These are concrete quality and allocation results
for the qualified profiles; general virtual texturing and arbitrary asset coverage
remain future work.

Evidence: [GPU mip tails](docs/validation/M4_GPU_MIP_TAIL.md),
[filtered mip cooking](docs/validation/M4_FILTERED_MIP_COOKING.md),
[LOD hysteresis](docs/validation/M4_GPU_LOD_HYSTERESIS.md),
[asynchronous refinement](docs/validation/M4_ASYNC_MIP_REFINEMENT.md).

### A streamed world with 10,000 GPU-grounded visual proxies

The integrated benchmark renders camera-streamed world chunks and up to **10,000
moving visual proxies** through shared visibility/depth. Ground queries use the
currently committed fine/coarse terrain, so proxy placement follows the selected
surface. These are visual workload instances, not 10,000 networked players or
fully simulated characters.

The terrain compute qualification checked **569,680 heights** with sampled error
below **3.65e-7 m** against its source reference. World/crowd integration passed
23 native cases in each of debug and release, plus eight replays with **1,581 image
probes**, proxy pose/count checks and world/crowd occlusion in both directions.
A later live HUD displays GPU work counts, completed timings and residency state;
its replay checks include **45,360 glyph pixels** and **1,084 scene probes**.
Separate release runs retain **3,600 measured frames** for timing analysis.

Evidence: [ground queries](docs/validation/M4_GPU_SELECTED_TERRAIN_HEIGHTS.md),
[world and crowd integration](docs/validation/M4_STREAMED_WORLD_CROWD.md),
[live HUD](docs/validation/M4_STREAMED_WORLD_HUD.md),
[timing scopes](docs/validation/M4_STREAMED_WORLD_TIMINGS.md).

### Server-owned movement and combat

A separate Rust simulation owns movement rules, swept collision against the
analytic static-world fixture, health, damage, range, line of sight, cooldowns and
death. Clients submit sequenced intents; server context determines actor identity.
Dense storage keeps logical identity independent of row compaction, and presentation
receives readonly state.

The movement lab executed **100,000 ticks and 300,000 admitted inputs**, checking
speed limits, collision and invalid/replayed input. The combat oracle checked
**5,000 ticks and 9,921 requests**, including **948 hits and 8,973 cooldown
rejections**, against an independent model. Same-tick ordering, death, despawn and
rejected targets are tested explicitly.

Evidence: [authoritative movement](docs/validation/M7_AUTHORITATIVE_MOVEMENT.md),
[authoritative combat](docs/validation/M7_AUTHORITATIVE_COMBAT.md).

### Encrypted replication with deliberate packet faults and repair

Separate server and viewer processes communicate through certificate-verified
Quinn/QUIC. Movement and reliable combat use independent sequence domains.
Replication selects application-acknowledged baselines; clients reconstruct full
or delta snapshots before publishing readonly state, and request full repair when
a baseline is missing.

The fault harness drops, delays, reorders and duplicates opaque encrypted packets.
Recorded two-viewer GPU runs and a **600-frame native input/combat run** verify
acknowledged hits, death and presentation through those faults with zero Vulkan
diagnostics. Explicit ACK withholding forces actual missing-baseline repair during
resize/recreation runs; a repair replay matches **32,387 sampled pixels**. This
connects authority, transport, recovery and visible presentation in one bounded
end-to-end slice.

Evidence: [live packet faults](docs/validation/M8_LIVE_PACKET_FAULTS.md),
[live delta replication and repair](docs/validation/M8_LIVE_DELTA_REPLICATION.md).
Accounts, durable persistence, WAN qualification, generalized reconnect and network
scale are still open.

### Independent benchmark tooling that exposes tradeoffs

An isolated Bevy comparison executable runs the same cooked source scenes and
logical camera/proxy routes. It supports replay comparisons and independent
streaming with shared frame deadlines, bounded fine slots, encoded-memory caps,
optional GPU timings and HUD on/off controls. Bevy is a comparison dependency only;
Theia's custom runtime continues to use Rust/ash/Slang.

Five independent-streaming pairs passed coverage, ownership and clock checks;
diagnostic/replay captures passed **664 scene probes** and **380,000 proxy pose
checks**. Separate 60 Hz release controls retained **600 measured frames per run**
with zero full-period deadline misses. The reports expose mismatched detail and
unresolved demand instead of hiding them: one recorded pair had **140 missing-fine
region-frames for Theia versus 275 for Bevy**, out of 1,125 common near-demand
region-frames. This is a scoped readiness observation, not a whole-engine ranking.

The reports retain raw GPU timestamps and distinguish CPU scopes, selected detail,
logical memory and physical allocation. Captures are separated from timed runs.
That provides a repeatable basis for identifying regressions and evaluating changes
without treating unlike measurements as equivalent.

Evidence: [independent streaming comparison](docs/validation/M4_INDEPENDENT_STREAMING_COMPARISON.md),
[comparison tool](tools/theia-bevy-benchmark/README.md).

### Explicit Vulkan ownership and platform qualification

Vulkan resources use explicit ownership, staged publication and retirement, with
RAII cleanup on construction failures. Qualification covers real validation-layer
activation, debug callback delivery, synchronization validation, swapchain
recreation, zero extents, missing shaders and capacity failures. Recovery injection
is labelled separately from actual driver-return evidence.

Beyond X11, the repository records native Wayland surfaces in owned nested
compositors, composited pixel checks and texture-capacity runs. Windows work includes
GNU cross-build/Clippy qualification for 11 Rust packages and CPU tests under Wine.
The tests caught a real Winsock oversized-UDP receive difference and qualified its
recovery. Native Windows GPU execution and broad cross-vendor behavior remain open.

Evidence: [Wayland pixels](docs/validation/WAYLAND_PIXELS.md),
[Wayland texture capacity](docs/validation/M4_WAYLAND_TEXTURE_CAPACITY.md),
[Windows build and compatibility](docs/validation/WINDOWS_BUILD.md).

## Architecture and direction

The central design choice is to put coherent, resident presentation work on the
GPU while retaining CPU ownership of I/O, resource allocation, synchronization and
server authority. Stable identities are separate from storage locations; rendering,
collision, replication and persistence use representations suited to their jobs.

| Area | Implemented experimental foundation | Further development |
|---|---|---|
| Rendering | Compute/task/mesh work, visibility/depth, material and texture resolve | Broader lighting, shadows, hierarchical visibility and performance tuning |
| ECS | Bounded dense GPU reference, handles, migration, retained epochs | General schemas, scalable structural mutation and overlapping readers |
| World content | Cooked meshlets, textured bundles, coarse/fine streaming and budget checks | General meshlet hierarchies, wider asset support and virtual textures |
| Simulation | Fixed-tick authoritative movement/combat on analytic fixtures | Mesh navigation/collision, richer interactions and persistence |
| Networking | Encrypted two-client transport, sequencing, snapshots, deltas and repair | WAN/load qualification, reconnect, accounts and production operations |
| UI and animation | Fixed GPU debug HUD and moving visual proxies | General Bézier UI, skeletal animation, pose sharing and accessibility integration |
| Lighting | Material/texture reconstruction foundation | Sparse-SDF software dynamic GI, radiance caching and temporal reconstruction |

Rust is the runtime and simulation language; Slang is the primary GPU language.
Go services remain an architectural target. Python orchestrates offline cooking,
qualification and reports. Mature third-party C/C++ tools are permitted offline;
there is no custom C++ runtime. Hardware ray tracing is not required. The optional
shared primitive interface uses a documented, qualified GLSL path where needed;
shader frontend changes follow explicit compatibility gates.

See [architecture](docs/ARCHITECTURE.md), [decisions](docs/DECISIONS.md),
[subsystems](docs/SUBSYSTEMS.md), [roadmap](docs/ROADMAP.md) and
[risks and validation](docs/RISKS_AND_VALIDATION.md) for the detailed contracts.

## Repository layout

| Path | Purpose |
|---|---|
| `apps/theia-gpu-test` | Preserved headless compute regression |
| `apps/theia-render-lab` | Native renderer, visual qualification and benchmarks |
| `apps/theia-ecs-lab` | GPU ECS versus CPU oracle |
| `apps/theia-server-lab` | Authoritative movement, combat and request qualification |
| `apps/theia-network-lab` | Encrypted transport and replication qualification |
| `crates/theia-ecs` | Dense ECS reference and identity/lifetime contracts |
| `crates/theia-assets` | Asset validation, bundles, residency and selection contracts |
| `crates/theia-sim` | Server-owned simulation |
| `crates/theia-protocol` | Sequencing, snapshots, deltas and baseline handling |
| `crates/theia-network-fixture` | Shared bounded encrypted transport/fault fixture |
| `shaders` | Compute, task, mesh and resolve programs |
| `fixtures` | Synthetic source assets and reference inputs |
| `tools` | Importers, cooking, qualification, reports and optional Bevy comparison |
| `docs/validation` | Recorded execution evidence and limitations |

## Build and run

Use Rust with edition 2024 support, Cargo, Python 3, the qualified Slang compiler
and SPIRV-Tools. The recorded shader toolchain uses Slang 2026.19. Desktop execution
requires a Vulkan loader/driver, an active display and the Khronos validation layer
unless validation is explicitly disabled. The intended GPU profile requires Vulkan
1.3 and both mesh/task capabilities; individual pipelines may omit the task stage.
Render Lab checks additional features and maintenance extensions at startup. See
[platform requirements](docs/PLATFORMS_AND_BUILD.md) and
[Render Lab usage](apps/theia-render-lab/README.md) for the exact requirements.

Run commands from the repository root. Packages have separate manifests and lockfiles.

```sh
# Compile and validate renderer shaders, then run the mesh triangle smoke test.
python3 tools/compile_render_shaders.py
cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- --mesh-only --smoke-test

# Compile and run the GPU ECS qualification.
python3 tools/compile_ecs_shaders.py
cargo run --locked --manifest-path apps/theia-ecs-lab/Cargo.toml

# Run CPU authority and encrypted loopback qualification.
cargo run --locked --manifest-path apps/theia-server-lab/Cargo.toml -- --combat-test
cargo run --locked --manifest-path apps/theia-network-lab/Cargo.toml -- --replication-test --faults
```

For the integrated streamed world and crowd benchmark:

```sh
python3 tools/run_scene_benchmark.py --streamed-world --slangc /path/to/slangc
```

This launcher builds shaders/the renderer, prepares eight fine/coarse graphs and
runs 10,000 proxies with live stats. Asset preparation requires the documented
meshoptimizer/KTX tool cache or a pre-cooked manifest; it does not install those
tools automatically. Follow [scene benchmark setup](docs/SCENE_BENCHMARK.md) for
cooking, headless execution, timing controls and report interpretation. The
[Bevy tool README](tools/theia-bevy-benchmark/README.md) documents optional comparisons.

## Validation and contributions

For each changed Rust package, run formatting, denied-warning Clippy and tests:

```sh
cargo fmt --manifest-path apps/theia-render-lab/Cargo.toml -- --check
cargo clippy --locked --manifest-path apps/theia-render-lab/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path apps/theia-render-lab/Cargo.toml
python3 -B -m unittest discover -s tools -p 'test_*.py'
```

[CI](.github/workflows/rust.yml) defines Linux/Windows package checks, CPU authority
and networking runs, and shader/asset qualification. GPU execution and pixel evidence
require a suitable local environment. Keep CPU tests, headless GPU execution,
native input, captured replay and performance measurements labelled separately.

The [2026-11-27 integration checks](docs/validation/BRANCH_INTEGRATION.md) record
fresh package tests, shader validation and CPU/network runs after merging the
development branches, alongside the GPU checks that require local hardware.

Preserve the existing regression paths, add one verifiable slice at a time, and
record both the result and its limits. No production player-count, whole-engine
performance superiority or universal platform-support claim follows from these
bounded experiments. Keep credentials and private exports outside the repository.
