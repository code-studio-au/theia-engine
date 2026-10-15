# Theia procedural MMO scene benchmark v1

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

**The procedural `--benchmark` has not yet adopted these bundles.** World rendering,
multi-quadrant residency/pressure, representative art, geometry LOD and texture mip
integration remain subsequent work. This offline result is not streaming or GPU
performance evidence.
