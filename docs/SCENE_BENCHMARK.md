# Theia MMO scene benchmarks

The launcher supports procedural, resident cooked and camera-streamed profiles.
For the latest integrated sequence use `--streamed-world`: four protected coarse
regions, camera-driven fine publication, GPU-grounded visual proxies and live
stats. The original procedural profile remains the default for compatibility.

## Streamed world quick start

```sh
python3 tools/run_scene_benchmark.py --streamed-world \
  --slangc /path/to/slangc
```

This builds the renderer and validates 52 shader modules, cooks/checks eight
fine/coarse graphs, then runs **10000 proxies** at a requested 1280x720 with the
HUD enabled. It uses the already qualified offline meshoptimizer 1.3/KTX 4.4.2
cache at the paths shown in the cooking section below; it does not download or
install native tools. Missing prerequisites fail with their paths. This cached
preparation layout is Linux-qualified. On other setups prepare the eight-graph
manifest with `cook_benchmark_scene.py --chunk-fallbacks` and supply its path:

```sh
python3 tools/run_scene_benchmark.py --streamed-world \
  --cooked-scene build/qualification/chunk-fallback-cook/scene.json \
  --skip-build --players 10000 --frames 3660 --warmup 60
```

Without `--cooked-scene`, each invocation freshly prepares assets under
`<output>/assets/`, including source-to-cook and Rust CPU admission proof. Reuse
that `assets/scene.json` to avoid cooking again. `--skip-build` reuses compiled
renderer/shaders/importer; the source/admission checker still invokes Cargo.
Automatic preparation does not produce a whole-world bundle for resident mode.

Native desktop execution opens the sequence; add `--headless` for an owned
private compositor, with the dependency/device setup described below. For example:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
python3 tools/run_scene_benchmark.py --streamed-world --skip-build \
  --cooked-scene build/qualification/chunk-fallback-cook/scene.json \
  --headless --headless-dri-prime pci-0000_03_00_0 --gpu 7900 \
  --validation --frames 240 --warmup 30
```

The streamed camera starts moving at frame zero. Warmup excludes samples from
summaries; it does not delay the route. `--route-frames` sets a repeating period
(default `--frames`); every period spans 60 logical seconds. `--seconds` currently
accepts only 60 for this profile. `--fine-slots 1..4` defaults to two. Aggregate
`--bundle-byte-budget`, `--bundle-gpu-budget` and `--texture-budget` cover encoded
CPU graphs, current/pending GPU graphs plus the fixed ground table, and physical
image bytes respectively. Work arenas, staging, offscreen targets and driver
allocations have separate scopes. `--no-overlay` selects the 16-storage-buffer
fragment profile; the live HUD needs 21 and checks the device limit.

The HUD shows recent completed CPU/GPU time, presentation-call FPS, GPU
meshlet/proxy/detail counts, selected texture bindings, asset/draw MiB and fine
publication/retirement/deferral counters. [Full legend](validation/M4_STREAMED_WORLD_HUD.md).
Each `NN-streamed-N-proxies/` directory contains `report.json`, `report.csv`,
interactive `report.html`, `camera.json`, a manifest snapshot and `run.log`.
HTML can plot GPU/CPU time, asset/encoded CPU MiB and residency counters, with
frame selection/epoch in the tooltip. CSV includes all per-frame timing samples
and explicit current/pending/fixed memory columns. `suite.json` publishes only
when all requested populations complete.

These are opaque unlit fixture graphs and visual mock proxies. GPU frame queries
exclude earlier uploads/graphics rebuilds; CPU draw wall includes synchronous
renderer costs. FIFO presentation and one frame in flight limit interpretation.
The scene does not establish gameplay authority, character animation, general
streaming, independent per-view LOD or production performance capacity.
[Launcher qualification](validation/M4_STREAMED_BENCHMARK_LAUNCHER.md).

## Original procedural workload

This is an integrated rendering workload for M4, created following the request for
a small repeatable MMO-style benchmark sequence. It runs inside Render Lab using
the engine's Vulkan surface/swapchain, GPU compute/indirect task+mesh infrastructure,
visibility/depth targets and deferred resolve. No external artwork, proprietary
scene or native runtime library is needed. All scene geometry is procedural Slang.

The scene contains 256 rolling-ground patches, 64 village buildings and 128 tree
proxies, plus a configurable **0..10,000 moving player proxies**. Camera phases
approach the settlement, orbit through it, then sweep the crowd area. The fixed
layout and per-proxy trajectories are deterministic from object ID and logical
time. Camera phase transitions use smoothstep easing and continuous eye/target
positions. The default logical route is 60 seconds, sampled across 3600 measured
frames after 60 stationary warmup frames. Playback wall time depends on the
machine/display; shortening the frame count still samples the entire route.

Ground patches emit two surface triangles each, with identical shared world XY
vertices evaluated through the same height function. They form a continuous surface;
the earlier independent flat-box prototype was removed after visible stepped seams
were reported. This is procedural mesh-shader grouping, separate from the cooked
terrain meshlet/LOD streaming path. Bounds cover the patch's height variation.

A benchmark-only GPU font panel is enabled by default. It shows a 30-frame-smoothed
presentation-call FPS, previous completed CPU/GPU frame milliseconds, current
GPU frustum-visible objects/proxies, near/far detail counts, tracked allocation MiB,
and logical phase/time. Counts stay GPU-resident and are formatted in the resolve
shader. The font is a debug overlay, not general engine UI/text shaping.
`--benchmark-no-overlay` disables it in direct runs; the launcher accepts
`--no-overlay`. Reports record whether it was enabled.

These are low-poly visual proxies, with no skinning, collision avoidance, navigation,
combat, networking or gameplay authority. Motion may pass through buildings.
This version uses simple face shading/fog, not production lighting or GI. Its
near/far detail rule adds roof/head pyramids within 35 world units, rather than
representing general geometry LOD with validated approximation errors.

## Run

From the repository root, with Rust, a Vulkan 1.3 mesh/task-capable driver and
Slang/SPIR-V tools installed:

```sh
python3 tools/run_scene_benchmark.py --slangc /path/to/slangc
```

The launcher compiles/validates shaders, builds optimized Rust, then runs populations
100, 500 and 1000 sequentially. The default window is 1280x720; use `--width 1920
--height 1080` for a larger requested extent. Actual per-frame extents are recorded.
Use `--players 1000` for one case, or `--players 0 100 500 1000 10000` for a wider
sweep. `--frames 240 --warmup 30` is a quick full-route correctness run. `--validation`
enables actual Vulkan validation; keep those observations separate from runs with
validation disabled. `--skip-build --profile debug` uses the matching existing
debug binary and compiled modules.

Native desktop execution is the default.

Omit `--headless` to watch the normal window. For example, after the release build:

```sh
python3 tools/run_scene_benchmark.py --skip-build --players 1000
```

The sequence exits after the selected frame count; closing it early cancels report
completion. Headless execution is optional automation, not a restriction on the
benchmark. A live desktop route with 9000 frames is useful for a longer preview on
a high-refresh display; it still spans the same 60 seconds of logical scene time.

The existing private headless display dependencies also support unattended Linux runs:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
python3 tools/run_scene_benchmark.py \
  --slangc /home/steven/dev/tools/slang/bin/slangc \
  --headless --headless-dri-prime pci-0000_03_00_0 --gpu 7900
```

Those ICD/compiler/device paths describe the qualification machine, not portable
defaults. [Headless dependency setup](validation/HEADLESS_DISPLAY.md) is required
for that optional route. The launcher owns/cleans the compositor and never changes
desktop power settings. Native Windows execution is a separate qualification gate;
the Rust rendering/report path remains platform-neutral.

For direct execution after building:

```sh
apps/theia-render-lab/target/release/theia-render-lab \
  --benchmark --benchmark-players 1000 \
  --benchmark-report build/benchmark/direct.json --no-validation
```

Direct execution writes JSON only. Use the launcher for CSV and an interactive
HTML frame trace. Output is under ignored `build/benchmark/`: one population
directory per run, build/run logs, `report.json`, `report.csv`, `report.html`, and
an aggregate `suite.json`. JSON includes binary/shader hashes when launched by the
helper, device/driver identifiers, validation mode, sampling parameters, raw frame
samples and mean/p50/p95/p99/max milliseconds. A failed launched run removes its
old JSON report rather than retaining a stale success artifact.

## What the measurements mean

One frame is in flight and presentation uses FIFO. CPU draw time includes frame
fence waits, acquisition, command recording, submission and presentation API time;
it excludes event-loop gaps. Four queue timestamps bracket compute, visibility
and resolve with `ALL_COMMANDS` ordering. GPU durations include barriers and
synchronization inside those ranges; timestamp collection and JSON sample creation
add instrumentation overhead. Warmup samples remain in the trace but are excluded
from summaries. GPU values are not an uncapped FPS score or a production throughput
claim. Compare like-for-like populations, resolution, binary/shaders, validation,
display backend, driver and sampling configuration.

Overlay cost is included when enabled. Frame-interval samples include event-loop
gaps and support the displayed presentation-call cadence; GPU timing alone does
not predict that FPS. Frustum-visible counts include objects hidden by other
geometry, since no occlusion-culling system is qualified here.

Tracked allocation bytes include the dense work arena and owned identity/depth/
material images. They exclude swapchain/driver allocations, descriptor/query pools
and unrelated process resources, so are **not total VRAM usage**. The fixed arena
reserves capacity for 10,000 proxies in every population case; CPU initialization
does not allocate or issue draws per object. Compute generates bounded records,
culls conservative boxes, selects detail and compacts visible IDs; indirect task
groups consume that list without CPU count readback. Only timing queries return to
the host during ordinary benchmark execution.

This v1 holds its procedural scene resident. It does **not measure cooked-content
streaming, texture residency or streaming stalls**. Those already qualified M4
subsystems remain separate and should be integrated into a subsequent benchmark
revision with explicit scripted residency pressure. Animated characters and richer
lighting belong to the M5/M6 expansion; server-controlled bots and replication
load belong to the M7–M9 expansion. No milestone is closed by this workload.

## Replay qualification

`tools/renderdoc_benchmark.py` runs inside a dedicated `qrenderdoc --python`
process. `THEIA_BENCHMARK_CAPTURE_OUTPUT`, `THEIA_BENCHMARK_PLAYERS`, and
`THEIA_BENCHMARK_CAPTURE_FRAME` select an owned output and one frame of a 181-frame,
zero-warmup 60-second route. Capture uses the debug executable, ordinary Xwayland
presentation and disabled capture validation, separately from actual native
Wayland validation. It exports a preview PNG and checks every generated record,
CPU culling/detail/counters, bounded indirect work, whole-image identity/depth
invariants, and 25 independent world-space ray/triangle and deferred-colour probes.
The ray oracle uses GPU records after their separate CPU motion/extent checks, to
isolate rasterization from trigonometric differences. Replay data never feeds
runtime work sizing. Validation captures are not used as performance samples.

After building the debug executable and modules, run the complete qualification:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
XDG_DATA_HOME="$PWD/build/qualification/renderdoc-data" \
PYTHONPYCACHEPREFIX="$PWD/build/qualification/python-cache" \
python3 tools/qualify_scene_benchmark.py \
  --output build/qualification/scene-benchmark-final \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --headless-dri-prime pci-0000_03_00_0
```

The suite checks five populations, all three route phases, zero/dense crowd cases,
invalid pre-Vulkan plans, and a stats-enabled capture. The latter checks displayed
glyph pixels against original-run timings and current GPU counts. Geometry probes
outside the panel remain independent. Integer phase hashing bounds trigonometric
arguments; motion is checked within 0.001 world units, with exact kind/extent checks
apart from float representation tolerance. Culling and detail thresholds are
checked against the actual generated positions, avoiding a false mismatch when
small arithmetic differences cross a decision boundary. This is a qualified
numeric profile, not a cross-vendor conformance theorem.

## Cooked static-world source (offline prerequisite)

`tools/cook_benchmark_scene.py` generates four independently cooked quadrants of
the same 128×128 metre ground/village layout: 256 continuous ground tiles, 64
roofed buildings and 128 pyramid tree proxies, totalling 2048 triangles. Four
64×64 opaque diagnostic checker textures exercise material-selected sampling.
These are synthetic test assets, not production art. Tree geometry differs from
the procedural cube/roof proxy. Player motion remains in the existing benchmark.

```sh
cargo build --locked --manifest-path tools/theia-gltf-import/Cargo.toml
python3 tools/cook_benchmark_scene.py \
  --output build/qualification/benchmark-world \
  --importer tools/theia-gltf-import/target/debug/theia-gltf-import \
  --library build/qualification/meshoptimizer/lib-build/libmeshoptimizer.so \
  --meshoptimizer-source build/qualification/meshoptimizer/meshoptimizer-1.3.tar.gz \
  --ktx build/qualification/ktx-tool/source-build/Release/ktx \
  --ktx-build build/qualification/ktx-tool/source-build \
  --ktx-source build/qualification/ktx-tool/ktx-source-v4.4.2.tar.gz
python3 tools/qualify_benchmark_scene.py \
  --scene build/qualification/benchmark-world/scene.json \
  --output build/qualification/benchmark-world-proof
```

After building Render Lab, add `--gpu-smoke --headless-dri-prime
pci-0000_03_00_0` with `VK_DRIVER_FILES` selecting RADV to run owned headless
Wayland checks. These load each quadrant and alternate quadrants 0/3. Six pressure
cases use measured exact/one-byte-short GPU/image and deduplicated encoded CPU
budgets. They check publication versus clean deferral and zero diagnostics through
teardown. They use the existing small asset-viewer camera, not the benchmark route;
they do not establish whole-world framebuffer correctness or performance.

`--source-only --output <directory>` generates portable glTF/PNG sources without
the native toolchain. Full cooking uses the existing bounded Rust glTF importer,
pinned offline meshoptimizer and KTX tools, and unchanged indexed bundle formats.
The qualifier independently checks source/normalized positions, indices, UVs,
cooked winding/coverage/material assignments, texture bindings and every texel,
then runs strict Rust complete-bundle admission. Ground topology tests check exact
shared vertices and a complete rectangle across all quadrant boundaries.

Read `scene.json` for each hash-named bundle path. Failed cooking/qualification
writes a failed summary; it cannot leave a previous successful summary in place.
The existing texture-table provenance hashes source assignment paths: complete
bundle hashes repeat at a fixed output location, but change across output roots.
Geometry, material, UV and texture hashes repeat across roots with the same tools.
Encoded-byte totals are deduplicated file bytes, not physical GPU allocations.

The procedural `--benchmark` still uses its original GPU-generated scene. The
separate cooked route below consumes these assets. Simultaneous multi-quadrant
residency, a cooked world mixed with moving proxies, representative art, geometry
LOD and texture mip integration remain subsequent work.

## Cooked bundle route and timing observations

Use `--benchmark-cooked --bundle <indexed bundle>` directly in Render Lab, or
select a generated quadrant through the same build/export launcher:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
PYTHONPYCACHEPREFIX="$PWD/build/qualification/python-cache" \
python3 tools/run_scene_benchmark.py \
  --cooked-scene build/qualification/benchmark-world/scene.json \
  --quadrant 0 --stream-quadrant 3 \
  --frames 240 --warmup 30 --profile release \
  --slangc /path/to/slangc \
  --headless --headless-dri-prime pci-0000_03_00_0 \
  --gpu 7900 --output build/benchmark/cooked
```

Omit `--stream-quadrant` to retain one bundle. Replacement alternates two complete
bundles every twenty presented frames, using the existing bounded worker, fence
polling and conservative graphics-generation retirement. This is a controlled
replacement workload, not camera-driven world demand or simultaneous quadrants.
`--bundle-byte-budget`, `--bundle-gpu-budget` and `--texture-budget` forward the
existing CPU/GPU/image caps; insufficient coexistence capacity retains the active
complete bundle and reports deferrals. Add `--validation` for diagnostic runs.

The route and four-query timing recorder are shared with the procedural benchmark.
Reports identify `theia-cooked-bundle-route-v1`, source bundle paths, requested
budgets, driver/device context and final publication/deferral counts. No visual
player proxies or near/far geometry selection runs in this profile. The JSON
uses zero players and a null procedural static-object count. HTML describes the
actual cooked workload; JSON/CSV/HTML export and executable/shader digests remain
available through the launcher.

CPU draw and frame-interval measurements include publication stalls. GPU frame
queries exclude transfers queued before the first frame query, so they are not a
complete streaming GPU cost. The frame allocation trace counts work/targets;
active/pending cooked buffers and images remain separately charged by bundle
budgets and excluded from that trace. FIFO/single-frame and owned-display limits
still apply. These synthetic runs are not a production capacity or FPS score.

`tools/qualify_cooked_benchmark.py --scene <scene.json> --output <directory>
--qrenderdoc <executable> --headless-dri-prime <selector>` owns separate native
Wayland validation and Xwayland replay. It checks complete timing/warmup accounting,
exact/short budgets, invalid plans, the procedural regression and six cooked
captures. Replay checks immutable GPU buffers/texels, meshlet hull compaction,
whole-image identity/depth bounds and independent world-space ray/UV/texture colour
probes. At nearest-texel boundaries, a recorded ±0.002-texel coordinate envelope
permits only adjacent source texels; encoded RGB tolerance remains two bytes.
This bounded RADV numeric profile is not cross-vendor conformance.

## Complete resident cooked world

Add `--whole-world` to `cook_benchmark_scene.py` to retain the four original
quadrant assets and additionally cook one complete world bundle. It contains all
64 roofed buildings and 128 trees, with a continuous 8×8 ground grid over the same
128×128 metre rectangle. Equal position/UV rows weld within each material; authored
prop triangles, winding, UVs and material seams remain unchanged.

The finer 16×16 ground plus all props exceeded the current 32-meshlet viewer
profile in the evaluated cook. The complete representation instead has 1664
triangles, 1553 vertices and 28 meshlets. This is a fixed coarse-ground test asset,
not dynamic geometry LOD or independently resident quadrants. Ground topology
retains complete rectangular coverage and matching edges. The existing exact
continuous-heightfield checker gates vertical difference from the four-quadrant
fine ground at 0.12 metres; the qualified bound is 0.11387628316879274 metres.
This comparison is between represented triangular surfaces, not an analytic
terrain, silhouette, normal or general 3D error guarantee.

`scene.json.whole_world` records the complete asset, its own object/triangle counts
and error proof. Legacy top-level totals still describe the quadrant representation.
`qualify_benchmark_scene.py` additionally checks the whole asset and strict Rust
admission when this entry is present. Cooked formats and GPU limits are unchanged.

Select the complete resident asset through the existing launcher:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
PYTHONPYCACHEPREFIX="$PWD/build/qualification/python-cache" \
python3 tools/run_scene_benchmark.py \
  --cooked-scene build/qualification/benchmark-world/scene.json --whole-world \
  --frames 240 --warmup 30 --profile release --skip-build --validation \
  --headless --headless-dri-prime pci-0000_03_00_0 --gpu 7900 \
  --output build/benchmark/whole-world
```

The source cook must have included `--whole-world`. This selection cannot combine
`--quadrant` or `--stream-quadrant`. Launcher reports identify whole-world selection
and the exact source manifest hash; HTML labels the complete resident scene.
Direct Render Lab `--benchmark-cooked --bundle <whole-world bundle>` also works.

Add `--whole-world` to `qualify_cooked_benchmark.py` to run the resident world route
and three camera-phase replays. It checks the same immutable buffer/texel, hull,
identity/depth and ray/colour invariants. The original quadrant/pressure suite
remains its default. No new renderer/shader implementation, moving proxies,
dynamic LOD or independent world-chunk scheduling is added by this asset step.

## Cooked benchmark stats overlay

Cooked benchmark runs now enable an optional GPU debug panel by default, for both
quadrants and the complete resident world. Direct runs accept
`--benchmark-no-overlay`; the launcher accepts `--no-overlay`. Normal cooked asset
viewing outside benchmark mode keeps its existing pipeline.

The panel displays:

- Previous completed CPU/GPU frame times and a recent submission-cadence FPS value.
- Current conservative visible meshlets/total meshlets and resident texture count.
- `BUNDLE MIB`: active cooked geometry/material/UV/table buffers and images, from
  actual host-owned Vulkan allocation charges, with three decimal places.
- `DRAW MIB`: work/visibility/depth/dummy resolve-image allocations, with one
  decimal place, matching the existing draw-allocation trace.
- Completed bundle publications, budget deferrals and camera phase/time.

Meshlet counts come directly from the GPU work buffer and immutable geometry
header. No count readback sizes work. Other values are CPU-owned allocation/
publication metadata or already completed timing queries. Active bundle memory
excludes pending/staging/driver allocations; neither memory label is total VRAM.
Per-frame JSON `cooked` records and CSV columns retain active bundle allocation
bytes, texture count and publication/deferral totals. HTML records the overlay
setting; overlay cost remains included in timings.

The optional shader uses a separate fragment pipeline, five storage-buffer bindings
and a 112-byte push range: camera bytes 0..64, timing/phase bytes 64..96, and owned
bundle/count metadata bytes 96..112. Device limits are checked. The disabled path
uses the existing four-storage-buffer/64-byte resolve and does not need the HUD
module. Scoped compute/graphics barriers publish counts to the fragment stage and
cover their reuse. The same benchmark-only debug font is reused; this does not
replace the planned general UI/text system.

The cooked qualifier now includes resident, successful-publication and short-budget
deferral glyph checks, plus the original procedural HUD replay and both disabled
paths. Its `--whole-world` selection verifies the complete resident scene panel.

## Mixed resident world and visual proxies

The complete cooked village can now render alongside **0..10000 moving visual
proxies**, with shared visibility/depth and a combined stats panel. After cooking
`scene.json` with `--whole-world`, run from the repository root:

```sh
python3 tools/run_scene_benchmark.py \
  --cooked-scene build/qualification/resident-world-cook/scene.json \
  --whole-world --mixed-world --players 10000 --frames 9000 --warmup 60 \
  --slangc /home/steven/dev/tools/slang/bin/slangc
```

Use your actual cooked manifest and compiler paths. `--skip-build --profile release`
reuses matching binaries/modules; `--headless` runs the same route in the private
display. `--validation` enables validation and `--no-overlay` disables the panel.
Reports identify workload `theia-cooked-mmo-v1`, include cooked allocation metadata,
population and provenance, and export JSON/CSV/HTML. Multiple populations run
sequentially. Direct Render Lab uses `--benchmark-mixed --bundle <indexed-bundle>`
and `--benchmark-players N`; other primary benchmark modes and bundle replacement
are rejected in this first resident slice.

The immutable 28-meshlet village and GPU-generated crowd use two fixed indirect
mesh/task draws and separate 256-byte/376160-byte work arenas. Both write one
visibility/depth target, so buildings and proxies occlude one another. Identity
bit 31 distinguishes frame-local proxy IDs from cooked meshlet IDs; it does not
change the asset format or create persistent entity handles. Resolve uses five
fragment storage buffers without the HUD, six with it, and 64/112-byte push data;
proxy compute uses 96 bytes. Device limits are checked before creating pipelines.
The eleven-line HUD adds GPU visible/total proxies and near/far counts to cooked
stats. DRAW includes both work arenas and tracked targets; BUNDLE is the active
bundle allocation charge. Neither is total VRAM usage.

Proxy bases follow the resident world's 16-unit triangular ground interpolation.
The world uses a fixed, error-checked coarse mesh, not dynamic terrain LOD. Proxy
near/far detail remains the procedural 35-unit rule. There is no world collision,
navigation, skinning, networking or authority: proxies can pass through buildings.
The resident checker-texture village is a correctness fixture, not representative
production art, lighting, texture filtering or independent chunk residency.

## Separate world-chunk integration probes

World streaming prerequisites are qualified separately from the displayed mixed
benchmark: bounded CPU complete-bundle ownership, distance demand, a four-chunk
GPU upload ownership probe, then two resident geometry pages sharing visibility/
depth. The last uses `--world-pair-test <sha256.theia-mesh> <sha256.theia-mesh>`
with diagnostic tints; `--work-items 0|1|2` chooses empty/angled/full cameras and
`--wsi-smoke-test` exercises recovery. It renders no textures or visual proxies
and exports no benchmark timing score. These probes do not enable streamed world
chunks in `--mixed-world` yet.
[Two-chunk scope and qualification](validation/M4_WORLD_PAIR_RENDERING.md).


`--world-bundle-pair <sha256.theia-bundle> <sha256.theia-bundle>` additionally
composes two complete resident v2 graphs with their own textures. It uses the
same isolated fixed cameras, supports the three independent CPU/bundle/image
budget flags, and rejects streaming/benchmark modes. This is a world-rendering
integration probe, separate from the timed `--mixed-world` crowd workload.
[Complete-pair limits/evidence](validation/M4_WORLD_TEXTURED_PAIR.md).


For independent replacement in the separate pair probe, append
`--world-stream-bundle <replacement.theia-bundle>` and optionally
`--world-stream-report build/qualification/world-stream.json`. Chunk 0 stays
resident while chunk 1 alternates every twenty presented frames after bounded
I/O and fence-gated uploads; both remain visible during budget deferral. This
schedule is diagnostic and does not follow the camera-distance oracle.
[Lifecycle, caps and proof](validation/M4_WORLD_CHUNK_STREAMING.md).

## Native Bevy visual comparison

`tools/run_bevy_comparison.py` runs the current streamed Theia route without its
HUD, exports cooked resources and observed view/selection samples, then replays
the visual workload in a standalone Bevy 0.19.1 executable. It matches opaque
unlit world/proxy appearance, requested extent and FIFO presentation. Bevy uses
preloaded representations and CPU ECS proxy updates; this is not an equivalent
streaming implementation or a total-engine performance ranking.
[Usage and interpretation](../tools/theia-bevy-benchmark/README.md),
[executed evidence](validation/M4_BEVY_VISUAL_REFERENCE.md).

The comparison now also accepts `--stats-overlay`, enabling Theia's fixed GPU
HUD and a corresponding native Bevy UI text panel. `compare_stats_display.py`
records both engines with their displays disabled/enabled and filters summaries
to matching source selections. Their fonts, available metrics and pipeline
placement differ; no general UI ranking is implied.
[Stats profile and evidence](validation/M4_STATS_DISPLAY_COMPARISON.md).

Both comparison launchers now accept `--present-mode fifo|immediate|mailbox` and
optional `--gpu-timings` for frame-associated Bevy render-graph queries. The
standalone Theia launcher accepts the same presentation option. Exact support
checks reject unavailable modes; compositor pacing can still apply. GPU/CPU
scopes and Bevy's preload/replay policy remain explicit.
[Presentation and timestamp qualification](validation/M4_BEVY_PRESENTATION_TIMINGS.md).

A separate `--streaming-probe` reference profile now qualifies one asynchronous
real cooked fine load, GPU-ready publication, cancellation and retirement over
protected coarse coverage. The normal paired visual replay remains separate;
this fixed operation is not equivalent camera streaming or a performance run.
[Controlled lifetime gate](validation/M4_BEVY_FINE_STREAM_PROBE.md).
