# Theia — Actual Implementation Status (2026-10-15)

**This file separates observed evidence from planned systems.** The original handoff
records user-supplied terminal evidence. Entries below record independently executed local qualification; the latest
update is 2026-10-15.

## M4 independent GPU chunk-upload ownership — locally qualified (2026-10-15)

An isolated Render Lab probe now uploads four independently owned complete cooked
chunks: 16 root buffers and 16 images, 357472 physical bytes on RADV, including
262144 image bytes. CPU owners deduplicate to 165148 encoded bytes. Transfer
fences retire before publication; cancelled ready sets release, retained readers
block physical reuse, and last-owner release permits an exact-budget replacement.
One-byte-short aggregate and image caps retain the first three complete chunks
when the fourth fails. This is a synchronous ownership probe with no chunk draws,
GPU content readback or graphics readers. The mixed benchmark remains unchanged.
[Evidence](validation/M4_WORLD_UPLOAD_OWNERSHIP.md).

## M4 bounded CPU chunk-distance demand — locally qualified (2026-10-15)

A stateless outdoor XY-distance oracle now prioritizes bounded chunk demand from
actual residency, with separate load/retention radii and explicit slot deferral.
It performs no I/O, eviction or GPU work. Four policies pass 686 samples each
using the benchmark camera route plus boundary excursions and real quadrant
bounds; every desired/load/remove/deferred set, CPU publication epoch and encoded
byte charge matches an independent model. All 67 asset tests and denied-warning
Clippy pass, including 2000 integer-lattice oracle queries and failed-request
simulation. The mixed renderer remains one resident world bundle; protected
fallback coverage and independent GPU chunk publication remain next.
[Contract](M4_ASSET_CONTRACT.md#bounded-outdoor-chunk-distance-demand-oracle),
[evidence](validation/M4_WORLD_DEMAND.md).

## M4 independent CPU world-bundle residency — locally qualified (2026-10-15)

A separate bounded world owner now admits several complete bundles against one
shared encoded-byte pool and a chunk-slot cap. Atomic named chunk add/replace/
remove preserves retained world/chunk snapshots and rejects malformed/foreign/
over-capacity transactions without changing publication. Four real cooked
quadrants qualify 165148 unique bytes (372748 counted separately), exact/short
budgets, independent removal/replacement and last-reader release. All 63 asset
CPU tests and denied-warning Clippy pass, including 4000 randomized world
operations against an independent resource-union model. Camera demand and GPU
chunk publication remain open; the mixed renderer retains one resident bundle.
[Contract](M4_ASSET_CONTRACT.md#independent-complete-bundle-cpu-world-ownership),
[evidence](validation/M4_WORLD_BUNDLE_RESIDENCY.md).

## M4 mixed resident village and crowd — independently executed (2026-10-15)

The complete cooked village and 0..10000 GPU visual proxies now share visibility,
depth and deferred resolve. Two fixed indirect draws retain separate GPU work
arenas; the live panel adds visible/total proxies and near/far counts. Eight
240-frame native Wayland cases pass zero diagnostics, and nine mixed RenderDoc
captures pass all proxy records, both compactions, identity/depth, 439 independent
ray/colour probes and 15575 HUD glyph pixels. Preserved cooked/procedural replay
regressions pass. Renderer 28 tests, offline Python 31 tests, denied-warning Clippy,
both 41-module shader interfaces and Windows GNU/Wine qualification pass.
The fixed resident ground is coarse; proxies have no navigation, collision,
skinning or gameplay authority. Independent world-chunk residency, representative
art/LOD/mips and remaining M4 gates stay open.
[Usage](SCENE_BENCHMARK.md#mixed-resident-world-and-visual-proxies),
[evidence](validation/M4_SCENE_BENCHMARK.md#mixed-resident-village-and-crowd-2026-10-15).

## M4 cooked benchmark live stats — independently executed (2026-10-15)

Cooked quadrants and the complete resident world now have an optional live panel:
GPU meshlet counts, timings, active bundle/draw allocation charges, textures and
publication/deferral counters. Four cooked panels pass 16625 glyph pixels, including
successful replacement and retained content under short capacity; the original
crowd HUD passes 4375 pixels in each of two suites. Seventeen 240-frame native
Wayland cases pass zero diagnostics, with 13 cooked captures/560 ray-colour probes
plus two procedural captures. Both shader interfaces validate 39 modules; renderer
27 tests, Python 31 tests, formatting/Clippy, Windows GNU eleven packages/154 Wine
CPU tests and the original compute test pass. Missing optional shader cleanup and
the HUD-disabled path pass. No desktop window is launched for this qualification.
[Usage](SCENE_BENCHMARK.md#cooked-benchmark-stats-overlay),
[evidence](validation/M4_SCENE_BENCHMARK.md#cooked-benchmark-live-stats-2026-10-15).

## M4 complete resident cooked world — independently executed (2026-10-15)

The asset pipeline now optionally cooks the entire village into one admitted
bundle: 64 buildings, 128 trees and continuous coarse ground, 1664 triangles in
28 meshlets. Exact continuous vertical error against the finer ground is bounded
by 0.114 metres under a 0.12-metre gate. Prop geometry/UVs are preserved. All 31
offline Python tests and strict Rust admission pass; headless native Wayland and
optimized launcher each complete 240 frames with zero diagnostics. Three camera
replays pass immutable buffers/texels, hull compaction, image bounds and 96 ray/
colour probes. This renders the whole static scene together as one resident
bundle; independent quadrant streaming, moving proxies and the cooked HUD remain
open. No runtime/shader or binary-format changes.
[Usage](SCENE_BENCHMARK.md#complete-resident-cooked-world).

## M4 cooked-bundle benchmark route — independently executed (2026-10-15)

An explicit cooked mode now reuses the benchmark camera/timing recorder with the
existing meshlet/visibility/textured resolve path. It renders one admitted bundle
at a time, optionally alternating two through bounded I/O/upload/publication.
Eleven 240-frame cooked runs plus the preserved procedural route pass zero Vulkan
diagnostics, complete warmup accounting and exact/one-byte-short CPU/GPU/image
budgets. Six replay captures verify immutable GPU buffers/textures, culling and
identity/depth plus 286 ray/colour probes across all four material bindings.
Renderer 27 and offline Python 29 tests, denied-warning Clippy, release launcher,
Windows GNU eleven packages and 154 Wine CPU tests pass. Cooked content and moving
proxies are still separate workloads; simultaneous quadrants, world-demand
scheduling, representative art/LOD/mips and the cooked HUD remain open.
[Workflow](SCENE_BENCHMARK.md#cooked-bundle-route-and-timing-observations),
[evidence](validation/M4_SCENE_BENCHMARK.md#cooked-bundle-route-and-timing-integration-2026-10-15).

## M4 integrated procedural scene benchmark — independently executed (2026-10-15)

The next offline prerequisite now cooks a four-quadrant static-world fixture:
2048 triangles, four diagnostic textures and unchanged indexed bundles. Each
quadrant has 10–11 meshlets within the existing 32-meshlet profile. Independent
source/import/cooked coverage, winding, UV, material/texture mapping and texel
checks plus strict Rust bundle admission pass. This source fixture is not yet
connected to the procedural benchmark or world streaming.
[Workflow](SCENE_BENCHMARK.md#cooked-static-world-source-offline-prerequisite).

A repeatable three-phase camera route now renders continuous ground, 64 buildings,
128 tree proxies and 0..10000 moving visual proxies through GPU culling/detail,
indirect task/mesh draws and visibility/deferred resolve. An optional on-screen
stats panel reads GPU counts directly; the launcher builds and exports JSON/CSV/
interactive frame traces. Five 240-frame Wayland populations pass zero diagnostics;
seven replay captures pass all-record/compaction/image checks, 171 ray/colour probes
and 4375 stats glyph pixels. Renderer 27 tests, both 38-module shader interfaces,
Windows GNU eleven packages and 154 Wine CPU tests pass. This resident procedural
workload does not yet measure cooked-content streaming, skinning, GI or MMO server
load. [Usage](SCENE_BENCHMARK.md), [evidence](validation/M4_SCENE_BENCHMARK.md).

Three optimized 3660-frame headless routes for 100/500/1000 proxies also export
complete reports with 60 warmup frames excluded, validation/overlay disabled and
explicit presentation/backend context. These are initial observations, not an
uncapped FPS score or a production engine capacity claim.

## M4 asynchronous mip-tail refinement — independently executed (2026-10-14)

One bounded finer-suffix request now reads an immutable source on a background
worker, submits an owned transfer and publishes only after fence readiness and
conservative graphics-generation retirement. Default/shared replay preserves the
coarse image during transfer and under insufficient coexistence capacity; changed
sources are rejected. Twelve captures also cover protected-1x1 to full-chain
refinement. The 507→508 dense-cap case measures 1536+1792 image bytes:
3328 publishes, 3327 retains. Native Wayland runs have zero Vulkan diagnostics;
Linux renderer/assets 23/59 tests and Windows GNU eleven packages/150 Wine CPU
tests pass. Tiled feedback, production scheduling and multi-frame reuse remain
open. [Evidence](validation/M4_ASYNC_MIP_REFINEMENT.md).

## Headless asset qualifier integration — independently executed (2026-10-14)

Fixed and automatic mip qualifiers now own private display environments: actual
Wayland validation/budget/cleanup and Xwayland replay stay distinct. Linear tail-5
and native generated fixed views pass six captures/504 pixels, all-level/probe
checks and zero-diagnostic allocation boundaries. Injected caller failure removes
the private runtime/server. No Rust/shader changes; Xwayland full recovery remains
open. [Evidence](validation/HEADLESS_ASSET_QUALIFIERS.md).

## Headless display qualification — independently executed (2026-10-14)

Owned Weston/GL and Xwayland now execute GPU tests with the desktop monitor off.
Eight native Wayland WSI modes and eight ordinary Xwayland modes each pass 120
frames/zero diagnostics. Six replay captures verify visibility counts 0/1/2 and
linear mip-tail selection: 501 pixels and 243 diagnostic records. The Xwayland
recovery smoke times out after frame 80 and remains explicitly unqualified; native
Wayland recovery passes. No Rust/shader or desktop power/configuration changes.
[Evidence](validation/HEADLESS_DISPLAY.md).

## M4 native filtered mip cooking — independently executed (2026-10-14)

Pinned offline KTX now generates bounded power-of-two opaque sRGB box-filtered
chains from a lossless PNG envelope. Five sources/31 levels preserve exact bases
and match independent linear-light per-step averages with observed zero RGB error;
strict Rust admission and repeated cooking pass. Thirteen producer tests and six
GPU fixed/automatic captures pass, with allocation-boundary cleanup and zero
standalone Vulkan diagnostics. No Rust/shader changes. General filtering quality,
odd/HDR/alpha/import profiles, tiled demand and async tail publication remain open.
[Evidence](validation/M4_FILTERED_MIP_COOKING.md).

## M4 GPU coarse mip tails — independently executed (2026-10-14)

Conventional images now contain only a protected coarse suffix, while GPU analytic
footprints retain original source dimensions and sample image-relative levels.
Twenty-four tail captures qualify exact/short texel boundaries, 1x1-only/full tails,
shared interface, odd/vertical shapes and numeric fallback. Physical image caps and
post-upload cleanup pass zero diagnostics; horizontal allocations measure 9472/
1792/1536/256 bytes for full/tail-4/tail-5/1x1. A precision-aware CPU envelope handles
one dark bilinear pixel using the device's queried eight-bit weights. Renderer/
assets 23/59 tests, both shader variants 33 outputs, seven Wayland modes, Windows
GNU 11 packages and 150 Wine CPU tests pass. Tiled demand, async tail publication,
source mip generation/quality and native Windows GPU remain open.
[Evidence](validation/M4_GPU_MIP_TAIL.md).

## M4 CPU coarse mip-tail contract — independently executed (2026-10-14)

An immutable borrowed mip-tail plan keeps the finest complete coarse suffix within
a dense texel-byte budget, protects 1x1 and maps source levels to resident image
levels. Exact/short budgets, exhaustive small odd-size cases, thin chains and
forged inputs pass; assets 58 tests, formatting/Clippy and 156 native cooked-chain
plan checks pass. GPU adoption/physical allocation qualification remains next.
[Evidence](validation/M4_MIP_TAIL_CONTRACT.md).

## M4 GPU bilinear sampling — independently executed (2026-10-14)

Opt-in linear filtering within explicit automatic integer mips now qualifies
checkerboard/alternating texels, mip-zero numeric fallback, both shader interfaces,
all-level GPU bytes, gradients and colour/depth references. Twelve captures pass;
exact/one-byte-short image budgets and missing-shader cleanup pass zero diagnostics.
Seven nested-Wayland modes pass. Assets/renderer 54/23 tests, producer eight tests,
formatting/Clippy, 69 existing SPIR-V modules and Windows GNU 11-package cross-build/
Clippy with 145 Wine CPU tests pass. Production filtering quality, trilinear/
anisotropy, tiled residency and native Windows GPU remain open.
[Evidence](validation/M4_GPU_LINEAR_FILTER.md).

## M4 CPU bilinear sampling reference — independently executed (2026-10-14)

A bounded ideal bilinear-repeat oracle decodes sRGB before interpolation and
qualifies centres, seams, thin/odd levels, invalid inputs and 1000 independent
periodic-kernel samples. All 54 assets tests, formatting/Clippy and 102 cooked-chain
sample probes pass. GPU filtering adoption remains next; nearest renderer behavior
is preserved. [Evidence](validation/M4_LINEAR_TEXTURE_REFERENCE.md).

## M4 GPU automatic mip selection — independently executed (2026-10-11)

Complete resident mip views now use Slang analytic perspective UV gradients and
explicit footprint-based mip choice, with mip-zero fallback outside the supported
numeric profile. Default/shared thin-chain and native wide-quad replay verify all
levels, 81 GPU diagnostic records per capture and colour/depth references. Exact/
one-byte-short image budgets and post-upload missing-shader cleanup pass zero
Vulkan diagnostics. Preserved modes, seven nested-Wayland WSI modes, renderer 23
and producer eight tests pass. Both shader variants validate 32 outputs; Windows
GNU 11-package cross-build/Clippy and 141 Wine CPU tests pass. Filtering quality,
tiled/partial residency, full-chain replacement and native Windows GPU remain open.
[Evidence](validation/M4_GPU_AUTOMATIC_MIPS.md).

## M4 CPU UV gradients and texture footprint — independently executed (2026-10-11)

A bounded analytic perspective UV-gradient model and explicit isotropic mip policy
now qualify affine/scaling/winding cases, 1000 independent finite-difference cases,
repeat seams, exact thresholds and guards. The cooked-scene CPU oracle checks 255
framebuffer probes; covered small textures select mip 0, thin texture mips 4/5.
GPU selection is not implemented in this unit. Assets/renderer tests 50/23, formatting/
Clippy pass; Windows GNU 11 packages and 141 Wine CPU tests pass. Shader arithmetic,
automatic GPU mip choice, filtering and tiled fallback remain next/open.
[Evidence](validation/M4_UV_FOOTPRINT_CONTRACT.md).

## M4 GPU authored mip chains — independently executed (2026-10-11)

Explicit fixed-mip viewing now uploads complete admitted chains into one Vulkan
image, copies all levels with full-range barriers and uses a one-level view with
the existing Slang shader. Twelve square/odd/thin/shared captures verify all uploaded
mip bytes and 85 pixels each. Per-view WSI, exact/one-byte-short physical image budgets
and preserved indirect/authority/16-image streaming pass zero diagnostics. Invalid
mip index/alpha reject before Vulkan setup. Native Wayland nine-mode execution passes;
Windows GNU 11 packages and 137 Wine CPU tests pass. Renderer/assets tests 23/46,
formatting/Clippy and active SPIR-V checks pass. Automatic mip choice, gradients,
filtering and tiled/partial or full-chain replacement remain open; no performance or
native Windows GPU claim. [Evidence](validation/M4_GPU_TEXTURE_MIPS.md).

## M4 authored texture mip-chain contract — independently executed (2026-10-11)

A separate bounded pure-Rust reader admits complete opaque RGBA8 sRGB KTX2 chains,
1..11 levels with exact ranges/layout/metadata/hash/alpha checks and borrowed texels.
Pinned offline KTX writes explicit authored levels; square, odd rectangle and eleven-
level thin files pass exact source checks, official validation and independent Rust
admission. Repeated cooking is identical. Assets 46/46, renderer 22/22, eight producer
tests, formatting/asset Clippy pass; Windows GNU assets/examples and 46 Wine tests pass.
Legacy single-mip admission stays separate; 16-image WSI streaming passes zero
Vulkan diagnostics and the legacy path cleanly rejects mip-chain files. GPU mip
upload/sampling, filtering/derivatives/automatic choice and tiled fallback remain next.
CI configuration added, hosted unexecuted. [Evidence](validation/M4_TEXTURE_MIP_CONTRACT.md).

## M4 GPU LOD hysteresis — independently executed (2026-10-11)

The two resident heightfield pages now retain GPU-owned level history with explicit
first-dispatch initialization and read/write synchronization. A stricter coarsening
threshold preserves the original hard-limit refinement and clip/profile guards.
Default zero fraction preserves stateless behavior. Sixteen default/shared trajectory
captures and four guard/disabled/zero-error captures verify work and 81 pixels each.
Standalone full trajectories/WSI and preserved indirect/texture/authority/compute
regressions pass zero diagnostics. Native Wayland execution passes eight WSI modes
plus a full trajectory. Assets/renderer tests 42/22, both shader variants 31 outputs,
formatting/Clippy pass; Windows GNU 11-package cross-build and 132 Wine CPU tests pass.
No hierarchy/page demand, morphing, general 3D error or performance claim. Native
Windows/cross-vendor and broader M4 gates remain open.
[Evidence](validation/M4_GPU_LOD_HYSTERESIS.md).

## M4 CPU LOD hysteresis contract — independently executed (2026-10-11)

A bounded two-page selection oracle delays fine-to-coarse transitions using a
stricter entry threshold while preserving the original hard-limit refinement and
clip guards. Caller-owned view/page history resets to Fine; fraction zero preserves
the existing stateless policy. Three new tests include a 10000-step independent
orthographic state machine. Assets 42/42, renderer 22/22, formatting and asset Clippy
pass. GPU history/adoption remains the next unit; no runtime/performance claim.
[Evidence](validation/M4_LOD_HYSTERESIS_CONTRACT.md).

## M4 maximum texture capacity on Wayland — independently executed (2026-10-11)

Native Wayland surfaces in owned nested Weston execute all four 16-image bundles,
complete-set replacements and exact/one-byte-short CPU/GPU/image budgets. Together
with six preserved renderer modes, 15 runs pass 120 frames each with zero Vulkan
diagnostics. Exact replacement publishes five times and adds five graphics generations;
short limits defer five times and retain the initial set. The new harness's initial
seven-generation assumption was corrected to include replacement generations.
Capacity replay/pixel proof remains X11-only; programmatic Wayland restore, native
seat/Windows/cross-vendor and performance gates remain open. Weston retains its
own shutdown diagnostic. [Evidence](validation/M4_WAYLAND_TEXTURE_CAPACITY.md).

## M4 full-capacity texture replacement — independently executed (2026-10-11)

Two 16-image complete sets now qualify asynchronous upload/publication and exact
coexistence budgets: 22088 unique encoded CPU bytes (independent hash-union check),
141760 GPU root/image bytes and 131072 image bytes. Exact limits publish five times;
one byte below each limit independently defers five times and retains the initial
complete set through 120 frames. All four validation runs pass zero diagnostics.
Four replay captures verify retained/replaced material sampling, all 16 image texels
and 85 pixels each. No Rust/shader changes, runtime readback or performance claim.
Device-idle retirement, general streaming/LOD/virtual texture and platform gates
remain open. [Evidence](validation/M4_TEXTURE_STREAMING_CAPACITY.md).

## M4 maximum resident texture table — independently executed (2026-10-11)

Actual indexed sampling now qualifies all 16 descriptor slots on RX 7900 XTX/RADV/X11.
Four replay captures each verify all uploaded image texels and 85 framebuffer probes;
triangle-centroid probes cover every descriptor index. Separate WSI smoke passes
120 frames/ten generations with zero diagnostics. The measured 65536-byte image-only
budget passes exactly; one byte less rejects the final image with clean validation
teardown. Complete repeated cooking is identical, five existing producer tests pass.
No runtime/shader changes or performance claim. Concurrent capacity replacements,
aggregate bundle budgets at this capacity, other platforms/vendors and broader M4
gates remain open. [Evidence](validation/M4_TEXTURE_CAPACITY.md).

## Windows GNU build/compatibility qualification — independently executed (2026-10-11)

All 11 Rust CI packages cross-link normal/test/example targets and pass denied-
warning Clippy for x86_64-pc-windows-gnu. Wine 9 executes 129 CPU/library tests,
plus normal authority movement/combat/request and readonly script executables.
This is GNU cross-build/compatibility evidence, not native Windows/MSVC/GPU evidence.

The relay's oversize test exposed Winsock WSAEMSGSIZE: truncated UDP receive returns
error 10040 instead of a truncated length. That packet now drops and the relay
continues; targeted Wine test logs the actual recovered error, Linux 11 shared tests
and encrypted fault/replication regressions pass. Available Wine's encrypted Quinn
startup rejects an IPv6-only query on an IPv4 socket (10045), so that compatibility
exchange remains unqualified; native Windows behavior is not inferred. No transport/
crypto fallback or dependency patch. [Evidence](validation/WINDOWS_BUILD.md).

## Composited Wayland pixel reference — independently executed (2026-10-11)

The owned nested compositor now captures its own output via Weston's screenshot
API. An offline analytical reference locates the unscaled 960x540 canvas and checks
GPU-generated indirect counts 0/1/2, interpolated RGB/sRGB and clear background.
Three actual 300-frame Wayland runs pass zero diagnostics; screenshots verify
7973/7932/7888 sampled pixels, including absent/right-present geometry. Six WSI
modes also pass again. No Rust/shader changes in this unit.

This is composited display evidence, not RenderDoc replay (the available build still
rejects Wayland surfaces). Debug screenshot access exists only in the owned temporary
compositor; no main desktop capture/system installation. Weston logs its own layer
finalization diagnostic on termination; no clean-compositor-shutdown claim. Native
seat/manual restore, Windows/cross-vendor and performance gates remain open.
[Evidence](validation/WAYLAND_PIXELS.md).

## Nested Wayland platform qualification — independently executed (2026-10-11)

An isolated Weston 13 compositor nested on X11 now executes Theia's actual Wayland
Vulkan surface path on RX 7900 XTX/RADV. Six standalone modes each pass 120 frames/
seven swapchain generations with zero diagnostics, plus two encrypted full/delta/
repair viewers under packet faults and WSI recovery. Backend logging confirms
Wayland. X11 ten-generation/minimize-restore and 256-value compute regressions pass.

Initial smoke stalled after minimizing: pinned Winit cannot unminimize Wayland
programmatically. Smoke now explicitly skips that round trip and still verifies
resize, zero-size deferral and labeled recovery injection. Native user-driven
minimize/restore remains unqualified. RenderDoc 1.46 portable build rejects
VK_KHR_wayland_surface, so capture replay remains unqualified, not passed. Renderer
22 tests/formatting/Clippy pass; no shader changes. This is nested-compositor
reference evidence, not native-seat/Windows/cross-vendor or performance evidence.
[Evidence](validation/WAYLAND_NESTED.md).

## M8 actual authority/live GPU deltas — independently executed (2026-10-11)

Live authority now prepares bounded per-recipient full/delta TDR1 states, selecting
only application-acknowledged baselines. Clients reconstruct before atomic readonly
publication, check the exact bootstrap roster and send snapshot ACK/full-repair
DATAGRAMs. Movement and reliable combat retain independent sequence domains.
Explicit revision THN3/ALPN `theia-lab-3` rejects older peers; no fallback guessing.

Faulted two-GPU 180-frame run and 600-frame native combat pass full/delta publication,
four acknowledged hits/death and zero Vulkan diagnostics. Explicit ACK-withholding
qualification forces actual missing-baseline/full repair during faulted 120-frame/
ten-generation WSI runs, both zero diagnostics. Repair replay matches 32387 sampled
pixels; closed replay verifies 32400 background pixels. Laboratory profile, separate
probes, direct/faulted gameplay and isolated replication regressions pass. Shared/
renderer tests 11/22, formatting/Clippy pass. Windows/Wayland/hosted CI unexecuted;
no persistence/accounts/general roster/reconnect/WAN/load/performance claim.
M7/M8 remain open. [Evidence](validation/M8_LIVE_DELTA_REPLICATION.md).

## M8 isolated encrypted delta exchange — independently executed (2026-10-11)

Separate network lab `--replication-test` now exchanges TDR1 full states/deltas and
application baseline ACK/full-repair DATAGRAMs through certificate-verified Quinn.
A synthetic complete-state source is checked independently on both clients; this
unit does not replace gameplay authority. Receiver history eviction deliberately
triggers repair while ACKs are withheld; unknown baselines never partly publish.

Direct run and three opaque packet-fault runs reach final tick 120 on both clients,
with at least one missing-baseline repair each, exact readonly states and bounded
sender/receiver/relay history. Shared nine tests, app formatting/Clippy and preceding
encrypted gameplay fault regression pass. No live viewer adoption or production
protocol negotiation yet; THN2 listener/viewer still sends full state. Next integrate
the qualified baseline/repair contract with actual authority and GPU clients using
an explicit protocol revision. Windows/hosted CI unexecuted.
[Evidence](validation/M8_ENCRYPTED_REPLICATION.md).

## M8 bounded replication envelope codec — independently executed (2026-10-11)

Separate TDR1 codec encodes full sets, full-actor deltas, snapshot ACKs and repair
requests with explicit little-endian fields, strict tags/order/values and a 128-byte
limit. Counts are checked before allocation; at most two actors per list. Encoder
rejects invalid caller values without asserting. Largest valid envelope is 88 bytes.

Shared/protocol/renderer tests 9/23/22 and encrypted THN2 fault regression pass;
formatting/Clippy pass. New codec tests cover every truncation/extension, explicit
bytes, count/ID/range/order/overlap/version failures and 20000 arbitrary-byte inputs.
The codec is not yet sent over Quinn; existing THN2 listener/viewer remain full-set
only. Next qualify isolated certificate-verified encrypted replication exchange,
then adopt the qualified contract in live viewers. Windows/hosted CI unexecuted.
[Evidence](validation/M8_REPLICATION_CODEC.md).

## M8 acknowledged sender baselines — independently executed (2026-10-11)

A transport-neutral per-recipient sender retains 2..16 complete prepared sets and
selects a delta baseline only after explicit application acknowledgement. Unknown/
future/evicted ACKs cannot create baselines; stale and previous-connection contexts
cannot advance them. Eviction falls back to full state. Repair requests force full
state until a newer complete set is acknowledged, fencing delayed old ACKs.

Protocol tests 23 pass; a 10000-generated-packet loss/reordering oracle verifies
both state and ACK paths, bounded sender/receiver history, 316 full repairs and
final complete tick 10001. Renderer 22 tests, preceding delta example and encrypted
fault regression pass; formatting/Clippy pass. THN2 still sends full sets: no wire
ACK/delta/repair exchange yet. Next add explicit versioned bounded messages and
qualify encrypted baseline/repair exchange before updating live presentation.
Windows/hosted CI unexecuted. [Evidence](validation/M8_ACKNOWLEDGED_BASELINES.md).

## M8 transport-neutral delta reconstruction — independently executed (2026-10-11)

A bounded full-actor delta prototype names its exact complete baseline tick,
canonical upserts/removals and target tick/watermark. Readonly publication accepts
only a retained endpoint, reconstructs atomically and preserves state when the
baseline is missing. Full snapshots explicitly repair history; source checks and
monotonic ticks/watermarks remain mandatory. THN2 transport still sends full sets.

Protocol tests 20 pass, including 3000 independent sparse-membership round trips,
lifecycle/limits/conflicts/reset/baseline eviction. A 10000-generated-packet CPU
loss/reordering example checks every publication against complete-state values,
exercises 2922 missing-baseline repairs and verifies final full tick 10001. Renderer
22 tests and encrypted fault regression pass; formatting/Clippy pass. This unit
has no baseline ACK producer/cache policy, wire delta or network repair exchange.
Next qualify a bounded sender's explicitly acknowledged baseline selection before
wire integration. Windows/hosted CI unexecuted. [Evidence](validation/M8_DELTA_RECONSTRUCTION.md).

## M8 live GPU packet faults — independently executed (2026-10-11)

Render Lab `--network-faults` reuses the bounded shared opaque UDP relay after
verified bootstrap. Two separate GPU viewers qualify real drops/reorders/duplicates,
180-frame and 120-frame/ten-generation WSI execution, zero diagnostics and bounded
relay retention. Native input still produces four acknowledged server hits/death;
faulted replay matches 32387 sampled pixels. Direct GPU and encrypted headless
fault regressions pass. Shared/renderer tests 6/22 and formatting/Clippy pass.

Cancellation clears presentation before bounded transparent close drain and joins
the relay. No WAN/load/handshake-loss/reconnect or performance claim; Windows/
Wayland/hosted CI unexecuted. M8 remains open. Next qualify transport-neutral
bounded delta reconstruction and missing-baseline full-state repair before wire
integration. [Evidence](validation/M8_LIVE_PACKET_FAULTS.md).

## M8 separate encrypted GPU viewers — independently executed (2026-10-11)

Render Lab `--network-client` now connects to the separate Quinn listener using
its verified public certificate. Server-owned bootstrap selects actor/target and
scene profile; readonly snapshots feed one 64-byte push/three-group mesh draw.
A bounded network thread sends directional intents and one reliable request at
a time. The live path creates no local gameplay `World`. Unknown/closed actor
identities and absent scene geometry produce no mesh output.

Two RX 7900 XTX viewers pass 180 frames and separate 120-frame/ten-generation WSI
runs with zero diagnostics. Native keyboard input produces four acknowledged
server hits/death over 600 frames. Five RenderDoc replays verify fixed/live demo/
laboratory/unpaired/closed pixels, including empty state and profile geometry.
Renderer/protocol tests pass 22/16; formatting/Clippy and both 31-output shader
variants pass. Fixed/indirect WSI and preserved 256-value compute regressions pass.
Seven invalid startup cases reject before surface creation. Windows/Wayland/hosted
CI unexecuted. No WAN/load, account authentication, deltas, reconnect, persistence
or performance claim. M7/M8 remain open. Next qualify the live viewer under bounded
post-handshake packet faults. [Evidence](validation/M8_NETWORK_VIEWER.md).

## M8 separate listener and actor bootstrap — independently executed (2026-10-11)

`theia-network-lab --serve` binds localhost and writes only its public DER
certificate; signing keys stay in memory. Two independent `--client-probe`
processes verify that certificate, receive server-owned actor/target/profile
bootstrap, bind snapshot reception to their exact retained connection and send
sequenced movement. The pair ends when a peer disconnects. Fixed laboratory and
local-demo scene profiles are server-selected; this is not an account service.

Prototype version is now THN2/ALPN `theia-lab-2`; old version/unknown profile/
duplicate bootstrap IDs fail. Four shared codec tests, one app relay test,
formatting/Clippy pass. Direct/faulted two-client gameplay still passes two hits/
health [75,75], and foreign-connection reception is rejected before consumption.
Independent process probes qualify roles 10/20, direction/applied watermark and
health 100 for both scene profiles. A six-second delayed second peer passes with
one-second keepalive; non-loopback/malformed CLI cases reject. Windows/hosted CI
unexecuted. Next connect this encrypted bootstrap/state to the GPU viewer.
[Evidence](validation/M8_NETWORK_LISTENER.md).

## M8 shared encrypted transport fixture — independently executed (2026-10-11)

The already qualified strict wire, TLS configuration, two-actor authority loop
and readonly client receiver now live in `crates/theia-network-fixture`; the
network lab imports them. The bounded UDP fault proxy stays in the qualification
app. This is explicitly a shared prototype fixture, not a production network API.

Three shared codec tests and one app relay test pass; formatting/Clippy pass.
Both direct/faulted encrypted runs preserve exactly two hits/health [75,75],
certificate/ALPN rejection, request/ACK repair and readonly interpolation samples.
No wire/security/gameplay behavior changed. Next add a separate listener/client
bootstrap before connecting the viewer; live network presentation remains open.

## M7 interactive local authority demo — independently executed (2026-10-11)

`--authority-demo` accepts physical WASD/arrows movement and Space attack requests;
server-owned `World` decides position, collision, cooldown/health/death. Focus loss
clears held input and attacks; aliases track independently. A separate 50 ms
Winit timer advances at most four fixed steps per wake, visibly skipping excess
wall intervals. Readonly snapshots feed the existing 64-byte GPU interpolation
profile; no client/GPU gameplay authority or network connection is introduced.

The 600-frame scripted target-GPU run completes 100 ticks/four hits/death/four
static blocks with zero diagnostics. Native XTest D/Space input runs complete
four hits/death over 600 frames with zero diagnostics. Separate 120-frame/ten-
generation WSI qualification passes; replay matches 32372 sampled pixels including
dead-target gray and the blocked actor pose. Twenty renderer tests and formatting/
Clippy pass; four legacy WSI modes and malformed options pass. Initial 120-frame
script was too short for its 26-tick collision phase; frame limits were preserved
and longer gameplay qualification was run. M7 remains open. Next connect the live
encrypted authority feed to presentation. [Evidence](validation/M7_INTERACTIVE_AUTHORITY.md).

## M7 fixed GPU authority presentation — independently executed (2026-10-11)

Separate Render Lab `--authority-test` consumes a fixed two-tick movement/combat
fixture from `theia-sim`, validated through the readonly replica/presentation
oracle. One 64-byte push transfers two previous/current positions, split 64-bit
logical IDs, current health, blend mask and alpha. Slang mesh groups interpolate
positions and generate two actor squares plus one static obstacle; one draw,
no vertex buffer or per-actor host draw. Gameplay remains CPU/server-owned.

RX 7900 XTX default/shared authority modes pass 120 frames/ten swapchain generations
with zero diagnostics. Three RenderDoc captures (alpha 0/.5/1) match 32371/32371/
32387 sampled pixels to CPU positions/health colours, checking IDs above 32 bits.
Sixteen legacy modes also pass 120-frame/WSI validation; preserved compute verifies
256 values. Both shader variants compile/SPIR-V validate 31 outputs. Renderer
15 tests, formatting/Clippy and malformed-option/missing-shader cleanup pass.
This is a fixed snapshot fixture, not interactive or connected to Quinn. Next
add bounded interactive local authority controls before a live network feed.
[Evidence](validation/M7_GPU_AUTHORITY_PRESENTATION.md).

## M8 bounded presentation interpolation — independently executed (2026-10-11)

The readonly replica now retains two complete sets and provides a separate CPU
presentation oracle. Explicit integer tick/fraction avoids large-clock precision
loss. Matching logical-ID positions blend within retained endpoints; latest
health/membership stay exact. New actors and observed dead-to-alive transitions
snap to latest. Large gaps snap; outside-history times hold endpoints. No
extrapolation, prediction, clock estimator or authority mutation is introduced.

Four new tests bring the protocol suite to 16, including 3000 independent convex-
weight comparisons, lifecycle/gap/reset checks and near-u64-max clock samples.
Both encrypted modes pass sampled presentation from received state: direct
88/16 samples and faulted 92/16, with two hits/health [75,75]. App tests (four),
formatting/Clippy pass. This bounded CPU oracle is not a production GPU renderer.
Next inspect and qualify a small renderer integration. [Evidence](validation/M8_PRESENTATION_INTERPOLATION.md).

## M8 readonly client snapshot publication — independently executed (2026-10-11)

The transport-neutral `theia-protocol::replica` now publishes complete readonly
client state by server tick. Newest validated sets replace atomically; older sets
are stale and identical same-tick sets are idempotent. Contradictory same-tick
state or regressing processed movement watermarks fail without changing the
current state. Locally issued connection contexts reject old/foreign delivery;
reset/disconnect clear the prior replica. Logical IDs are independent of rows.

Five new tests bring the protocol suite to 12, including a 10000-delivery loss/
reordering oracle, complete lifecycle replacement, validation/limits/exhaustion,
connection reset and spare-allocation normalization. Both direct and faulted
encrypted two-client labs now consume snapshots through this store and observe
health [75,75]. App tests (four), formatting/Clippy and the 10000-tick request
regression pass. No interpolation/prediction, delta baseline or renderer connection
yet. Next qualify bounded presentation interpolation using separate clocks.
[Evidence](validation/M8_CLIENT_SNAPSHOTS.md).

## M8 encrypted packet fault qualification — independently executed (2026-10-11)

`theia-network-lab --faults` routes each established Quinn connection through a
bounded localhost UDP relay. Directional ordinal rules drop every eleventh,
hold every seventh (up to 10 ms), and duplicate every thirteenth eligible packet.
A successor overtaking a held packet explicitly reorders it. Relays do not decode
or modify encrypted payloads and hold at most one packet per direction.

The faulted two-client scenario preserves exactly two hits/health [75,75], reliable
cached results/conflicts/ACK repair and independent movement sequences. Movement
retries at 50 ms until a processed watermark appears in repeated full snapshots.
Four unit tests and formatting/Clippy pass; the relay test verifies both routes,
unknown-source rejection and oversized-packet rejection without retained growth.
Default loopback still passes. This is controlled post-handshake loopback loss,
not WAN congestion, handshake-loss qualification or scale measurement. Next
qualify client snapshot ordering/session reset before presentation integration.
[Evidence](validation/M8_NETWORK_FAULTS.md).

## M8 encrypted two-client loopback — independently executed (2026-10-11)

A separate `theia-network-lab` exchanges bounded versioned messages through
Quinn 0.11.11/rustls/ring. Two TLS certificate-verified local clients send
supersedable movement DATAGRAMs and independent reliable combat/ACK streams;
the single-writer authority publishes full two-actor snapshot DATAGRAMs. ALPN
matches explicitly, 0-RTT is disabled and certificates/keys stay in memory.
Server connection context binds actors; no account authentication is present.

IPv4 runs verify range rejection, movement application, two authoritative hits,
immutable duplicate results, conflicts, window/ACK-hole repair, stale inputs,
malformed messages and isolation of a stalled stream. Both clients observe
health [75,75]. Three codec tests include all message truncations/extensions,
explicit enum/ID/numeric limits and 20000 arbitrary-byte cases. Formatting and
Clippy pass. IPv6 bind fails because this host disables IPv6; Windows/hosted CI
remain unexecuted. Loss/reordering, deltas/baselines, persistence and renderer
integration remain open. [Evidence](validation/M8_QUINN_LOOPBACK.md).

## M8 bounded request adapter — independently executed (2026-10-11)

`theia-protocol` separates external supersedable movement sequences from reliable
combat request IDs, assigning private simulation input sequences. Pending and
completed retries reuse their result; conflicting target reuse fails. A bounded
request window retains unacknowledged results and out-of-order ACK tombstones.
Application outcome ACKs advance a contiguous floor; pressure rejects new work
without silent eviction. Peer contexts are server-issued and session scoped.

Seven adapter tests pass. The typed-message lab executes 10000 ticks/40000
requests, verifies 110000 duplicate replies, 40000 payload conflicts and 20000
window rejections; health 930000 matches an independent one-hit-per-tick model.
Movement/combat regressions and formatting/Clippy pass. This is transport-neutral:
no sockets, wire format, authentication, durable effects or reconnect continuity.
Next qualify encrypted Quinn DATAGRAM and reliable-stream exchange with two
local clients. M8 remains open. [Evidence](validation/M8_REQUEST_SEQUENCING.md).

## M7 bounded authoritative combat — independently executed (2026-10-11)

The separate authority store now owns health, damage, point-target range,
static-rectangle line of sight and cooldown. Clients request a target/sequence;
server connection context determines the attacker. Movement completes before
combat; admitted attack order is deterministic. Only successful hits consume
cooldown; death stops held movement and rejects further actions. Outcomes carry
request sequences; final health changes coalesce once per actor/tick. Snapshot
values remain readonly copies and logical IDs survive compaction.

Sixteen library tests pass. The headless combat lab executes 5000 ticks/9921
requests: 948 hits, 8973 cooldown rejections, final health 93364, matching an
independent model. Range/LOS/self/missing/dead targets, same-tick death/order,
replay, target despawn and counter failures pass. The 100000-tick movement run
retains its preceding totals. Formatting/Clippy/app tests pass. M7 remains open;
no networking, mesh navigation, hit volumes, inventory or renderer integration.
Next establish independent movement/reliable-request sequencing and bounded
idempotent outcome handling before Quinn transport.
[Combat evidence](validation/M7_AUTHORITATIVE_COMBAT.md).

## M7 parallel authoritative movement slice — independently executed (2026-10-11)

`theia-sim` is a separate single-writer Rust authority prototype. Server-owned
rules/static collision/session bindings accept sequenced directional intents,
move each actor once per fixed tick, enforce speed and swept world collision,
and expire missing input. Dense SoA columns keep logical server-supplied world
IDs independent of rows; compaction and character respawn preserve identity,
while stale/foreign session contexts fail. Ordinary actors overlap freely.
Collision is an analytic flat floor plus static rectangles; renderer LOD/GI is
never gameplay truth. Session binding is not authentication/networking.

Nine tests pass, including 5000 independent segment-intersection comparisons
and 5000 three-actor ticks. The headless server lab executes 10000 and 100000
ticks (30000/300000 admitted inputs), verifies maximum movement <=150 mm/tick,
collision and replay/malformed-intent rejection. Formatting and Clippy pass.
Prototype units/rules are not engine-wide or wire-format decisions. No combat,
navigation/mesh collision, networking, persistence or renderer integration yet.
Next add bounded server-owned interaction/combat rules. M7 remains open;
M4/general hierarchy/virtual textures and M5 GI also remain open/deferred.
[Movement evidence](validation/M7_AUTHORITATIVE_MOVEMENT.md).

## M4 GPU heightfield LOD selection — independently executed (2026-10-11)

Separate `--lod-test <fine coarse>` keeps two source-linked geometry pages
resident. Compute selects the page from projected vertical error, culls its
meshlets and generates indirect task/mesh work; the mesh stage reads that same
selection. There is no frame-time host selection/count readback. Two fixed
storage bindings avoid a new descriptor-array requirement. Compute/mesh push
blocks are 112/64 bytes, explicitly separated and device-checked.

Nine LOD configurations pass 120-frame/ten-generation synchronization validation
with zero diagnostics; 16 legacy renderer configurations also pass. Twelve
captures verify GPU page choice, both encoded buffers and culling/work; eleven
pass 81 depth/colour samples each. The near-plane case verifies work/choice
only because the CPU raster oracle cannot clip triangles. CPU-forbidden coarse
choices never occur in the tested cases; a threshold case correctly keeps fine
because of GPU numerical margins. The two-page physical charge is 13232 bytes;
exact budget passes, 13231 safely rejects the second allocation. Missing shader
and malformed source/options clean up or fail before Vulkan as appropriate.

Both shader variants compile/SPIR-V validate 29 outputs. Renderer/assets tests
pass 14/39, formatting/Clippy pass, bundle-stream replay passes 85 samples and
headless compute verifies 256 values. This is a bounded, resident two-level
heightfield slice, not a general hierarchy/streamed LOD or material-error system.
M4 remains open. [GPU LOD evidence](validation/M4_GPU_LOD_SELECTION.md).

## M4 projected heightfield LOD CPU selection — independently executed (2026-10-11)

A separate pure Rust oracle bounds projected correspondence error for two
continuous heightfields enclosed by a common AABB. It accounts for perspective
W changes as well as X/Y displacement, evaluates box corners with outward
rounding, and keeps fine LOD at Vulkan clip boundaries or unbounded projection.
Producer float bounds convert upward to represented f32. `--lod-reference`
checks real source links, common position bits/counts/profile and reports the
CPU choice before Vulkan initialization. It does not recompute the offline proof.

Thirty-nine asset tests pass, including analytic cases, perspective denominator
changes, conversion/clip/extreme-numeric guards and 3000 independent sampled
point correspondences. The real 128-triangle terrain chooses coarse at a 10-pixel
budget/960x540 (Y bound 5.03307), fine at 1920x1080 (10.06614) or the narrow
camera (16.47987). Flat terrain qualifies zero error/budget. Three malformed
bindings/budgets fail without Vulkan. Renderer tests (14), formatting and
Clippy pass. No GPU LOD selection is implemented yet; that is next. Details:
[projected-error evidence](validation/M4_PROJECTED_LOD_ERROR.md).

## M4 continuous heightfield LOD error — independently executed (2026-10-11)

The offline terrain simplification gate now supplements sampled checks with
exact rational triangle-overlay evaluation. For bounded continuous rectangular
XY heightfields it validates full coverage, no interior overlaps and matching
shared-edge heights, then finds the maximum vertical difference over every
overlay polygon vertex. Input represented f32 positions are exact rational values;
the report includes the exact error fraction and an upward-rounded float bound.
This is not a general 3D Hausdorff/silhouette/material error or runtime proof.

Terrain reductions 512->256 and 512->128 triangles qualify bounds approximately
0.00497735 and 0.01225427 position units; flat reduction has zero error. Each
passes Rust admission and 120-frame/ten-generation GPU validation without
diagnostics. Two captures pass 81 depth/material samples. Eighteen Python tests
pass, including 100 analytic diagonal-change comparisons. Repeated cooking is
identical; a sampled-permitted tighter limit fails the continuous gate. Original
base fixture hashes remain unchanged. Next define bounded projected-error LOD
selection; general hierarchies and virtual textures remain open. Details:
[continuous-error evidence](validation/M4_CONTINUOUS_HEIGHT_ERROR.md).

## M4 complete indexed-bundle streaming — independently executed (2026-10-11)

`--bundle <v2> --stream-bundle <v2>` alternates two complete fixed graphs.
One bounded file worker loads roots/leaves; complete CPU reservation precedes
four buffer and all image submissions. The frame loop polls every dedicated
fence and retains the current set until all uploads complete. After its sole
frame fence retires, it rebuilds a graphics generation with the entire new set,
then publishes the CPU epoch. One/two-image descriptor-count changes pass.
Generation retirement currently uses device idle; no overlap/performance claim.

Six 120-frame/WSI cases qualify five complete replacements each (default,
shared, reverse, exact CPU/physical/image budgets). Three one-byte-short budget
cases retain the current set and record five deferrals each. Actual physical
charges: two-image set 12704, alias set 8416; coexistence needs 21120 bytes and
12288 image-only bytes. CPU coexistence shares one texture and charges 11012.
Four captures verify initial/replaced/restored/shared states at 85 samples each.
Late missing source/shader and initial-budget failures clean up without Vulkan
diagnostics. Renderer checks (14), formatting/Clippy and preserved headless
compute (256 values) pass. M4 remains open: general LOD/error/hierarchy,
terrain integration and mip/virtual texture work are still outstanding. Details:
[streaming evidence](validation/M4_BUNDLE_STREAMING.md).

## M4 submitted texture-set uploads — independently executed (2026-10-11)

Resident image uploads now expose submit, readiness and completion ownership.
The indexed initializer submits every image before any completion wait. Each
pending upload retains staging/image/command-pool/fence owners; cancellation
retires its dedicated fence before destroying resources. The single-image
wrapper preserves its preceding behavior. This enables later streaming; startup
still waits for the complete set and no frame-time bundle replacement is claimed.

Five RX 7900 XTX 120-frame/ten-generation configurations pass synchronization
validation without diagnostics (default, alias, shared, v1 and exact budget).
The 8191-byte case rejects the second image after submitting the first and safely
retires cancellation with zero diagnostics. Three independent captures pass 85
samples each. Renderer tests (14), formatting and denied-warning Clippy pass.
See [texture evidence](validation/M4_TEXTURES.md).

## M4 complete-bundle CPU residency — independently executed (2026-10-11)

A separate `theia_assets::bundle_residency` store admits every v1/v2 root and
texture leaf before reserving ownership. Staging reserves the complete set or
fails; identical encoded resources share owners. CPU publication replaces the
whole set at a checked epoch. Pending stages and retained reader snapshots
remain charged until their last owner releases them. The budget includes
manifest bytes and excludes caller I/O, decoded temporary data and VRAM.

Thirty-three asset tests pass, including five complete-set tests and a 3,000-step
independent live-byte-union oracle. Real two-image/alias bundles charge
6528/5416 bytes separately, 11012 together (932 shared); v2/v1 bundles charge
6528/4912 separately, 10508 together (932 shared). Both examples verify budget
rejection, atomic current-set preservation, reader retention and last-owner
reuse. Existing geometry-only residency is preserved. GPU upload completion and
atomic descriptor publication remain the next gate. See
[residency evidence](validation/M4_BUNDLE_RESIDENCY.md).

## M4 authored multiple-image bundle — independently executed (2026-10-11)

Opt-in glTF `--unlit-textures` imports distinct material-selected external PNGs,
deduplicates exact encoded image bytes, and preserves source bindings in
provenance. Schema 5 cooks the existing geometry/factor/UV containers, KTX2
children and dense texture table. THEIABND v2 replaces the fourth root with
that table; complete root and texture-child admission precedes Vulkan setup.
Version 1 and manual indexed paths remain available.

Two-image default/shared and one-image alias captures pass 85 CPU/GPU samples
each. Four standalone 120-frame/ten-generation cases (two-image bundle, alias,
legacy bundle, manual table) pass synchronization validation without diagnostics.
Both fixture images match all 32 known source RGBA bytes after offline decode.
Missing leaves and five malformed normalized sets fail; repeated cooking is
identical. Assets/importer/renderer/Python checks pass 28/12/14/13 tests.
No asynchronous complete-resource-set publication is implemented yet; that is
the next bounded M4 unit. Details: [texture evidence](validation/M4_TEXTURES.md).

## M4 GPU material texture selection — independently executed (2026-10-11)

`--texture-table` admits fixed geometry/factors/UVs and the complete indexed
texture set before Vulkan initialization. It uploads hash-sorted resident images
and the immutable table once; Slang selects the descriptor from the resolved
material index with NonUniformResourceIndex. Device descriptor/image/storage
limits and aggregate physical image-memory budget are checked. Existing
single-image/bundle/diagnostic paths remain available.

RX 7900 XTX passes 19 standalone 120-frame/ten-generation validation cases with
zero diagnostics: fourteen preceding configurations plus imported default,
shared, terrain and exact one/two-image budget cases. Two images charge 8192
physical bytes; 8191 rejects the second upload and releases the completed first
image safely. Missing shader cleanup passes; four malformed sets fail before
Vulkan initialization. Seven indexed captures verify GPU dependency bytes/texels,
work/identity/depth and CPU colours; terrain checks all 512 visible triangles and
material/texture mappings [1,0,1]. Original single-image/M3 replay also passes.

Fourteen renderer tests, formatting/denied-warning Clippy and both 27-output
shader builds/SPIR-V validation pass. Disassembly verifies runtime descriptor
array/nonuniform sampled-image capabilities and explicit mip-zero sampling.
GPU execution qualifies one/two textures, not every supported table count.
Authored multiple-image normalization/bundling and asynchronous residency remain
open. Next: bounded multiple authored texture import. No push/benchmark claim.
[Evidence](validation/M4_TEXTURES.md).

## M4 bounded material texture table — independently executed (2026-10-11)

THEIATEX v1 binds an exact material-page hash to up to 16 hash-sorted immutable
texture dependencies and one dense texture index per material. Pure Rust checks
counts/budgets/digests/padding/UTF-8, index ranges, duplicate/unreferenced textures
and every child KTX2/opacity/provenance. A two-material/two-texture file-backed
example passes with deliberately reversed material/descriptor order.
Three new tests bring the asset suite to 27; 13 Python tests and formatting/
denied-warning Clippy pass. Repeated cooking is identical; independent source-
order permutation preserves physical material-to-texture choices. This is CPU
admission only; GPU indexed table lookup and multiple authored image import are
not implemented by this unit. Next: bounded GPU material texture selection.
[Contract](M4_ASSET_CONTRACT.md), [evidence](validation/M4_TEXTURES.md).

## M4 bounded authored PNG import — independently executed (2026-10-11)

`--unlit-texture` now imports a shared external base-colour PNG for opaque unlit
materials with authored float2 UVs and explicit nearest-repeat sampling.
Normalized schema 4 carries exact encoded PNG bytes/hash and source bindings;
the cooker validates mapping/dimensions, uses pinned KTX offline to decode the
bounded image, and emits the existing four-role bundle. No runtime image decoder
or asset binary version change. Full fixture decoded texels match all 16 known
source bytes. Ten normalizer tests and 11 Python tests pass; fmt/Clippy pass.

All five Rust readers admit the final set. RX 7900 XTX WSI passes 120 frames/ten
generations with zero diagnostics; default/shared captures each verify GPU bytes,
identities/work/depth and 85 sampled colours, including repeated UVs. Three
malformed PNGs, five malformed normalized texture records and symlink escape
fail before publishing a usable bundle. Repeated normalization/cooking is
identical; original grid/quad/triangle hashes remain unchanged.

This admits one texture shared by all selected materials. Multiple authored
textures, per-material image tables, mip/gradient/filtering and asynchronous
dependency residency remain open. Next: bounded per-material texture selection.
[Importer](../tools/theia-gltf-import/README.md), [evidence](validation/M4_TEXTURES.md).

## M4 immutable textured bundle admission — independently executed (2026-10-11)

THEIABND v1 records four ordered content hashes/lengths: geometry, material
factors, UVs and one shared texture. Pure Rust admission validates every child
container, both geometry bindings and cooked opaque texture provenance. The
viewer `--bundle` admits the complete set before Vulkan initialization; missing,
truncated, corrupted, foreign-UV and malformed-manifest cases fail at that stage.
Three new tests bring the asset suite to 24; renderer tests remain 14. Formatting
and denied-warning Clippy pass. Packaging twice produces identical bytes.

RX 7900 XTX default/shared bundle WSI tests each pass 120 frames/ten generations
with zero diagnostics. Separate default/shared captures each verify all four
dependencies, GPU bytes/texels, identities/work, depth and 85 sampled colours.
The bundle selects the preceding resident texture path; shaders are unchanged.
This is fixed-set admission, not asynchronous dependency graph publication or
per-material texture selection. Authored image import remains the next task.
[Contract](M4_ASSET_CONTRACT.md), [evidence](validation/M4_TEXTURES.md).

## M4 resident texture sampling — independently executed (2026-10-11)

`--texture-test <ktx2>` connects one immutable opaque RGBA8 sRGB texture to fixed
geometry, UV and unlit factor pages. Vulkan image-format limits and physical
allocation budget are checked; dedicated upload fence and transfer-to-fragment
sampled-read barrier publish the image. One nearest-repeat sampler samples mip
zero; Slang reconstructs perspective UVs and multiplies sampled linear RGB by
the authored factor. Descriptor/frame retirement protects image/sampler lifetime.

RX 7900 XTX passes 15 standalone 120-frame/ten-generation validation paths,
including imported/terrain/shared texture paths and earlier rendering modes,
with zero diagnostics through teardown. Six separate texture captures verify
actual GPU texels/geometry/UV/factor bytes, identities/work, depth and colours;
terrain checks all 512 visible triangles plus 81 grid samples. Original UV and
M3 captures also pass. A 4095-byte physical image cap rejects the 4096-byte image;
missing texture shader cleanup passes. Fourteen renderer tests, formatting,
denied-warning Clippy and both 26-output shader builds/validation pass.

This applies one explicitly supplied texture to all material factors. It does
not import glTF images, establish per-material texture bindings, mip/derivative
selection, streaming dependency publication or virtual textures. Next: bounded
authored image import and dependency binding. No push or performance claim.
[Evidence](validation/M4_TEXTURES.md).

## M4 bounded texture asset contract — independently executed (2026-10-11)

Pinned offline KTX-Software 4.4.2 now produces synthetic opaque RGBA8 sRGB,
top-left, single-mip 2D textures. The Release source build excludes its separately
licensed ETC decoder. Exact source texels and deterministic output are checked;
official validation succeeds with its custom-provenance-key warning recorded.
Pure Rust bounded KTX2 decoding checks sections, DFD, metadata, alpha and hash;
21 asset tests and ten Python tests pass with formatting/denied-warning Clippy.
This CPU contract does not establish GPU upload/sampling or glTF image import.
Next: resident texture upload/sampling, then authored image/dependency integration.
[Contract](M4_ASSET_CONTRACT.md), [evidence](validation/M4_TEXTURES.md).

## M4 authored UV reconstruction — independently executed (2026-10-11)

Opt-in `--unlit-uv` imports float2 TEXCOORD_0 into normalized schema 3. Source UVs
stay unchanged through node transforms/reflections; coincident positions with
UV seams retain distinct dense rows. The cooker emits an exact-geometry-bound
THEIAUVS v1 page. Three reader tests cover finite values, byte/count/hash/binding
checks and invalid padding/UTF-8; 18 asset tests and eight normalizer tests pass.

Render Lab `--asset <mesh> --uv-test <page>` uploads one immutable UV buffer and
reconstructs selected-triangle UVs using perspective-correct weights in Slang
fragment resolve. Upload/frame-fence/descriptor ownership remains explicit;
fixed geometry only. An analytic center-ray reference verifies UV weights.
Default/shared builds compile/SPIR-V validate all 25 shaders; 13 renderer tests,
formatting and denied-warning Clippy pass.

RX 7900 XTX UV/terrain/shared WSI and 11 prior paths pass 120-frame/ten-generation
regression with zero diagnostics. Six separate captures match uploaded geometry/
UV bytes, work, identities, depth and UV-encoded color: imported full/empty/narrow,
imported shared, terrain full/shared. Full terrain checks all 512 visible triangles
plus 81 grid samples. Missing UV fragment and wrong geometry clean up safely.
Repeated cooking is identical; old grid/quad/triangle hashes remain unchanged.

This is diagnostic UV reconstruction, not sampled authored textures, mip/gradient
qualification, material dependency streaming or a completed M4. The next gate is
bounded texture data/import and sampling. Work resumed at user request; nothing
was pushed. [Contract](M4_ASSET_CONTRACT.md), [evidence](validation/M4_COOKED_MESHLETS.md).

## M4 bounded glTF unlit import — independently executed

Offline `--unlit-factors` imports explicit double-sided opaque KHR_materials_unlit
linear factors and source primitive assignments. Normalized schema 2 tracks dense
material indices across instances/reflections; the native cooker remaps by
winding-preserving triangle identity and emits exact-geometry-bound THEIAMAT v1.
Unknown/unsupported material features fail explicitly. Existing binary mesh/cell/
material formats and default position-only import remain intact.

Six normalizer tests and eight Python LOD/remapping tests pass; Rust fmt and
Clippy with denied warnings pass. Repeated normalization/cooking is byte-identical.
Independent Rust readers validate the final 2,656-byte/eight-vertex/four-triangle
mesh and 444-byte/two-factor material page. Raw grid/quad/triangle hashes remain
exactly unchanged. RX 7900 XTX final-pair WSI presents 120 frames/ten generations
with zero validation diagnostics. Two separate replay captures (default/shared
identity interfaces) verify geometry/material GPU bytes, work, all four triangle
identities and 85 depth/color samples each; targets present 180 frames.

This is factor-only unlit import, not textured/PBR material support, UV/normal/
vertex-color import, transparency or material dependency streaming. M4 remains
open. User requested stopping after this unit; no next task was started and
nothing was pushed. [Tool workflow](../tools/theia-gltf-import/README.md).

## M4 GPU authored opaque factors — independently executed

`--materials <hash.theia-material>` on a fixed `--asset` validates an exact geometry
binding, uploads one immutable material buffer, and resolves meshlet/triangle IDs
to authored opaque linear factors at fragment binding 5. No runtime CPU triangle
lookup/draw/readback. Material upload uses the existing dedicated fence and scoped
fragment storage-read publication; swapchain/descriptor retirement precedes
buffer/device destruction. Cell/fallback material dependencies remain unsupported.
Default and optional shared-interface builds each compile/validate 24 shaders.

RX 7900 XTX authored glTF/terrain WSI passes 120 frames/ten generations with zero
diagnostics; twelve new/previous GPU paths pass regression. Six separate replay
cases verify geometry/material GPU bytes, compacted work, exact identities,
depth and factor color: imported full/empty, terrain full/narrow, imported shared
and terrain shared. Full terrain checks all 512 visible triangles plus 81 grid
samples; narrow checks 239 triangles plus 81 samples. Both imported variants
check all four visible triangles/three assigned factors. Original M3 replay
remains qualified; wrong geometry and missing authored fragment clean up without
diagnostics. Thirteen renderer tests, fmt and denied-warning Clippy pass.

Assignments still target explicit cooked triangle IDs. The later bounded glTF unlit factor unit qualifies import/remapping;
UVs/textures/normals/lighting/alpha, material dependency
streaming and broader scene integration remain open. M4 remains incomplete.

## M4 bounded opaque material contract — CPU/offline qualified

THEIAMAT v1 sidecars bind exact immutable geometry hashes to finite opaque
linear RGBA factors and explicit cooked-triangle material assignments. Dense
meshlet prefix offsets are independently checked against the decoded geometry;
lengths/digests/padding/counts/references are validated before use. No existing
mesh or cell format changes. Four new asset tests (15 total), fmt and denied-
warning Clippy pass. Python cooking produces a 460-byte, three-factor/four-
triangle sidecar which Rust independently decodes; repeat cooking is identical.
Six malformed assignment cases and a different-geometry binding fail explicitly.

Assignments are hand-authored against cooked triangle IDs, not glTF primitive
indices. This CPU/offline unit adds no GPU binding; the later GPU unit above qualifies
lookup/resolve. General glTF material import remains open; textures/UVs/normals, lighting,
alpha/transparency and material dependency residency remain open. M4 stays open.

## M4 optional shared primitive interface — independently executed

A specific GLSL fragment fallback now qualifies Slang shared-vertex mesh output
with custom PerPrimitiveEXT identity. `--shared-primitive-interface` compiles
an alternate shader directory; default Slang expanded-vertex modules remain
unchanged. Both stage interfaces use Location 0/PerPrimitiveEXT and neither adds
Geometry capability. Publication rejects missing interface evidence. Pinned
Ubuntu glslang 15.1.0 package/binary digests are recorded; CI configuration checks
the optional path locally, with no hosted execution claim.

RX 7900 XTX standalone mesh/cell WSI runs pass 120 frames/ten generations with
zero diagnostics; cell replaces five pages. Six replay cases match 81 exact
identity/depth/color samples each and GPU geometry/work, including nonzero arena
offset. The current conservative lab admission still requires 192 output
vertices/8192 bytes despite the alternate shader emitting at most 64 vertices;
this qualifies interoperability, not a relaxed device profile or speedup.
General authored materials/attributes remain the next content gate; M4 stays open.

## M4 cooked visibility/material resolve — independently executed

Optional `--material-test` connects cooked meshlets, perspective camera, GPU
frustum work and fenced arena residency to R32G32_UINT meshlet/triangle identity,
D32 depth and deferred diagnostic material resolve. Resolve reconstructs world
position with perspective-correct weights from the selected immutable triangle
and samples the existing two-texture nonuniform array. No runtime host work/ID
readback. Slang compile/SPIR-V validation passes (23 render outputs).

RX 7900 XTX standalone WSI passes 120 frames/ten generations with zero validation
diagnostics. Six separate replay cases verify GPU geometry/work and 81 exact
identity, depth and color samples each: full/narrow/empty terrain, transformed
glTF, near-before-far occlusion, and cell replacement at arena offset 5528.
Original M3 replay remains qualified. Thirteen renderer tests, fmt and denied-
warning Clippy pass; prior rendering paths and construction cleanup are checked.

This uses a diagnostic meshlet-parity palette, not authored materials/UVs/mips.
Slang builtin primitive ID added Geometry capability; custom per-primitive
fragment annotation was ignored. This bounded test instead emits three vertices
per triangle and explicitly requires 192 mesh output vertices/8192 output bytes.
Shared-vertex per-primitive interoperability remains open; no performance claim
or renderer-wide expanded-vertex decision follows. M4 remains incomplete.

## M4 GPU meshlet frustum selection — independently executed

Optional `--frustum-test` computes conservative clip-plane candidates from GPU
resident meshlet vertices, compacts IDs and writes indirect counts. Task/mesh
stages consume those IDs with perspective/depth rendering; no host count/list
readback or per-meshlet draws. A 256-byte work ABI holds command/count, candidate
count/mask and up to 32 IDs. Queried compute limits cover 32 lanes/eight shared
bytes/64 push bytes; Slang compile/SPIR-V validation passes (20 render outputs).

RX 7900 XTX cell WSI passes five page replacements, 120 frames/ten generations,
zero validation diagnostics. Three full-terrain captures match CPU candidate
IDs/counts/masks for empty/narrow/full views: 0 / 6 / 8 candidates, masks 0 / 0x5f /
0xff, with 81 perspective color/depth samples each. An additional cell capture
checks narrow-view IDs and nonzero arena offset after page replacement. Thirteen
renderer tests, fmt and denied-warning Clippy pass; nine prior rendering paths
and missing cull mesh construction failure pass zero-diagnostic GPU regression.

This is conservative meshlet selection, not exact primitive visibility, HZB,
projected-error LOD, cell-demand feedback or M1/M2/M3 scene integration. The forward diagnostic path remains available; the later cooked material test
connects selection to identity/depth and resolve.

## M4 bounded static glTF geometry — independently executed

Offline Rust normalizer evaluates pinned `gltf` 1.4.1 (verified MIT/Apache-2.0)
for static position-only triangles. It bakes parent/local transforms, converts
RH Y-up meters to RH Z-up, and preserves intended winding for reflected nodes.
Embedded, external and GLB buffers are bounded/checked before accessor decoding.
Four tests cover analytic TRS/reflection, dense instance ranges, invalid inputs
and buffer-container parity; fmt and denied-warning Clippy pass. Unsupported
materials/attributes/animation/skin/extensions/cameras fail explicitly.

Synthetic authored glTF fixture has two shared-mesh instances, one reflected,
rotated and raised. Cooked output: eight vertices, four triangles, one meshlet,
2,072 encoded bytes. Provenance includes original buffers/document, exact parser,
normalizer source/lock/binary/compiler/target and native cooker/library hashes.
Repeated normalization with the same binary/source is byte-identical.
RX 7900 XTX camera WSI presents 120 frames/ten generations with zero validation
diagnostics; separate replay verifies GPU bytes/counts and 81 perspective
color/depth samples. CI configuration adds the offline package/workflow locally.

This is a bounded geometry converter, not complete glTF/material import, GPU
instancing or a complete static scene database. Names/indices are provenance,
not persistent world IDs. M4 stays open; GPU frustum selection and broader scene
contracts are next. [Tool contract](../tools/theia-gltf-import/README.md).

## M4 perspective/depth reference — independently executed

Optional `--camera-test` transforms cooked meshlets with a fixed Z-up perspective
camera, aspect-dependent rows and a D32 LESS depth target. Cell/page residency,
GPU indirect counts and existing XY viewer stay intact. Slang camera mesh stage
compiles/validates (18 render outputs total); three camera tests independently
check Vulkan near/far depth, aspect/Y direction and analytic center-ray occlusion.
All twelve renderer tests, formatting and denied-warning Clippy pass.

RX 7900 XTX terrain/cell camera WSI smoke passes five replacements, 120 frames,
ten swapchain generations and zero validation diagnostics. Four separate captures
verify uploaded page bytes/counts and 81 perspective color/depth samples each for
coarse terrain, full terrain, empty work and near-before-far occlusion. CPU oracle
uses perspective-correct world attributes and nearest depth; replay tolerance is
2e-6 depth / two RGB byte levels. The fixture deliberately puts farther triangles
after nearer ones, with ordering verified by the offline cooker.
All eight prior render paths and the new occlusion camera smoke pass regression;
missing camera mesh after arena/upload/depth creation cleans up with zero diagnostics.

This qualifies a fixed camera/raw-position meshlet reference, not authored glTF
scene/node transforms, general clipped-triangle CPU rasterization, materials,
camera controls or M1/M2/M3 scene integration. M4 remains open.

## M4 fixed geometry arena — independently executed

Cell streaming now allocates one fixed device-local geometry buffer and leases
aligned page ranges inside it. Immutable ranges remain owned through pending
upload/frame retirement; last-owner release coalesces reusable space. A pure Rust
range oracle passes retained-lease, fragmentation, overflow/exhaustion and 5,000
randomized occupancy checks. All eleven asset tests and nine Render Lab tests,
formatting and denied-warning Clippy pass locally.

RX 7900 XTX requires 4-byte storage descriptor offsets. The 16 MiB arena smoke
passes five replacements, 120 frames, ten generations, zero validation warnings/
errors. The grid uses a 7,244-byte range at offset 848; the quad uses 848 at offset
0. Four separate captures verify selected range bytes/counts and 81 pixel samples
each, including nonzero offsets and safely reused ranges. An 8,096-byte arena
permits replacement; 8,080 preserves coarse with five deferrals. An 8,092-byte
request requires 8,096 physical bytes and fails its strict allocation budget.
Pending-upload shutdown and an arena too small for coarse also clean up safely.

This is a bounded host-allocated range prototype. Staging still uses a separately
capped allocation per upload. General arena classes, GPU allocation/demand queues,
multi-frame overlap, authored scene/camera integration and virtual textures remain
open. M4 remains incomplete. The older dedicated-page evidence below is historical.

## M4 bounded offline heightfield LOD — independently executed

Official meshoptimizer 1.3 simplifies a synthetic curved 16x16 terrain patch
from 512 to 128 triangles, then cooks two meshlets (5,528 encoded bytes).
An independent checker verifies all 64 original boundary edges, positive XY
winding, manifold edge counts, equal projected area, coverage and height at
5,059 points. Maximum sampled vertical difference is 0.0122543, below the 0.03
qualification limit; the native absolute metric separately reports 0.00903259.
These are distinct metrics; sampling does not prove a Hausdorff/global error bound.

Four analytic/adversarial Python tests pass. A stricter 0.001 height limit rejects
the cook. Repeated terrain cooking is identical; the original grid fixture's
digest is preserved. Rust reader/file-backed lifetime checks pass on new files.
RX 7900 XTX cell-stream and capacity-fallback WSI runs each present 120 frames
with zero validation diagnostics; streaming replaces five pages. Separate
captures verify full/coarse GPU bytes/counts and 81 XY color/coverage pixels each.
The viewer has no depth attachment, so these captures do not validate terrain
depth or perspective appearance. Full/coarse physical allocation sizes are
7,248/5,536 bytes. CI recipe is updated locally, without a hosted result claim.

General authored meshes, conservative hierarchy errors, projected-error GPU LOD,
cross-cell seam transitions and production geometry arenas remain open.

## M4 bounded asynchronous cell residency — independently executed

Render Lab `--cell` reads two validated mesh pages on a bounded I/O worker and
submits one fenced staging upload at a time. It publishes a replacement only
after upload completion and the sole rendering frame fence, then retires the
old descriptor/buffer and CPU reader. Successful uploads use no queue/device idle
wait. Geometry budget counts Vulkan allocation requirements, including overlap
of active and pending pages; one staging allocation has a separate 64 MiB cap.

RX 7900 XTX passes default and exact-capacity WSI smokes with five replacements,
120 frames, ten swapchain generations and zero validation diagnostics including
teardown. The grid is 7,244 encoded / 7,248 allocated bytes; the quad is 848 for
both. A budget of 8,092 bytes defers five replacements and preserves the coarse
page; 8,096 bytes permits them. Shutdown with an unpublished upload and startup
below the coarse allocation requirement both clean up without diagnostics.
Four separate captures verify immutable GPU bytes/counts and 81 pixel samples
each across coarse/fine/coarse replacement and protected budget deferral.
All seven prior render modes pass 120-frame regressions with zero diagnostics;
Render Lab's nine unit tests, formatting and denied-warning Clippy pass.

This qualifies two-page, single-frame, same-queue fixture residency. Production
preallocated arenas, multi-frame reader overlap, general dependencies/LOD,
authored scenes, terrain and virtual textures remain open; M4 is incomplete.
See [updated evidence](validation/M4_COOKED_MESHLETS.md).

## M4 immutable page lifetime CPU contract — locally tested

Added bounded hash-addressed cell manifests and a pure Rust residency oracle.
Immutable snapshots retain evicted pages; staged/retained bytes remain charged
until their last owner releases them. Duplicate staging shares one allocation;
foreign tokens, corrupt pages and invalid/duplicate removals fail safely.
Eight asset tests pass, including 2,000 randomized lifetime operations against an
independent live-page set. Formatting and Clippy with denied warnings pass.
The file-backed cell-7 example protects the 7,244-byte grid after eviction, then
releases the reader and publishes the 848-byte quad at epoch 3.

This is a CPU contract, with an encoded-byte budget. Async I/O, Vulkan publication
completion, frame-retirement fences, physical resident budgets and renderer
load/unload integration are the next qualification unit. No new GPU capability or
M4 completion is claimed. Changes remain local.

## M4 initial cooked-asset/viewer slice — independently executed

Official meshoptimizer 1.3 (offline only) cooks/optimizes a 512-triangle grid into
eight meshlets and verifies coverage/winding. Pure Rust reader validates content
address, schema, payload digest, ranges/indices, finite positions and byte budget.
Render Lab uploads immutable geometry once and Slang decodes/draws it through
GPU-written indirect counts. RX 7900 XTX WSI and capacity-fallback smoke runs pass
with zero validation warnings/errors including teardown. Four captures verify
selected immutable GPU bytes/counts plus 81 coverage/color samples each.

A source-linked 848-byte coarse quad replaces the 7,244-byte grid under a 2,048-byte
encoded-geometry budget; complete planar replay images differ by at most one RGB
byte level. Invalid content/source links/budgets and missing shader after upload
fail cleanly. Seventeen render shader outputs validate; Rust checks and all prior
rendering/headless/ECS regressions pass. Work remains local without hosted CI claims.

**M4 remains open** for authored scene integration, general LOD, immutable cell
paging, async publication/retirement, actual per-arena residency, terrain and
virtual textures. This is a bounded prototype, not production streaming.
[Contract](M4_ASSET_CONTRACT.md), [evidence](validation/M4_COOKED_MESHLETS.md).

## M3 bounded visibility/material reference — independently executed

Render Lab `--visibility-test` writes integer instance/primitive identity and D32
depth, then reconstructs barycentric color and samples two materials through a
nonuniform runtime descriptor array. RX 7900 XTX passes the 120-frame WSI smoke
with ten generations and zero validation diagnostics including teardown. Three
separate captures each verify 84 identity/depth/sRGB pixels against the CPU oracle,
including near/far occlusion, far material and background. Fourteen render shader
outputs validate; nine unit tests and all five earlier render-mode regressions pass.

This qualifies the bounded local M3 reference gate. General perspective/mip
reconstruction, patterned materials, lighting, scene integration and other
platforms remain unqualified. Evidence and compiler/display limitations:
[M3 visibility/material qualification](validation/M3_VISIBILITY_MATERIALS.md).

## M2 bounded spatial/visibility reference slice — independently executed

Render Lab `--spatial-test` builds direct-addressed XY CSR memberships and gathers
candidates, applies height-aware bounds culling, compacts visible IDs and renders
GPU-generated indirect task/mesh work. RX 7900 XTX passes the 120-frame WSI smoke
with ten swapchain generations and zero validation warnings/errors. Separate
RenderDoc captures match independent CPU bounds/CSR/masks/IDs/counts for empty,
filtered and full views; replay images contain exactly 0, 8 and 32 triangles.

Nine render stages validate; seven CPU tests pass, including 1,000 randomized
view comparisons against brute force. All four earlier rendering modes and the
M0 capture workflow pass regression checks. This qualifies the bounded M2 reference
slice, not dynamic M1 scene integration, perspective/HZB culling, packed vertical
indexes, content streaming or scale. Evidence:
[M2 spatial qualification](validation/M2_SPATIAL_VISIBILITY.md).

## M1 bounded GPU reference slice — independently executed

Separate `apps/theia-ecs-lab` executes Slang initialization, ordered structural
mutation, parallel component integration and retained-reader compute. RX 7900 XTX
passes 526 batches / 8,717 commands against the CPU oracle with zero synchronization
validation warnings/errors including teardown. Both archetype migration directions,
compaction, 64-bit world identities, stale/conflicting handles, chunk/slot exhaustion,
retained old GPU data and permanent generation exhaustion are checked.

Four ECS shader outputs pass SPIR-V validation. Local Rust checks pass; missing
shader and unmatched-device construction failures clean up without diagnostics.
M1's bounded reference slice is qualified; production parallel mutation, real
concurrent rendering-reader overlap, general schemas, scale and other platforms
remain open. CPU readbacks are diagnostic only. Evidence:
[M1 GPU ECS qualification](validation/M1_GPU_ECS.md).

## M1 CPU oracle — previously qualified

`crates/theia-ecs` now provides the bounded dense-column/generational-handle CPU
reference. Six local unit tests pass, including 10,000 deterministic randomized
operations checked against an independent map. Formatting and Clippy with denied
warnings pass. This is reference scaffolding; the subsequent bounded GPU execution is recorded above. Scope and next GPU checks: [M1 qualification contract](M1_GPU_ECS.md).

## M0-D reference qualification closeout — independently executed

- RX 7900 XTX / RADV / X11: `--wsi-smoke-test` completes all four explicitly
  injected acquire/present recovery probes, 120 successful queued presentations
  and ten swapchain generations, with zero validation warnings/errors including
  teardown. Genuine driver suboptimal results were observed during resize.
  Genuine driver out-of-date, surface/device-loss and allocation-failure returns
  remain unobserved; fatal present classification has CPU tests only.
- RenderDoc 1.46 captures and replays compute + indirect task/mesh work. Diagnostic
  replay verifies command/count `(1,1,1,1)`, two downstream mesh groups and GPU
  transforms, and exports the correct two-triangle image. Runtime has no readback.
- Capture runs explicitly disable validation. Combined capture/validation produced
  layout diagnostics; standalone synchronization-validation runs pass. Muted
  RenderDoc output also suppresses the deliberate callback check. These tooling
  limitations are recorded, not treated as a validated combined workflow.
- Normal indirect smoke and preserved headless compute (256/256) pass. Formatting,
  Clippy with denied warnings and five unit tests pass locally. Changes remain
  local at user request; no new hosted CI result is claimed.

The **local RADV/X11 M0-D reference slice is qualified**; M1 can begin. Windows,
Wayland, other vendors, GPU-assisted validation, real fatal WSI returns and
performance remain separate qualification work. No locked ADR changed.
Evidence and commands: [M0-D closeout](validation/M0D_CLOSEOUT.md).

## M0-D GPU indirect work — independently executed

- Default Render Lab runs Slang compute to write a device-local 64-byte work buffer:
  indirect task group dimensions, draw count, transforms and downstream mesh count.
  It executes `vkCmdDrawMeshTasksIndirectCountEXT` with no host mapping/count
  readback. `drawIndirectCount` is queried/enabled explicitly.
- Resource-scoped Sync2 barriers order buffer reuse and publish compute writes to
  indirect command/count reads and task storage reads. The task copies transforms
  into its shared payload and launches the GPU-selected number of mesh groups.
- Radeon smoke cycles qualification inputs 0, 1, 2 while resizing/minimizing/restoring:
  120 queued presentations, six swapchain generations, zero warnings/errors including
  teardown. Three fixed-input 120-frame runs also pass; window pixels verify a clear
  frame, one left triangle and two triangles respectively.
- `--direct-task`, `--mesh-only` and `--clear-only` preserve prior paths. All three
  smoke regressions and unchanged headless compute (256/256) pass. Missing indirect
  task after work-resource creation exits cleanly; invalid/mismatched CLI inputs fail.
- Six render shader outputs pass Vulkan 1.3 SPIR-V validation; Rust formatting,
  Clippy with denied warnings and both extent tests pass. CI includes the new stages.

Evidence: [GPU indirect work qualification](validation/M0D_INDIRECT_WORK.md).
This qualifies a bounded work-generation test, not scene culling or GPU ECS.
The subsequent local reference closeout is recorded above.
Windows/Wayland GPU runtime, other vendors, GPU-assisted validation and performance
remain unqualified. No locked architectural decision changed.

## M0-D task amplification — previously qualified

- Slang task + payload-consuming mesh shaders compile to SPIR-V 1.6 and pass
  Vulkan 1.3 validation. Task output uses `TaskEXT`, `TaskPayloadWorkgroupEXT`,
  and `OpEmitMeshTasksEXT(2,1,1)`; both stages use 32 lanes.
- Default Render Lab submits one task group, which launches two mesh groups and
  supplies two offsets plus a scale. Each group generates an RGB triangle with
  no vertex/index buffers. `--mesh-only` preserves the qualified single-triangle
  path; `--clear-only` preserves the bootstrap.
- RX 7900 XTX / RADV / X11: task smoke passes 120 frames and six swapchain
  generations, with zero validation warnings/errors including teardown. A
  300-frame run also passes; own-window checks verify both triangle centers,
  six color regions and the clear gap between them.
- Slang 2026.19 crashed for the documented `out payload` task-parameter form.
  Explicit `groupshared` payload storage compiles, validates and executes; no
  HLSL/GLSL fallback or locked architecture change occurred.
- Formatting, Clippy with denied warnings and both extent tests pass. Mesh-only,
  clear-only and unchanged headless compute regressions pass; missing task module
  errors clean up the already-created mesh/fragment modules without diagnostics.
  Shader CI now includes task and both mesh variants.

Evidence and limitations: [task amplification qualification](validation/M0D_TASK_AMPLIFICATION.md).
The subsequent compute-written indirect qualification is recorded above. **M0-D
remains open for qualification closeout.** Windows/Wayland GPU execution, other vendors and GPU-assisted
validation remain unqualified; no performance claim is made.

## M0-D mesh-only triangle — previously qualified

- Slang 2026.19 compiles mesh + fragment stages to SPIR-V 1.6; both pass
  `spirv-val --target-env vulkan1.3`. Mesh uses `MeshShadingEXT` / `MeshEXT`,
  local size 32x1x1, three output vertices and one primitive. No language fallback.
- Render Lab now creates a mesh-only graphics pipeline and executes
  `vkCmdDrawMeshTasksEXT(1,1,1)` on RX 7900 XTX / RADV / X11. There is no vertex
  shader, vertex buffer or index buffer. **At this qualification, task shaders had not executed yet.**
- Mesh smoke: 120 queued presentations, six swapchain generations, zero validation
  warnings/errors including teardown. A 300-frame run also passes; own-window image
  checks verify the clear background, interpolated center and red/green/blue regions.
- Pipeline/layout/module RAII covers creation errors and recreation for the actual
  swapchain format. `--clear-only` preserves the bootstrap path; `--shader-dir`
  supports explicit shader locations. Missing shaders fail with clean teardown.
- Rust formatting, Clippy with denied warnings and the two extent tests pass.
  The original compute app/source remain unchanged. CI now also compiles/validates
  shaders with the official Slang release, pinned by SHA-256.

Evidence and limitations: [mesh triangle qualification](validation/M0D_MESH_TRIANGLE.md).
The subsequent task amplification qualification is recorded above;
compute-written indirect mesh/task work remains outstanding. **M0-D remains open.** Windows/Wayland GPU
execution, other vendors and GPU-assisted validation remain unqualified.

## M0-D bootstrap — previously qualified

- Added separate `apps/theia-render-lab`: ash 0.38, ash-window 0.13, winit 0.30.13,
  pinned lockfile. Native X11 surface and FIFO swapchain execute on RX 7900 XTX,
  RADV Mesa 26.2.4; dynamic rendering displays a dark blue clear.
- Queries/enables Vulkan 1.3 mesh/task capability, BDA, descriptor-indexing subset,
  timeline semaphores, dynamic rendering and Sync2. **At bootstrap qualification, mesh/task pipelines had not
  executed.** The lab additionally requires EXT swapchain/surface maintenance for
  presentation-fence retirement; this does not redefine the approved engine profile.
- Explicitly enables `VK_LAYER_KHRONOS_validation` (installed version 1.3.275),
  requests synchronization validation via instance pNext, and verifies callback
  delivery with an intentional diagnostic excluded from totals.
- Corrected hazards found in the first synchronization-validation run. Final smoke:
  **120 queued presentations, six swapchain generations, zero warnings/errors
  including teardown**. Requests resize/minimize/restore and explicitly verifies
  zero-size deferral. A separate 300-frame run and own-window capture verify the clear.
- Unsupported `--gpu` selection exits with an actionable error and zero teardown
  diagnostics. Zero frame count is rejected. Extent tests: **2/2 passed**.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
  pass for both apps. Existing compute source is unchanged; Slang compute compile,
  `spirv-val --target-env vulkan1.3`, and **256/256 GPU values** pass again.
- Added Linux/Windows Rust CI (build/lint/unit tests, without GPU execution).
  Wayland, Windows runtime, other vendors, split queue families, device/surface loss,
  and GPU-assisted validation remain **untested**. The smoke log does not prove an
  actual `ERROR_OUT_OF_DATE_KHR` return was exercised.

Commands, observed logs and limitations: [M0-D bootstrap qualification](validation/M0D_BOOTSTRAP.md).
The subsequent mesh-only triangle qualification is recorded above. Task amplification
and GPU-written indirect work counts remain outstanding. **M0-D remains open.**

## Original handoff — completed and verified

1. Linux Mint, AMD RX 7900 XTX (RADV NAVI31) detected via `vulkaninfo`; RADV Mesa 26.2.4, device Vulkan API 1.4.354.
2. Vulkan driver filtering with `VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json` selects RADV and removes unrelated Dozen/llvmpipe in reported enumeration (integrated Radeon still appears).
3. `VK_EXT_mesh_shader` advertised; `taskShader=true`, `meshShader=true`; maxTaskPayloadSize 16384; maxMeshOutputVertices=256; maxMeshOutputPrimitives=256. **No mesh/task GPU pipeline has executed yet.**
4. Slang `slangc -version` prints **2026.19** and PATH configured from `~/dev/tools/slang/bin`.
5. `shaders/compute/buffer_test.slang`: set 0 binding 0 RWStructuredBuffer<uint>, 64 threads/workgroup, writes `index * 2 + 1` for 0…255.
6. Slang compiled shader to `build/shaders/buffer_test.spv`; `spirv-val --target-env vulkan1.3` succeeded with no errors; `spirv-dis` shows `GLCompute`, `LocalSize 64 1 1`, set 0/binding 0, SPIR-V 1.6.
7. Rust 1.99.0 / Cargo 1.99.0; `ash` 0.38.0+1.3.281 added to `apps/theia-gpu-test`.
8. Headless Rust test loaded Vulkan using ash, selected RX 7900 XTX, created buffer, compute pipeline, dispatched four workgroups and read back 256 integers; first values 1,3,5,7,9,11,13,15; last values 497…511. **256/256 passed**.
9. Vulkan debug-utils messenger added, created, destroyed; application prints `Vulkan debug messenger initialized.`; executed under `VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation` with no diagnostics displayed. **Layer activation/message delivery not independently proven**, and synchronization validation/GPU-assisted validation not yet independently run.
10. `cargo fmt` and `cargo clippy` completed with no reported warnings after unused import cleanup; debug run completed normally.
11. Local project directory renamed to `~/dev/theia-engine`; Git repository initialized, nested Cargo-created `.git` issue fixed and user reported initial commit completed. **GitHub remote creation/push not confirmed in this conversation.**

## Files expected in the user's local checkout

```
~/dev/theia-engine/
  .gitignore
  apps/theia-gpu-test/Cargo.toml
  apps/theia-gpu-test/Cargo.lock
  apps/theia-gpu-test/src/main.rs
  shaders/compute/buffer_test.slang
  build/shaders/buffer_test.spv   # generated/ignored
```

This handoff ZIP does **not** include copies of the user's latest local application sources; Codex should inspect the Git checkout. An earlier uploaded `main.rs` in the conversation had the debug callback; local version may have changed after that.

## Original handoff plan — M0-D

Create **separate** `apps/theia-render-lab` (cargo new `--vcs none`), native window and Vulkan surface/swapchain, compile and run Slang task+mesh+fragment shaders, render one GPU-generated triangle. Then compute-generated indirect mesh task dispatch, Vulkan validation and correct resize/recreation. Keep headless compute test intact as regression. No ECS, GI, game server or editor yet.

## Original handoff caveats (see the update above for new evidence)

- Early one-shot compute test uses HOST_VISIBLE|HOST_COHERENT memory, `queue_wait_idle`, and large explicit unsafe block. Correct for qualification, **not** production data path. Production should use device-local buffers, staging, fences/timelines, RAII and scoped unsafe.
- `VK_EXT_debug_utils` messenger creation is observed but no proof of callback delivery; check enabled validation instance layer and submit diagnostic message or enumerate instance layers explicitly.
- Cargo had transient crates.io DNS resolution failure, then subsequent compilation succeeded.
- Shader path is relative to working directory `build/shaders/buffer_test.spv`: run executable from repo root until asset path resolution improves.
- User has not reported running render-lab or pushing a GitHub repo; do not mark complete without evidence.
