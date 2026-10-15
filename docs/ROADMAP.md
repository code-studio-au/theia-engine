# Theia — Milestone Roadmap and Acceptance Gates

The goal is an **early end-to-end playable vertical slice**, not years of isolated rendering research. Work can advance on GPU client and authoritative server tracks in parallel once contracts stabilize.

| Milestone | Deliverable | Exit criterion |
|---|---|---|
| M0-A | Device/RADV and shader compiler qualification | Report GPU capabilities, Slang compile and SPIR-V validation |
| M0-C | Headless GPU compute | Rust ash dispatch produces 256 expected values — **DONE** |
| M0-D | Render Lab | Native window + Vulkan swapchain + task/mesh-generated triangle; validation and resize work |
| M1 | GPU ECS minimal | [Bounded reference slice qualified on RADV](validation/M1_GPU_ECS.md); dense components, handles, mutations and retained epochs vs CPU oracle |
| M2 | Flat 2.5D visibility | [Bounded reference slice qualified on RADV](validation/M2_SPATIAL_VISIBILITY.md); CSR candidates, height-aware bounds culling, GPU indirect mesh/task draws |
| M3 | Visibility buffer/materials | [Bounded reference slice qualified on RADV](validation/M3_VISIBILITY_MATERIALS.md); identity/depth, barycentric resolve, nonuniform sampled material array |
| M4 | Static world/streaming | [Initial cooked/viewer slice qualified](validation/M4_COOKED_MESHLETS.md); general LOD, terrain, virtual textures, immutable cells/residency remain open |
| M5 | GI/temporal | Static streamed SDF + dynamically updated shared radiance cache, motion vectors, reconstruction |
| M6 | Characters/UI | GPU animation/skin, shared pose baseline, Bézier text, UI, host shaping/IME |
| M7 (parallel) | Local authoritative game | Rust server tick, navigation/collision, movement, interactions, simple combat |
| M8 (parallel) | Network vertical slice | Quinn DATAGRAM+streams, state deltas, server validated play, persistence, two clients |
| M9 | Scale and operations | Cohort replication, NetLOD, worker authority transfer, profiling, reliability/load exercises |

## Immediate M0-D implementation plan

1. Audit existing repo and current Rust version, dependencies and Slang source.
2. Create minimal executable `apps/theia-render-lab` without nested Git (`cargo new --vcs none`).
3. Native `winit` event loop, Vulkan 1.3 instance + debug utils + surface; select compatible GPU/queue families and query mesh/task features/limits.
4. Create swapchain, image views, dynamic rendering render targets, basic sync and resize handling; start clear-screen-only.
5. Implement minimal Slang mesh-only + fragment triangle; compile, validate and **execute**.
6. Add task shader amplification, validate output and framebuffer.
7. Add compute-written GPU work descriptors + indirect mesh-task dispatch, correct barriers, no host readback for count — [bounded reference test qualified on RADV](validation/M0D_INDIRECT_WORK.md).
8. Qualification closeout: [local RADV/X11 reference slice qualified](validation/M0D_CLOSEOUT.md), with labeled WSI recovery injection, genuine suboptimal returns, and RenderDoc capture/replay. Real driver out-of-date/fatal returns and other platforms/vendors remain unqualified.

## M1 reference slice — locally qualified

Implemented a bounded CPU oracle: dense chunked position/velocity columns, stable
generational handles, two fixed archetypes, spawn/despawn/component migration at
explicit epochs, and delayed slot reuse while readers retain an older epoch.
A Slang implementation is qualified against that oracle with deterministic and randomized
mutation batches, stale/conflicting handles, exhausted capacity and retained-reader
tests. Vulkan resource ownership and batched transfer/dispatch are explicit. Do
not infer that the full GPU ECS, scheduler or allocation strategy exists from M0-D.

## Exit gates / nonfunctional requirements

- No reported validation errors for implemented M0-D paths; image acquisition/present and teardown safe.
- Repeated resize/minimize/restore safe; out-of-date swapchain handled.
- Slang compiler errors and shader pipeline errors diagnosed, no silent fallback.
- Known-good M0-C compute test unchanged and remains runnable.
- `cargo fmt`, `cargo clippy`, shader validation and at least basic CI pass.
- Any hardware-specific workaround explicitly documented; no claim of universal performance.

## Don't build in M0-D

General ECS, complex render graph, radiance cache, geometry LOD streaming, network replication, audio, full UI, advanced multi-queue GPU scheduling or 10k-player load harness. Each is an intentionally later step.

## M2 reference slice — locally qualified

Implemented a bounded flat 2.5D grid and contiguous membership/candidate ranges, verified
GPU candidate selection/culling against an independent CPU reference, and fed
GPU-generated visibility counts/indices into task/mesh rendering. Use direct XY
cell addressing; keep world identity independent of cell/storage placement.
Static content streaming, HZB, virtualized geometry and production-scale tuning
remain later slices. Preserve M0/M1 qualification apps and evidence.

## M3 reference slice — locally qualified

Implemented bounded visibility/depth targets and frame-local primitive identity,
barycentric attribute reconstruction and nonuniform runtime-array material sampling.
Fixed w=1/one-primitive fixture passes CPU pixel comparisons. General perspective,
UV/mip gradients, material/geometry tables and scene integration remain open.

## M4 initial slice — locally qualified; milestone remains open

A prototype immutable container, official offline meshoptimizer tool, pure Rust
reader and GPU meshlet viewer are implemented/tested. Source-linked coarse planar
fallback is qualified for an encoded geometry byte budget. This is not a complete
LOD hierarchy, cell/page manager, terrain or virtual texture system.

## Next local M4 unit

The immutable cell/page contract and two-page asynchronous GPU replacement are
implemented/tested, with dedicated upload fences, single-frame retirement and
actual dedicated-allocation budget pressure. Bounded offline heightfield LOD now
preserves exact boundaries and passes independent sampled-error checks. A bounded
fixed geometry arena now qualifies aligned ranges, retirement/reuse and physical
capacity rejection. A fixed perspective/depth camera now passes CPU/replay checks.
Position-only static glTF node transforms/reflections now feed the cooker/viewer
with qualified source/tool provenance. Conservative GPU meshlet frustum selection
now feeds indirect task/mesh work and passes CPU/replay checks. Cooked identity,
depth and deferred diagnostic material resolve now connect those slices. Next
qualify bounded per-material texture selection, then asynchronous dependency residency.
The CPU material-bound texture table now admits a complete bounded texture set;
GPU table lookup now passes one/two-texture execution and per-material/reuse
replay checks. Authored multiple-image normalization and v2 complete bundle admission now pass
source-byte, deduplication and GPU/replay checks. Next qualify bounded CPU
complete-resource-set staging/publication, then asynchronous GPU residency.
One shared authored external PNG now imports/cooks/samples through the fixed
bundle against source-byte and GPU/replay checks; other image/sampler/mip
profiles remain unqualified.
Fixed four-role immutable bundle admission now checks geometry/factors/UV/texture
as a complete set before Vulkan initialization; asynchronous dependency residency
and per-material texture assignment remain open.
The bounded opaque single-mip KTX2 offline producer and Rust reader now pass
independent checks. One resident nearest-repeat mip-zero texture now uploads and
samples with perspective UVs and unlit factors against CPU/replay checks;
per-material texture binding and image import remain open.
The opaque material sidecar reader/cooker, GPU lookup/resolve and opt-in glTF
unlit factor import/remapping now pass independent checks. Authored float2 UV
import, exact geometry binding and perspective-correct GPU reconstruction are
now qualified; texture/mip/dependency work remains open. The optional shared-
vertex Slang mesh / GLSL fragment interface passes replay alongside the default
expanded-vertex Slang test. General
LOD/hierarchy errors, broader page dependencies/readers and virtual texture
residency remain open. Retain the current viewer/fixture checks; M5 GI stays deferred.

Complete-bundle CPU residency now passes full-set admission/reservation,
content-addressed sharing, atomic epoch replacement and retained-reader budget
checks. Next integrate bounded asynchronous whole-set uploads and frame-safe
GPU publication; fixed-renderer texture residency is still unchanged.

Pending sampled-image transfers now qualify submit/poll/finish/cancellation,
including complete-set submission before startup waits. Next connect CPU
bundle reservations, bounded I/O and all-resource GPU upload completion to
frame-safe publication.

Two complete v2 bundles now alternate through bounded asynchronous I/O, full-set
CPU reservations, pending buffer/image uploads and frame-safe graphics generation
publication. One/two-image count changes, budget deferrals and initial/replaced/
restored replay pass. Publication uses conservative device-idle generation
retirement. General LOD hierarchy/error selection, streamed terrain integration
and texture mips/tiles remain open; next qualify a bounded LOD error contract.

A continuous vertical-error gate now qualifies bounded rectangular terrain LODs
with exact triangle overlays and producer provenance, supplementing sampled
checks. General 3D errors/hierarchies remain open. Next implement a bounded
projected-error CPU selection contract before GPU LOD selection.

Bounded projected vertical-error CPU selection now passes source-link/profile
checks, perspective-denominator bounds, clip/numeric guards and real terrain
threshold/extent/camera cases. Next qualify GPU LOD selection against this oracle;
general hierarchy, hysteresis and 3D/material errors remain open.

GPU resident two-level heightfield selection now passes the CPU projected-error
oracle, culling/indirect draws, page-byte/framebuffer checks and clip/profile
fallback qualification. General hierarchy, streamed LOD, hysteresis and virtual
textures remain open; this does not close M4. Stable authority/presentation and
resource contracts now support starting a bounded M7 authoritative movement
slice in parallel, as the roadmap's early-playable-slice priority permits.
Advanced GI remains deferred.

The permitted parallel M7 track now has a bounded authoritative movement slice
with fixed ticks, server-controlled speed/swept static collision, sequenced
inputs, stale-session rejection, non-blocking ordinary actors and stable IDs
through compaction. Next add simple authoritative interactions/combat, then
transport/replication qualification. M7 is not complete; navigation/mesh collision
and client integration remain open. M4 hierarchy/virtual textures remain open;
advanced GI stays deferred.

The parallel M7 authority store now qualifies basic server-owned point combat:
range/LOS/current-state timing, cooldown, health/death, sequenced outcomes and
coalesced health changes. M7 remains open for navigation/mesh collision, wider
interactions and client integration. Next qualify transport-facing sequencing/
idempotency independently of movement, before implementing Quinn transport;
no networking/persistence is implemented yet.

The parallel M8 adapter now qualifies independent movement/request sequence
spaces, bounded pending/result retention, conflicting retries, application ACK
holes and session-scoped replay protection. Next implement a bounded encrypted
Quinn two-client loopback qualification with DATAGRAM movement and reliable
request/outcome streams. Persistence, replication baselines and client rendering
remain open; this does not close M7 or M8.

Encrypted Quinn IPv4 loopback now qualifies two client contexts, DATAGRAM movement/
full snapshots, separate reliable request/ACK streams, server-authoritative combat,
certificate/ALPN rejection and bounded malformed/stalled-stream handling. Next
qualify encrypted exchange under controlled packet loss/reordering, then bounded
snapshot versioning/repair and client presentation integration. Persistence and
accounts remain unimplemented; M8 is still open.

Encrypted post-handshake loss/delay/reordering/duplication now passes two-client
loopback through bounded opaque UDP relays, with sequenced movement retransmission
and repeated full-state recovery. Next qualify client-side snapshot ordering and
session reset before visual integration. General delta baselines, disconnect/
reconnect repair, persistence/accounts and scale remain open.

Readonly client full-snapshot publication now qualifies monotonic tick selection,
idempotent duplicates, conflict/watermark rejection, atomic complete-set replacement,
bounded retention and local connection reset. Both encrypted modes use it. Next
qualify bounded interpolation for presentation with explicit simulation/screen
clock inputs; prediction, general deltas, lifecycle/account repair and renderer
integration remain open.

Bounded two-snapshot CPU presentation interpolation now passes explicit clock,
logical-ID/lifecycle, health, gap/clamp and large-counter checks, and both encrypted
lab modes sample received state. Next inspect renderer integration for a small
verifiable presentation slice. Production GPU smoothing, network clock estimation,
prediction, delta baselines and account/persistence remain open.

A fixed server-owned two-tick actor fixture now passes GPU interpolation/health/
64-bit-ID colour checks and preserves the preceding render modes. Next add small
interactive local-authority controls using that presentation contract, then
connect a live encrypted network feed. This does not close M6 characters, M7
navigation/gameplay or M8 live client/persistence gates.

Interactive local direction/attack controls now pass native X11 input, server-owned
combat/death/static collision, bounded fixed-step timer, GPU interpolation and
replay. This remains a same-process development authority fixture. Next connect
separate encrypted Quinn authority to the renderer's readonly presentation data;
accounts/persistence, navigation/mesh collision and general client features remain
open. No milestone completion or scale claim is implied.

Separate localhost listener and independent client bootstrap now pass trusted
public-certificate verification, server-owned actor/profile assignment, connection-
bound readonly snapshots, sequenced movement and delayed pairing. Next feed that
live encrypted state to GPU presentation. General lifecycle/reconnect, account
identity/persistence and scalable replication remain open.

The parallel M8 separate encrypted GPU viewer now qualifies two independent
clients, physical input/server-owned combat, bounded readonly presentation,
profile geometry, disconnect clearing and Vulkan/RenderDoc evidence. Next apply
the bounded post-handshake packet-fault relay to the live viewer before expanding
replication. Accounts, deltas/baseline repair, reconnect/persistence and scale
remain open; this does not close M7/M8 or deferred M4/M5 work.

Live M8 viewer packet-fault qualification now passes bounded drops/reorders/
duplicates, keyboard combat, WSI and replay. Next isolate transport-neutral bounded
delta reconstruction/missing-baseline full-state repair before changing the tested
wire version. Production baselines, accounts/persistence and scale remain open.

M8 transport-neutral full-actor deltas now qualify exact named baselines, readonly
atomic reconstruction, bounded membership, stale/foreign rejection and explicit
full-state repair under synthetic loss/reordering. Next qualify a bounded sender's
acknowledged baseline selection before wire integration. THN2 still sends full
snapshots; this does not close the state-delta/persistence/network milestone.

M8 per-recipient sender baselines now qualify explicit application ACK selection,
bounded prepared history, source reset, eviction/full fallback and repair fencing
under synthetic state/ACK loss/reordering. Next qualify versioned bounded encrypted
delta/ACK/full-repair exchange before live viewer adoption. THN2 still sends full
snapshots; accounts/persistence and remaining milestone gates stay open.

M8 separate bounded TDR1 full/delta/snapshot-ACK/repair codec now passes canonical
binary and adversarial tests. Next qualify encrypted exchange in an isolated lab
before changing live listener/viewer behavior. THN2 transport remains unchanged;
this codec alone does not close the state-delta gate.

M8 isolated encrypted TDR1 exchange now qualifies full/delta publication, explicit
snapshot ACKs and full repair under controlled packet faults. Synthetic values are
not gameplay authority. Next explicitly revise live protocol and adopt the tested
contract for actual authority/GPU clients. Accounts/persistence, production baseline
leases/feature negotiation and remaining milestone gates stay open.

Actual authority and GPU viewers now adopt acknowledged full/delta replication
through explicit THN3/ALPN revision. Faulted input/combat, WSI, replay and deliberate
baseline eviction/full repair pass. Remaining gates include general lifecycle/
roster and baseline leases, account/session recovery, persistence and scale. M8
remains open; broader Windows/Wayland qualification and M4/M5 gates also remain.

The broader platform gate now has nested-Weston Wayland Vulkan execution evidence
for six renderer modes and faulted live replication/full repair. Native Wayland
seat/user-driven minimize-restore, Wayland capture replay, Windows and other vendors
remain open. The bounded X11 reference slice stays preserved; no performance or
complete cross-platform milestone claim.

Nested Wayland now also has composited visible-pixel reference evidence for GPU
indirect counts 0/1/2. This complements validation/execution; it does not close the
Wayland debugger replay, native seat/restore, Windows or cross-vendor gates.

The platform gate now includes Windows GNU normal/test/example cross-linking,
denied-warning Clippy, 129 Wine compatibility tests and headless authority/script
execution. Winsock oversized-UDP relay handling is corrected. Native Windows/MSVC/
GPU/Quinn execution and the Wine encrypted-socket limitation remain explicit gates.
Remaining M4/M7/M8 implementation slices can proceed independently.

M4 now qualifies every slot of the bounded 16-image resident descriptor table on
RADV/X11, including exact image-budget success and one-byte-short clean rejection.
Concurrent full-capacity replacements and aggregate bundle admission at this boundary
remain separate checks; general LOD and virtual textures remain open.

M4 full-capacity replacement now qualifies 32 simultaneously allocated images,
complete-set publication and independent exact/one-byte-short CPU/GPU/image budgets.
Replay verifies the replacement at exact limits and retained materials under all
three deferrals. Conservative retirement and broader M4 gates remain open.

The same maximum-capacity resident/replacement/budget cases now pass native Wayland
surface execution and validation in owned nested Weston. Capacity pixel replay stays
X11-only; native-seat/restore, Windows and cross-vendor gates remain open.

Next M4 implementation slice: GPU adoption of the qualified CPU hysteresis contract
for the bounded resident heightfield pair. Preserve hard pixel limits and clip
fallback, initialize history explicitly and qualify temporal selection trajectories.

M4 bounded GPU LOD hysteresis now qualifies retained selection, stricter entry,
hard-limit refinement and explicit reset in both shader variants. X11 replay,
Wayland execution and Windows GNU/Wine checks pass; general hierarchy/streamed page
demand and texture mip/tile fallback remain implementation gates.

M4 now qualifies a separate complete authored mip-chain CPU/native-cooker contract.
Next qualify full-chain GPU upload and fixed mip-view sampling before automatic mip
selection, material derivatives, filtering or tiled fallback publication.

M4 complete authored mip upload and fixed-view sampling now qualify on RADV with
all-level byte proof, per-view raster/WSI and allocation-boundary cleanup. Next
implement a bounded perspective-correct UV-gradient/texture-footprint contract,
then qualify GPU automatic mip selection before filtering/tiled fallback.

M4 CPU analytic UV-gradient/footprint contract now qualifies independent numeric,
repeat-seam and cooked-scene checks. Next implement automatic GPU mip choice and
compare analytic shader outputs/visible samples with this oracle.

M4 bounded GPU automatic mip choice now qualifies analytic gradients, complete-chain
sampling and explicit unsupported-profile mip-zero fallback. Default/shared replay,
allocation limits/cleanup, preserved modes, Wayland WSI and Windows GNU/Wine checks
pass. Filtering quality, tiled/partial residency, full-chain replacement, general
hierarchy/LOD and native Windows GPU remain open.

M4 CPU bilinear-repeat sampling now qualifies decode-before-interpolation, periodic
edges and native cooked-chain samples. Next qualify opt-in GPU bilinear sampling
while preserving integer automatic mip selection and nearest paths. Trilinear/
anisotropic quality and tiled/partial residency remain separate gates.

M4 opt-in GPU bilinear sampling now qualifies against the CPU oracle with varied
texels at base/coarser mips, numeric fallback and preserved nearest paths. Broader
filtering quality and tiled/partial residency remain open. Next isolate a bounded
coarse-resident mip-tail selection/admission contract before GPU fallback publication;
full virtual texture pages/feedback and asynchronous allocation reuse stay later.

M4 coarse mip-tail planning now qualifies protected 1x1 retention, exact/short
texel budgets and source-to-image index mapping against native authored chains.
Next qualify conventional GPU suffix images using original source dimensions for
footprints and independent physical allocation limits. Tiled pages/demand feedback
and asynchronous tail replacement remain later gates.

M4 conventional GPU mip-tail sampling now qualifies protected coarse suffixes,
original-dimension footprint demand, source/image mip mapping and independent
physical limits across horizontal/vertical/odd sources. A queried-precision filter
oracle handles observed dark-edge quantization. Tiled GPU demand, asynchronous
suffix refinement/publication and multi-frame allocation reuse remain open; source
mip generation/quality can be qualified independently with mature offline tools.

M4 native box-filtered colour mip production now qualifies exact base preservation,
31 generated levels, strict runtime admission and fixed/automatic GPU consumption.
This is a bounded power-of-two opaque sRGB recipe. General source/filter quality,
odd/alpha/HDR profiles, tiled demand, async tail refinement and multi-frame reuse
remain open. Reliable qualification while a physical display sleeps is also an
independent platform/tooling gate; the observed DPMS timeout is not a renderer pass.

Owned headless display qualification now keeps GPU rendering/replay independent
of desktop DPMS: native Wayland WSI and Xwayland visibility/mip-tail replay pass.
Xwayland recovery after the frame-80 injected presentation case remains open.
Next reuse these separate environments in asset qualifiers before continuing
asynchronous mip-tail refinement/publication.

Texture qualifiers now reuse an owned headless context with explicit Wayland
validation and Xwayland replay environments, including budget and failure cleanup.
Next implement bounded asynchronous mip-tail refinement/publication while retaining
a protected coarse image until the complete finer suffix is ready. Conservative
retirement and independent physical coexistence budgets remain necessary.

M4 now qualifies one asynchronous coarse-to-finer conventional suffix replacement:
background full-source admission, transfer-fence readiness, protected-image retention
under short capacity/changed source, and conservative generation retirement before
image/metadata publication. Physical coexistence budgets are independent of dense
texel caps. Tiled GPU feedback, multi-request scheduling and overlapped multi-frame
allocation reuse remain future gates; this bounded slice does not complete M4.
[Evidence](validation/M4_ASYNC_MIP_REFINEMENT.md).

The user-approved M4 scene-benchmark target now has a first resident procedural
workload: a repeatable approach/village/crowd camera route, moving proxies, GPU
culling/near-far detail, visibility resolve, timings and live stats. It executes
0..10000 visual proxies under bounded validation/replay; it is not network-player
capacity evidence. A separate cooked-bundle mode now shares the route/recorder,
with four checked quadrant assets, diagnostic textures and exact/short CPU/GPU/
image pressure on periodic whole-bundle replacement. A mixed resident world/crowd profile now combines the complete village with
0..10000 proxies. Next qualify independent chunk residency, representative art,
geometry LOD and texture mips in this same benchmark framework. Both cooked
and procedural profiles now have optional, independently checked debug stats.
Richer lighting/temporal rendering and animated characters expand it across M5/M6;
server-controlled bots and replication/load instrumentation expand it across M7–M9.
This adds an integration/profiling target without closing those milestones.
[Benchmark scope](SCENE_BENCHMARK.md), [qualification](validation/M4_SCENE_BENCHMARK.md).

A complete resident cooked village now fits the current bounded viewer in one
28-meshlet bundle, using an error-checked coarse ground mesh while preserving all
buildings/trees. It shares the camera/timing framework and has three-phase replay
evidence. This closes the single-quadrant visual gap for a fixed resident asset;
moving proxies now share its visibility/depth in a separate mixed profile.
Independent quadrant residency remains open.

Cooked stats now read GPU meshlet counts directly and display host-owned active
allocation/texture/publication metadata plus completed timings. Exact/short budget
states and the original crowd panel pass pixel checks. This diagnostic UI retains
the existing separation from the planned general engine text/UI system.


Independent complete-bundle CPU world ownership now supports atomic named chunk
add/replace/remove against one unique encoded-byte pool and separate slot cap.
Retained world/chunk readers prevent premature byte reuse. Four real quadrant
bundles pass exact/short budgets and shared ownership checks. Camera demand and
independent GPU chunk publication remain the next integration gates; the mixed
benchmark still uses its single resident whole-world representation.


The bounded CPU demand prerequisite now selects outdoor chunks from actual
residency using horizontal bounds, load/retention bands, deterministic distance
priority and slot deferral. The actual benchmark camera plus spatial excursions
qualify it against real quadrant bundles and independent publication/byte models.
Next connect protected fallback coverage and fence-gated independent GPU owners;
the policy alone does not stream or change the displayed mixed benchmark.


A separate native GPU ownership probe now admits/uploads all four complete
quadrants independently, qualifies aggregate/image short-cap cleanup and retains
removed allocations until the last reader releases them. Its transfer fences
retire synchronously and no chunk is drawn. Next connect independent chunk owners
to shared visibility/depth/resolve and protect fallback coverage before camera
requests can change rendered residency. Existing streamed/mixed paths stay intact.


A first two-chunk resident geometry renderer now gives each page independent GPU
frustum compaction and indirect work while sharing identity/depth and diagnostic
resolve. Its frame-local chunk tag is unrelated to persistent world IDs. Next
compose complete material/texture owners and prove independent publication plus
fallback coverage before connecting camera demand. The textured crowd benchmark
still uses its single resident world bundle.


The isolated resident pair now composes both complete cooked material/UV/texture
graphs using shared CPU ownership, independent GPU owners and construction-time
aggregate physical/image caps. Distinct palettes expose accidental cross-chunk
bindings. Next qualify independent complete-chunk replacement and protected
fallback coverage before camera demand can drive rendered residency. The crowd
benchmark remains a separate single-bundle workload.


Independent complete replacement now keeps one resident chunk fixed while a
bounded worker/pending set alternates another, with protected current sets under
short capacity and fence-gated descriptor/CPU publication. Four/two-image array
transitions and 65 successive publications qualify locally. Next expand bounded
complete-world composition and spatial fallback coverage before connecting the
camera-distance oracle to runtime requests. The timed crowd route stays separate.


A separate optional storage-descriptor-array compute gate now passes 64 divergent
GPU reads from four bound buffers. Additional indexing features are opt-in and
leave existing modes unchanged. Next qualify fragment access and bounded
four-chunk composition; advertised feature/limit support is not rendering or
performance evidence. [Gate](validation/M4_STORAGE_DESCRIPTOR_ARRAYS.md).


The opt-in four-chunk fixture now composes the complete fine village with shared
visibility/depth, four GPU compactions/indirect draws and nonuniform storage/image
arrays. Distinct image counts and nonwhite material factors qualify chunk resource
selection. Next protect spatial fallback coverage and connect camera-distance
demand to independently published GPU sets, then integrate that world path with
the timed crowd workload. General streaming, geometry LOD and virtual textures
remain M4 exit gates. [Evidence](validation/M4_FOUR_RESIDENT_CHUNKS.md).


Four offline coarse-quadrant candidates now preserve the fine terrain perimeter,
qualifying all 16 mixed selections without unmatched terrain edges. Continuous
vertical error, full props/material preservation and native cooked admission are
checked. Next connect protected fallback owners to committed fine residency;
fixed resident selections alone do not provide camera-driven streaming or runtime
LOD. [Evidence](validation/M4_CHUNK_FALLBACK_GEOMETRY.md).


Protected CPU fallback ownership now wraps complete coarse bootstrap, forbids
coarse mutation and selects fine data only from committed complete graphs.
Five real-bundle routes qualify camera demand, reservations, tight/coarse-only
capacity and retained coverage. Next mirror these owners/selections in fence-gated
GPU generations, then connect runtime camera demand and the timed crowd route.
[Evidence](validation/M4_PROTECTED_CPU_COVERAGE.md).

The covered-world startup profile now keeps coarse GPU owners while attempting
fine candidates within remaining caps. Successful complete transfers retire
before fine CPU publication; short capacity keeps coarse drawing. This fixed
startup slice precedes live fine removal/fallback reactivation, camera-driven
requests and crowd-route integration. [Scope](validation/M4_PROTECTED_GPU_STARTUP.md).


A finite fine-retirement schedule now reactivates all four retained coarse GPU
graphs with graphics retirement, complete descriptor rebuild and matching CPU
epoch publication. Prefix boundaries, partial-startup coverage and fatal rebuild
cleanup qualify locally; replay checks actual coarse contents after reuse.
Next qualify bounded fine reload while coarse coverage stays visible, then connect
camera demand and the timed crowd scene. [Evidence](validation/M4_RETAINED_COARSE_REACTIVATION.md).

One bounded asynchronous fine reload now completes after coarse reactivation,
including pending-ready shutdown and exact/short coexistence caps. Transfers and
prior graphics users retire before the new descriptor/CPU generation publishes;
coarse coverage survives deferred capacity. Next generalize the region request
and connect actual camera demand/repeated transitions, then integrate the timed
crowd route. [Evidence](validation/M4_ASYNC_FINE_RELOAD.md).

Camera-driven fine demand now qualifies repeated asynchronous publication,
retirement, obsolete ready-upload cancellation and bounded capacity deferral over
protected coarse GPU coverage. Nine replays verify live ownership and rendered
results; a repeated 1320-frame route exercises 88 publications/86 retirements.
Next define proxy ground heights over mixed selected terrain and integrate this
route with the timed crowd/HUD workload. Independent per-view LOD, general asset
catalogs and virtualized geometry/textures remain separate gates.
[Scope](validation/M4_CAMERA_WORLD_DEMAND.md).

The CPU selected-terrain query now matches independent source triangles over
569680 mixed-selection heights, including dense proxy trajectories. Next qualify
the same bounded query on the GPU and account for its allocations before wiring
streamed world selection into proxy compute/rendering and timed HUD reporting.
[Evidence](validation/M4_SELECTED_TERRAIN_HEIGHTS.md).

The batched Slang selected-ground query now verifies 569680 heights with zero
validation diagnostics and bounded interpolation error against independently
qualified source data. Next integrate the table with explicit allocation charges
and query it from GPU proxy generation, then qualify shared streamed-world/crowd
visibility, resolve and HUD/timing. No per-frame readback is intended in that path.
[Evidence](validation/M4_GPU_SELECTED_TERRAIN_HEIGHTS.md).

The actual camera-streamed world now renders together with up to 10000 GPU
proxies grounded on its committed fine/coarse triangles. Explicit table charges,
23 native cases and eight replays qualify ownership, placement, culling, detail
and two-way occlusion. Next connect timed recording and a live HUD for this
combined path, preserving graph/pending/fixed allocation scope. Broader catalogs,
independent per-view LOD and virtualized geometry/textures remain open.
[Evidence](validation/M4_STREAMED_WORLD_CROWD.md).

The combined route now records finite frame timestamps and matching residency
snapshots. Independent checks cover warmup/repeat clocks, all sample ledgers,
stream counters and summary arithmetic; captures verify the corresponding pixels.
Next add a live HUD that reads the four world/proxy work counters directly on
the GPU, using completed timing data and explicitly scoped memory labels.
[Evidence](validation/M4_STREAMED_WORLD_TIMINGS.md).

The live combined HUD now qualifies direct GPU meshlet/proxy/detail counters,
completed timings and explicit asset/draw allocation scopes. Nineteen native
cases, eight replays and a longer release run pass, including varied texture
counts, pending data and frame-zero defaults. Next expose the streamed profile
through the benchmark launcher/cooker for repeatable user runs, then continue
independent per-view LOD, larger catalogs and virtualized-content gates.
[Evidence](validation/M4_STREAMED_WORLD_HUD.md).

The public benchmark launcher now exposes the streamed world/HUD route with
optional automatic eight-graph cooking/admission and JSON/CSV/interactive HTML.
Native legacy and streamed routes, tight repeated demand, export fidelity and
failure cleanup pass. Next separate per-view geometry choice from fine residency;
larger catalogs and virtualized geometry/textures remain M4 gates.
[Launcher evidence](validation/M4_STREAMED_BENCHMARK_LAUNCHER.md).

CPU protected snapshots now support independent per-view representation requests
and explicit unavailable-fine reporting. Real-graph exhaustive residency/request
checks preserve ownership and encoded charges. Next connect verified per-region
error bounds and camera policy, then qualify GPU selection/grounding separately.
The rendered benchmark still uses finest committed residency.
[Evidence](validation/M4_CPU_VIEW_SELECTION.md).
