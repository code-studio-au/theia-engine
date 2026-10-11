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
qualify authored image import and material/dependency contracts.
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
