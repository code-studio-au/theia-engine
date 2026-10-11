# Theia — Actual Implementation Status (2026-10-11)

**This file separates observed evidence from planned systems.** The original handoff
records user-supplied terminal evidence. Entries below record independently executed local qualification; the latest
update is 2026-10-11.

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
