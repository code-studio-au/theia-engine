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
