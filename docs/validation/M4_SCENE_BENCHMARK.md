# M4 procedural MMO scene benchmark qualification (2026-10-15)

## Mixed resident village and crowd (2026-10-15)

`build/qualification/mixed-world-final/report.json` passes eight native Wayland
240-frame routes: populations 0/100/500/1000/10000, a HUD-disabled mixed route and
preserved cooked/procedural routes. All have zero Vulkan diagnostics through
teardown and complete timing/warmup accounting. Seven invalid plans fail before
Vulkan work. Nine separate RenderDoc 1.46 captures cover three route phases,
zero/100/1000/10000 proxies and three HUD cases. Capture validation is disabled;
standalone native routes activate actual Vulkan validation.

Replay checks all proxy poses/extents/kinds, both GPU compactions, immutable cooked
buffers/textures, whole-image identity/depth bounds, and 439 independent ray/colour
probes choosing the nearer world/proxy intersection. Both occlusion directions
are exercised. Three mixed panels pass 15575 glyph pixels. Original cooked and
procedural capture tools also pass after extracting their reusable CPU models.
The motion tolerance is 0.001 world units, depth 0.00002 and encoded RGB two bytes;
qualified nearest-texture boundary probes retain the existing 0.002-texel envelope.

Renderer tests pass 28, including an independent barycentric check of the mixed
coarse-ground interpolation. Offline Python tests pass 31; formatting and
warning-denied Clippy pass. Both default/shared interfaces compile and validate
41 SPIR-V modules, with no new language fallback. Windows GNU cross-build/Clippy
passes eleven packages and Wine CPU qualification passes; native Windows/GPU is
still unqualified. Logs are `mixed-world-{build,clippy,tests,python-tests,shaders,
shared-shaders}.log` and `windows-mixed-world/report.json` under ignored
`build/qualification/`.

At 1280x720 the qualified driver's tracked targets plus two work arenas request
12370024 bytes, while the active whole-world bundle charge is 120256 bytes.
These are scoped allocation charges, not total VRAM. World geometry stays fixed
at 1553 vertices/28 meshlets/1664 triangles. Proxies use the coarse triangular
terrain model, can intersect buildings and have no gameplay authority. This
resident integration does not close M4 or establish MMO network capacity.

The optimized launcher separately completes a validation-enabled 10000-proxy
240-frame headless route with 30 excluded warmup frames, zero diagnostics and
JSON/CSV/HTML exports in `build/benchmark/mixed-release-10000/`. The original
headless compute test re-verifies all 256 values (`mixed-world-compute.log`).

## Cooked benchmark live stats (2026-10-15)

`build/qualification/cooked-hud-world/report.json` and
`build/qualification/cooked-hud-quadrants/report.json` pass **17 native Wayland
runs × 240 frames**, including both HUD-disabled paths, all quadrants, the resident
world, exact/short CPU/GPU/image caps and the preserved crowd route. Every run has
zero Vulkan warnings/errors through teardown and complete timing/warmup accounting.

Thirteen cooked replay captures pass immutable GPU buffers/texels, hull counts,
whole-image identity/depth and **560 ray/colour probes**. Four panel captures pass
**16625 glyph pixels**: whole-world resident (4165), quadrant resident (4165),
successful replacement (4165) and short-cap retention (4130). The successful
capture reports three publications/zero deferrals and the replacement's 89520
active allocation bytes; the short case reports zero publications/three deferrals
and retains the initial 89376 bytes. Whole-world active bundle charge is 120256
bytes. GPU meshlet/total counts, texture count, both memory labels, timing digits,
publication/deferral digits and phase/time match the captured inputs. Each suite
also replays the original crowd HUD, passing **4375 glyph pixels** each. Capture
validation is disabled and is separate from native Wayland validation evidence.

The shared debug font stays intact. The new optional fragment shader uses five
storage bindings and 112 push bytes; the no-overlay path keeps the old pipeline.
The work buffer's publication/reuse barriers cover fragment reads. A missing HUD
module rejects before presentation with zero diagnostics and orderly teardown;
the same shader directory succeeds for three frames with the HUD disabled.
`cooked-hud-cleanup/report.json` records both checks.

Default/shared interfaces compile and validate **39 modules each**, retaining the
previous narrowly qualified GLSL primitive-interface fallback. Rust renderer 27
tests, offline Python 31 tests, formatting and denied-warning Clippy pass.
`windows-cooked-hud/report.json` passes eleven GNU cross-builds/Clippy and 154 Wine
CPU tests; native Windows/GPU remain unqualified. The original compute executable
again verifies all 256 values on RX 7900 XTX.

The optimized `cooked-hud-release/cooked-whole-world/report.json` launcher run
passes 240 headless frames with overlay enabled and validation disabled. Its JSON,
CSV and HTML preserve active bundle metadata and overlay context. This is a labeled
instrumented observation, not an uncapped FPS or production capacity score.
Active bundle bytes exclude pending/staging/driver allocations; draw allocations
retain their separate scope. No gameplay authority, mixed crowd/world rendering,
dynamic LOD, world-demand scheduling or general text/UI implementation is added.

## Complete resident cooked world (2026-10-15)

`build/qualification/resident-world-proof/report.json` passes strict Rust admission
and source/import/cooked checks for the four preserved quadrants plus an optional
complete world bundle. The world has **1553 vertices, 28 meshlets, 1664 triangles**
and four diagnostic textures. Topology/winding/UV tests preserve all 64 buildings
and 128 trees, with 64 continuous ground tiles covering the full 128×128 rectangle.
All 31 offline Python tests pass. Removing a coarse ground triangle rejects.

The exact continuous-heightfield checker reports a maximum vertical difference
of **1910527/16777216**, rounded upward to **0.11387628316879274 metres**, against
the original finer quadrant ground. It passes a 0.12-metre limit over 2506 overlay
pairs/4234 vertices. This is between represented triangular surfaces; it does not
bound analytic/silhouette/material/general 3D error. A forged proof field rejects.
Repeated cooking at the same destination reproduces the full manifest and all
hashes; existing quadrant geometry/material/UV hashes remain unchanged across
roots. Three incompatible launcher selections reject before launch.

`build/qualification/resident-world-final/report.json` passes 240 native headless
Wayland frames with zero Vulkan warnings/errors through teardown. Three separate
Xwayland/RenderDoc captures at frames 10/90/170 verify all four GPU buffers,
49152 aggregate GPU texels, hull compaction and whole-image identity/depth bounds.
All **96 ray/colour probes** pass across four materials, including 32 probes with
the existing explicit nearest-texel boundary envelope. The replay preview shows
the complete static village together. Captures are separate from validation.

The optimized `resident-world-launcher/cooked-whole-world/report.json` run also
passes 240 frames and exports JSON/CSV/HTML with complete-world labeling and source
manifest digest. The prior desktop quadrant preview finishes all 9000 frames and
zero diagnostics. No renderer, shader, Rust, binary format or platform API changed
in this unit. This is one resident coarse-ground bundle: independently streamed
quadrants, moving proxies, dynamic LOD and the cooked HUD remain unimplemented.

## Cooked-bundle route and timing integration (2026-10-15)

`build/qualification/cooked-route-final/report.json` passes twelve native headless
Wayland runs of 240 frames: four resident quadrants, alternating quadrants 0/3,
six exact/one-byte-short CPU/GPU/image caps, and the original procedural 1000-proxy
route. Each has zero Vulkan warnings/errors through teardown, 240 samples, 210
measured frames and final logical time 60 seconds. Exact caps permit publication;
short caps retain the active complete set and record deferrals. Five invalid plans
reject before Vulkan; both orders of mixed benchmark flags separately reject.

Six separate Xwayland/RenderDoc captures cover quadrants 0/3 at frames 10/90/170.
All four immutable GPU buffers and 16384 GPU texels per capture match the cooked
files. Compacted meshlet IDs/masks match an independent hull-plane oracle;
whole-image identities/depth obey bounds. **286 world-space ray/UV/texture probes**
match rendered identities/depth and encoded RGB within two bytes, covering all
four material bindings. Eighty probes use the documented ±0.002-texel boundary
envelope: only immediately adjacent source texels are candidates. No wall probes
are silently excluded. This numeric profile does not prove arbitrary coordinate
accuracy or cross-vendor sampler behavior. Capture validation is disabled; actual
validation evidence comes from the native Wayland runs.

`build/qualification/cooked-route-release/suite.json` passes a complete 240-frame
optimized launcher run with JSON/CSV/interactive HTML, executable/shader digests,
headless context and validation disabled. It is a labeled timing observation,
not an uncapped FPS score. The Rust renderer has 27 passing tests; formatting,
denied-warning Clippy and all 29 offline Python tests pass. The default 38 shader
modules compile/validate during the release launch; no shader source changed.
`build/qualification/windows-cooked-route/report.json` passes eleven GNU package
cross-builds/Clippy and 154 Wine CPU tests; native Windows/GPU remain unqualified.

Reports distinguish the cooked workload, zero procedural players, null procedural
object count, bundle paths/budgets and final publication/deferral counts. No cooked
HUD is enabled. Frame allocation traces exclude active/pending cooked resource
allocations; those retain separate physical caps. GPU frame timestamps exclude
transfers submitted before query recording, while CPU/frame intervals include
publication stalls. The periodic whole-bundle replacement is unrelated to camera
demand. Simultaneous quadrants, a cooked/crowd mixture, general geometry LOD and
texture mip integration remain open.

## Procedural workload

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
