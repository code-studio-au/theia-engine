# M4 procedural MMO scene benchmark qualification (2026-10-15)

The first integrated scene workload combines a scripted camera, continuous ground,
village/tree geometry and moving visual proxies with compute-generated frustum
compaction, indirect task/mesh draws, visibility/depth and deferred face shading.
An optional GPU debug-font panel reads counts directly from the work arena and
shows recent presentation-call FPS, completed CPU/GPU timings, detail counts and
tracked allocations. [Usage and precise scope](../SCENE_BENCHMARK.md).

This is a bounded procedural renderer test, not a production MMO scene, animation
system or network/load test. All content stays resident. It does not yet exercise
cooked content streaming, texture residency, general meshlet hierarchies, GI,
navigation or gameplay authority. Ten thousand executed visual proxies are not
evidence for ten thousand networked players.

## Cooked-world prerequisite and resource pressure (2026-10-15)

`build/qualification/benchmark-world-final/report.json` passes source-to-import
and cooked geometry/UV/material/texture binding checks, strict Rust admission of
four indexed bundles and 65536 decoded texel comparisons. Quadrants contain
624/624/619/629 vertices, 11/11/10/11 meshlets and 512/512/508/516 triangles.
All 29 offline Python tests pass, including three new scene topology/texture tests.
Ground edges form one 128×128 rectangle with identical shared vertices. Complete
bundles repeat at a fixed cook destination; table provenance makes bundle hashes
depend on the output root. Source/normalized and geometry/material/UV/texture
hashes agree across roots. A deliberately incorrect texture dependency rejects and
replaces a previous successful qualification summary with a failed summary.

Four native headless Wayland bundle loads and seven alternating-bundle runs each
present 120 frames with zero Vulkan warnings/errors including teardown. For
quadrants 0/3, active+pending buffer/image charge is 89376+89520 = **178896 bytes**;
image-only charge is **131072 bytes**, and unique encoded CPU owners cost
**117320 bytes**. Each exact cap permits five complete publications; each
one-byte-short cap produces five deferrals, zero publications and continued
presentation. These charges exclude work/targets/staging/driver allocations as
documented by the existing bundle-streaming contract.

These are synthetic opaque unlit nearest-mip-zero assets. Checks use the existing
asset-viewer camera; there is no whole-world pixel oracle, benchmark-route content
integration, multi-quadrant scheduler, crowd mixture, geometry LOD or performance
claim for this prerequisite. No renderer, shader or binary format changed.

## Independently executed

Linux Mint 22.3, RX 7900 XTX, RADV Mesa 26.2.4, Slang 2026.19, RenderDoc 1.46.
Owned Weston 13/GL uses compositor selector `pci-0000_03_00_0`. Actual standalone
validation uses native Wayland; ordinary Xwayland replay has capture validation
disabled. Desktop power settings remain untouched. Full Xwayland WSI recovery
and native Windows GPU qualification remain open.

The final suite `build/qualification/scene-benchmark-final/report.json` passes:

- Five actual Wayland populations: 0, 100, 500, 1000 and 10000 proxies. Each presents
  240 frames, with 30 warmup frames and a complete 60-second logical route. Actual
  layer activation and zero Vulkan warnings/errors through teardown are logged.
- Seven replay captures: all three phases for 1000 proxies, zero/small/dense crowd
  cases, and a stats-enabled mid-route capture. Every dense record's position,
  kind, extent and detail policy is checked, along with exact compacted ID sets,
  indirect bounds, counters and whole-image identity/depth invariants.
- 171 independent world-space ray/triangle, depth and colour probes. Terrain emits
  two surface triangles per patch, with shared edge coordinates and heights.
- 4375 stats glyph-pixel checks verify all nine displayed strings against original
  run timings and current GPU counters. Panel-covered geometry probes are excluded
  explicitly; geometry outside the panel retains the independent reference.
- Eight invalid population/frame/warmup/duration/extent/mode combinations reject
  before Vulkan. A separate negative check also rejects a negative player count.

| Proxies / captured frame | Visible objects | Near detail | Far detail | Maximum position error (world units) |
|---|---:|---:|---:|---:|
| 1000 / 10 | 1424 | 20 | 1404 | 0.00002242 |
| 1000 / 90 | 1024 | 618 | 406 | 0.00003242 |
| 1000 / 170 | 1331 | 417 | 914 | 0.00004292 |
| 0 / 90 | 289 | 10 | 279 | 0.00002242 |
| 100 / 170 | 434 | 47 | 387 | 0.00004003 |
| 10000 / 170 | 10316 | 4110 | 6206 | 0.00004959 |

GPU and CPU initially differed around a detail cutoff because fused arithmetic
changed a generated position slightly. The reference now tests decisions on the
actual generated records, separately from motion accuracy. Large angular inputs
also exposed avoidable float drift; integer phase hashing bounds those arguments
for both props and proxies. The final motion cap is 0.001 world units, not the
earlier looser tolerance. Depth probes permit 0.00002 and colour probes two encoded
RGB bytes. These are explicit diagnostic bounds, not cross-vendor guarantees.

Native X11 desktop previews were separately executed for 9000-frame routes with
zero Vulkan diagnostics, including visible stats and continuous ground. Those
previews preceded the final static-prop phase-hash adjustment; the final executable/
modules are qualified by the headless suite above. No replay/desktop-preview timing
is presented as an uncapped rendering score.

Formatting and denied-warning Clippy pass. Renderer tests pass **27**, including
timeline continuity, bounded deterministic motion, invalid configurations and
matching ground edges/conservative corner bounds. Default and shared shader
interfaces each compile/validate **38 modules**; the previously qualified specific
GLSL shared-primitive fallback remains confined to its original module. No new
Slang fallback is introduced. Windows GNU cross-link/Clippy passes all eleven Rust
packages and Wine runs **154 CPU tests**; native Windows/MSVC/GPU remains open.

Local logs: `scene-benchmark-final/`, `benchmark-complete-tests.log`,
`benchmark-complete-clippy.log`, `benchmark-complete-{shared-}shaders.log`, and
`windows-scene-benchmark/` under ignored `build/qualification/`. Preview PNGs and
raw capture files are local artifacts; no generated binaries/captures are committed.

The automatic build/run/export launcher also completes three optimized native
Wayland headless routes with 100/500/1000 proxies, 3660 frames each and 60 excluded
warmup frames. Validation and overlay are disabled for these separate baseline
observations. All JSON samples/summary values are finite, the full route ends at
logical time 60, and CSV/interactive HTML plus binary/shader provenance are exported
under `build/benchmark/headless-baseline/`. No numeric headless FPS is claimed as
desktop performance; source, driver, presentation and instrumentation context are
retained in each report.

Preserved indirect rendering, asynchronous mip-tail refinement and sixteen-image
whole-bundle streaming each re-run 120-frame native Wayland WSI recovery with zero
diagnostics through teardown. Refinement publishes once and bundle streaming five
times. The original headless compute app also verifies all 256 GPU values on the
RX 7900 XTX. Logs are `scene-benchmark-regressions/` and
`scene-benchmark-compute.log` under `build/qualification/`. Their sources/shaders
remain unchanged; benchmark integration preserves these existing paths.

The fixed work arena requests 376160 bytes for maximum capacity even in small
cases. Tracked image/work allocations at 1280x720 measure 12369768 bytes on this
driver. This excludes swapchain/driver/descriptor/query allocations and is not
total VRAM usage. FIFO presentation, one frame in flight and instrumentation costs
must remain explicit when interpreting reports. M4 stays open.
