# Theia — Open Questions, Risks and Qualification Strategy

This is an experimental GPU-first engine. **Consensus on an architecture is not evidence of achievable performance.**

| Risk | Why it matters | Validation / fallback |
|---|---|---|
| Slang task/mesh support | Mesh shader codegen and driver behavior may vary | Minimal task+mesh+fragment executed on RADV (see CURRENT_STATUS); broader features/vendors remain gates. Slang 2026.19 task output-parameter crash worked around with groupshared; alternate HLSL/GLSL modules only for specific failed features |
| GPU archetype structural mutation | allocator races, fragmentation, stale handles, compaction barriers | CPU reference oracle, randomized mutation fuzzing and safety checks; constrain dynamic composition / specialized stores if needed |
| GPU ECS epochs | in-flight references after relocation/eviction | per-resource lifetime tracking, multi-frame hazard tests; version selected mutable streams only |
| GPU workload DAG | performance lost to many barriers/dispatches | measure total critical path and compare direct/task/indirect variants |
| Virtualized meshlet cluster LOD | cracks, temporal popping, page dependencies | offline geometry correctness and edge tests; discrete meshlet LOD fallback |
| Visibility buffer | material derivative and random attribute fetch overhead | benchmark compact G-buffer and vis-buffer in representative scenes |
| SDF GI shared radiance cache | leakage/thin geometry/reflections, GPU cost | sample caves, interiors, day-night, spells, crowds; cheap GI fallback; no mandatory RT |
| GPU UI Bézier curves | antialiasing of small text, cubic math, RTL/shaping/IME | host shaping first, visual GPU curves; accessibility semantic bridge |
| Large-world spatial index | dense headers, high occupancy, dynamic maintenance | flat XY + sparse vertical plus CSR/deltas; benchmark rebuild vs incremental |
| Server authority distribution | cross-partition conflicts under dense combat | avoid premature live repartitioning; single-writer state ownership, fencing/epochs; targeted workload tests later |
| Group replication | baseline fragmentation and N×M fanout | per-client ACK + shared canonical values, bounded caches, hybrid push/pull |
| QUIC CPU/congestion | connection/crypto state might be expensive | optimized Quinn first, profile packets/sec/core and p99; quinn-proto customized runtime before RUDP |
| Windows first-class | Vulkan/window/cross-toolchain divergence | CI Windows build early; per-vendor execution later |
| 24 GiB dev GPU bias | engine might only run on high end | bounded resource budgets and lower VRAM tier later |
| GPU text/UI semantics | I/O/IME/accessibility cannot be GPU-only | host semantic/input bridge; avoid GPU readback critical path |

Conventional texture-tail refinement now qualifies one background full-file read
and fence-gated replacement with measured active/pending image coexistence caps.
Coarse rendering survives short capacity and changed source identity. Retirement
still conservatively idles the graphics/presentation generation; I/O worker join
has no latency/cancellation guarantee. Tiled feedback, repeated request scheduling,
eviction and multi-frame reuse are separate gates.
[Observed scope](validation/M4_ASYNC_MIP_REFINEMENT.md).

The first procedural scene benchmark now validates dense proxy rendering,
frustum/detail compaction, continuous ground and live GPU counts across a scripted
route. Timings retain FIFO/single-frame/instrumentation and display-backend context;
tracked allocations are not total VRAM. A resident low-poly scene does not validate
cooked-content streaming, skinning/lighting costs or server/network player scale.
[Scope and evidence](validation/M4_SCENE_BENCHMARK.md).

A separate cooked benchmark mode now shares the route/timing recorder and
qualifies periodic complete-bundle replacement under exact/short CPU/GPU/image
caps, with independent cooked buffer/texel/culling and framebuffer probes. It
renders one bundle at a time without moving proxies. An optional debug HUD now
qualifies direct GPU meshlet counts plus completed timings and CPU-owned active
allocation/publication metadata; disabling it preserves the original resolve.
Its memory labels exclude pending/staging/driver allocations. This does not yet
qualify a simultaneously resident streamed world or camera-driven demand. GPU
frame queries exclude uploads submitted before them; CPU/frame intervals retain
publication stalls. Nearest texture-boundary probes use an explicit narrow
coordinate envelope. [Evidence](validation/M4_SCENE_BENCHMARK.md#cooked-bundle-route-and-timing-integration-2026-10-15).

## Decisions needing concrete values later

- Actual minimum GPU/VRAM models; baseline fps and target resolution; vendor matrix.
- Actual chunk classes, pool sizes, meshlet vertex/primitive counts.
- Render work descriptor representation, frame-local visibility ID bit budget.
- Current and previous pose memory vs recomputation; animation LOD thresholds.
- Network tick cadence, interest radius, per-client bandwidth cap and loss/latency percentile goals.
- PostgreSQL schema, journal storage choice and exact durability SLOs.
- GI probe/brick spacing, update rates, representation of moving occluders.
- HDR/UI luminance conventions, text shaping implementation, audio backend.

## Correctness principles

Use Vulkan validation and GPU-assisted validation in diagnostic builds (not perf baselines), diff GPU kernels vs CPU reference, test worst-case allocation pressure and stale handles, and maintain benchmark artifacts labelled with GPU driver/compiler versions. No phantom successes; only mark tests passed from observed logs.

## Early authority/network reference evidence (2026-10-11)

Bounded Rust movement/combat, independent request sequences, encrypted two-client
Quinn IPv4 loopback, controlled post-handshake packet faults and readonly snapshot/
CPU interpolation oracles now have local qualification logs. No WAN/load, accounts,
persistence, general baseline repair or client prediction is qualified. Separate
loopback GPU viewers now consume verified readonly authority snapshots; bounded
arrival-relative smoothing is not network clock synchronization. Actual authority
full/delta baseline ACK/repair now qualifies on the live GPU path; this remains a
fixed two-actor profile with no general lifecycle/persistence or WAN/load evidence. Prototype units/tick/gap profiles are test settings, not locked network
precision or cadence choices. IPv6 is disabled on the local host; Windows/hosted
CI remain unexecuted. See CURRENT_STATUS and M7/M8 validation reports.

## Nested Wayland platform evidence (2026-10-11)

Actual Wayland Vulkan surfaces execute within an owned Weston-on-X11 compositor;
six standalone modes and a faulted encrypted full/delta/repair pair pass Vulkan
validation. Winit's unsupported programmatic unminimize is explicitly excluded
from automated smoke. Available RenderDoc 1.46 rejects the Wayland surface extension;
that capture attempt fails and is not counted as validation/replay success. Native
seat/user minimize-restore, Windows and other vendors remain gates. No timing/
performance claim. [Evidence](validation/WAYLAND_NESTED.md).

The owned compositor's screenshot API now provides a separate visible-output oracle
for indirect 0/1/2 work counts: actual pixel samples/300-frame validation pass.
RenderDoc Wayland replay remains unqualified. Debug screenshot access is confined
to the temporary owned runtime; screenshots are ignored local artifacts. No timing/
performance or native-seat claim. [Evidence](validation/WAYLAND_PIXELS.md).

Windows GNU cross-link/Clippy and 129 Wine CPU/library tests now pass, including
recovery from actual WSAEMSGSIZE without killing the bounded relay. Headless Windows
executables exercise authority/script fixtures through Wine. Encrypted Quinn setup
is blocked in this Wine environment by a rejected IPV6_V6ONLY query on IPv4; native
Windows and GPU remain unqualified. No weakened transport/crypto fallback or inferred
native success. [Evidence](validation/WINDOWS_BUILD.md).

## Automatic mip prototype limits (2026-10-11)

Bounded GPU analytic UV gradients and explicit mip selection now have CPU/replay
proof on RADV, including visible unsupported-profile mip-zero fallback. The sparse
fragment-storage probe is optional lab instrumentation with queried capabilities,
not an engine baseline requirement. Arbitrary clipping/degeneracy and cross-vendor
numeric behavior, filtered quality, tiled/partial residency, full-chain replacement
and performance remain unqualified. [Evidence](validation/M4_GPU_AUTOMATIC_MIPS.md).

Within-mip bilinear sampling now has varied-texel RADV replay proof against the ideal
CPU model. The test retains two-byte encoded-colour tolerance; it does not prove
cross-vendor weight quantization or production filtering quality. Explicit integer
mip selection, authored levels and unsupported-gradient mip-zero fallback remain
prototype constraints. [Evidence](validation/M4_GPU_LINEAR_FILTER.md).

Conventional mip tails now have GPU byte/pixel/gradient/index proof and exact physical
allocation boundaries. Dense texel reductions do not guarantee matching VRAM savings:
a vertical 508-byte suffix costs 12288 physical bytes on the qualified driver.
Dark bilinear edges exposed finite filter weights; diagnostics now use a one-step
per-weight envelope at queried device precision plus prior output-byte tolerance.
This does not prove all coordinate arithmetic or cross-vendor conformance.
[Evidence](validation/M4_GPU_MIP_TAIL.md).

Headless Weston now qualifies native Wayland WSI and separate Xwayland ordinary
frames/replay while the desktop display remains off. Full headless Xwayland recovery
still times out after frame 80 in both composition-GPU configurations; cause and
clean teardown remain unqualified. Do not infer native-seat/scanout or Wayland replay
from this evidence. [Details](validation/HEADLESS_DISPLAY.md).


The mixed resident benchmark now qualifies shared cooked-world/proxy visibility,
depth and resolve up to 10000 visual proxies on RADV. It does not establish network
player capacity, collision/navigation, animation or production rendering cost.
The fixed coarse ground, diagnostic textures, two procedural proxy detail levels,
FIFO presentation and one frame in flight constrain timing interpretation.
Independent chunk residency and representative geometry/texture LOD remain gates.
[Evidence](validation/M4_SCENE_BENCHMARK.md#mixed-resident-village-and-crowd-2026-10-15).


Independent CPU chunk publication and outdoor distance demand now have bounded
oracle evidence. A separate four-chunk Vulkan upload probe qualifies physical
ownership, retained readers and short-cap cleanup; it draws no chunk geometry.
None of these results proves multi-chunk rendering, protected fallback coverage,
asynchronous world demand scheduling or descriptor/frame retirement. Those gates
must precede replacing the mixed benchmark's single resident representation.
[Upload evidence](validation/M4_WORLD_UPLOAD_OWNERSHIP.md).


Two complete resident chunks now share identity/depth and textured resolve, with
separate static bindings and combined descriptor-limit checks. Palette-variant
replay probes distinguish their image sets. These tiny opaque nearest/mip-zero
fixtures do not establish production material complexity, texture filtering/LOD,
streaming latency or representative art quality. Construction-time caps exclude
work/staging/targets/driver resources and are not total VRAM measurements.
[Evidence](validation/M4_WORLD_TEXTURED_PAIR.md).


The two-chunk profile now qualifies asynchronous complete replacement of one slot
while another remains fixed, including image-array count changes and protected
current sets under short capacity. It still rebuilds/retire graphics conservatively
and uses a fixed frame schedule. Spatial coverage, camera-driven request priority,
four-chunk composition, GPU allocator reuse and native Windows GPU remain gates.
[Evidence](validation/M4_WORLD_CHUNK_STREAMING.md).


Nonuniform storage descriptor arrays now have a small opt-in compute correctness
gate on RADV. It does not establish fragment/mesh compiler behavior, many-buffer
performance, four-chunk rendering or broader vendor/Windows GPU support. Normal
modes request their prior feature set. [Evidence](validation/M4_STORAGE_DESCRIPTOR_ARRAYS.md).


Four resident complete chunks now qualify optional nonuniform fragment storage
arrays and a combined sampled-image array. This tiny fixed world is not descriptor
capacity/performance evidence. Direct divergent indexing of a pushed texture-base
vector failed the varied-palette replay despite valid SPIR-V/clean native validation;
explicit scalar component selection passes. Keep pixel oracles alongside validation,
and requalify compiler/driver combinations before changing that expression.
Camera-driven publication, protected fallback and cross-vendor execution remain
open. [Evidence](validation/M4_FOUR_RESIDENT_CHUNKS.md).

Protected startup coverage keeps unselected coarse GPU owners. A separate finite
retirement schedule now qualifies coarse reactivation, including its actual raw
contents after becoming active. RenderDoc 1.46 exposes inactive allocation
metadata without saving unused storage-buffer initial contents, so current
oracles distinguish retained inventory from selected payloads. One bounded fine reload now qualifies; repeated/camera-driven publication and
larger lifetime/latency workloads remain open.
[Startup scope](validation/M4_PROTECTED_GPU_STARTUP.md),
[reactivation evidence](validation/M4_RETAINED_COARSE_REACTIVATION.md).

The fine-reload worker is byte-bounded and joined on shutdown, with no hard I/O
latency deadline. Pending CPU reservations and GPU allocations are reported
separately from published graphs; failed partial candidates are cleaned up but
not included in a peak-memory claim. Current coverage tests exercise a finite
request, not a repeated world scheduler. [Scope](validation/M4_ASYNC_FINE_RELOAD.md).

Camera-driven four-region demand now qualifies repeated fine/coarse transitions,
obsolete ready-upload cancellation and bounded capacity deferral while preserving
protected coarse coverage. This profile still prevalidates all eight sources at
startup, couples fine rendering to committed residency, and retires whole graphics
generations on publication. Worker cancellation has no hard latency deadline;
fatal source/descriptor failures clean up without rollback. Its bounded history
and complete-current/pending allocation counters do not establish streaming
throughput or total VRAM. Independent per-view LOD and proxy ground-height
consistency need separate integration gates before timed crowd claims.
[Evidence](validation/M4_CAMERA_WORLD_DEMAND.md).

The combined camera-world/crowd correctness gate now checks all 10000 GPU poses
against selected source terrain and verifies two-way occlusion. Its persistent
40704-byte terrain table retains both levels even for nonresident fine graphs and
is charged separately before graph admissions. This fixture-wide metadata cost
needs a different policy at larger scale. Mock boxes sample terrain beneath their
centers; no slope-conforming animation, foot IK or transition morphing is claimed.
The current path has no timed recorder/HUD, and its correctness results do not
establish production streaming throughput or networked-player capacity.
[Evidence and scope](validation/M4_STREAMED_WORLD_CROWD.md).

Streamed-world timing reports now qualify frame-stage timestamps and per-frame
residency snapshots. GPU intervals exclude transfers and complete-generation
retirement before frame recording; CPU draw wall includes synchronous renderer
costs but not background worker CPU usage. FIFO/single-frame/instrumentation and
small prevalidated content constrain interpretation. Report correctness does not
prove production streaming throughput or networked-player capacity.
[Metric scope](validation/M4_STREAMED_WORLD_TIMINGS.md).

The streamed HUD qualifies an optional 21-storage/128-byte-push profile, retaining
the prior world profile without HUD. It reads current GPU work counts but shows
completed latency metrics; asset and draw memory exclude different allocations.
The first replay exposed a truncated HUD payload despite zero validation errors,
so glyph checks remain necessary. Dense overlapping proxies also exposed an
adjacent-D32 ordering case: replay reports only validated proxy hits at the same
or next depth code with exact stored/source agreement. It rejects wider gaps,
foreign identities and wrong primitives; ordinary depth/colour limits remain.
Frame-zero replay separates startup staging metadata and selects current targets
from actual shader bindings. [Evidence and bounds](validation/M4_STREAMED_WORLD_HUD.md).

The public streamed launcher qualifies automated fixture preparation, native
routes and export fidelity. The cached native-tool layout is Linux-qualified;
prepared manifests support other layouts without implying native Windows GPU
execution. The launcher adds no new streaming policy or performance-capacity
claim; timing/memory scopes and independent per-view LOD limitations remain.
[Launcher evidence](validation/M4_STREAMED_BENCHMARK_LAUNCHER.md).

CPU per-view selection now separates desired level, available representation
and missing refinement, retaining the source epoch. Coarse fallback preserves
coverage but does not satisfy an unmet fine error request. All source-epoch owners
remain pinned, even when a view chooses coarse, so this is not a minimal per-view
memory-retention policy. No GPU/view-policy adoption is claimed by CPU evidence.
[Evidence](validation/M4_CPU_VIEW_SELECTION.md).

A native Bevy port now qualifies the benchmark's visible workload, with sampled
pixels and fine/coarse ground agreement. It preloads eight graphs and replays
Theia selection, with CPU ECS proxy motion. FIFO host intervals and differently
scoped CPU measurements cannot establish relative maximum throughput, GPU cost,
streaming or VRAM efficiency. Initial presentation diagnostics required a
reference-only first-surface clear; corrected native Wayland runs pass.
[Evidence and limits](validation/M4_BEVY_VISUAL_REFERENCE.md).

Stats-display on/off profiles now compare the existing fixed GPU font with a
corresponding Bevy UI panel. Theia replaces scene shading under the HUD, whereas
Bevy draws UI after the scene; glyph workloads and available metrics differ.
Matched-selection summaries preserve these limits, differing CPU scopes and
missing Bevy GPU timestamps. This is not evidence about Theia's planned M6 UI.
[Evidence and observed profile changes](validation/M4_STATS_DISPLAY_COMPARISON.md).

Optional native reference runs now qualify Immediate/Mailbox requests and Bevy
GPU render-graph timestamps alongside the preserved FIFO path. Bevy's private
surface configuration is verified through matching native capabilities and its
pinned selection path, not direct readback. Serial timestamp collection affects
host cadence; differing GPU/CPU scopes, synthetic assets and preload versus
streaming still prevent a whole-engine ranking. Other compositors, offscreen
saturation and equivalent streaming remain unqualified.
[Evidence](validation/M4_BEVY_PRESENTATION_TIMINGS.md).

The optional Bevy reference now qualifies one actual cooked fine-file reload over
protected coarse resources. Prepared render-asset presence plus queue completion
gates publication; last-handle Unused, render-entry removal and queue retirement
gate encoded release. Adapter-specific logical payload limits are not allocator
or physical VRAM limits. This fixed operation leaves repeated camera scheduling,
post-startup source failure/mid-operation shutdown qualification and streaming
performance equivalence open. [Evidence](validation/M4_BEVY_FINE_STREAM_PROBE.md).

The optional Bevy reference now qualifies repeated demand using shared distance,
retention and slot rules, including ready-candidate cancellation and final fine
drain. Serial backend lifecycle ordering and uncapped wall-clock demand rates
still differ; matching logical frames alone is not a fair I/O deadline policy.
Camera image checks explicitly exclude equal-canonical-depth proxy ambiguities
and do not prove their GPU depth winner. An earlier kernel-reported private Weston
SDMA fault caused device loss and cascading teardown diagnostics; fresh recovery
runs pass, but root cause/device-loss cleanup/platform stability remain open.
[Evidence and limits](validation/M4_BEVY_CAMERA_STREAM.md).
