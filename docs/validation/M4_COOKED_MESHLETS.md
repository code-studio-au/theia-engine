# M4 initial cooked-meshlet/viewer slice — 2026-10-10

**M4 remains open.** This qualifies a small immutable asset contract, mature offline
meshlet construction, GPU decoding/drawing, and explicit encoded-byte capacity
fallback. It does not qualify world streaming, general LOD, terrain or virtual textures.

Reference: RX 7900 XTX / RADV Mesa 26.2.4 / Linux Mint X11, Rust 1.99.0,
Slang 2026.19, validation 1.3.275, RenderDoc 1.46. Offline tool is official
[meshoptimizer 1.3](https://github.com/zeux/meshoptimizer/releases/tag/v1.3),
[MIT licensed](https://github.com/zeux/meshoptimizer/blob/v1.3/LICENSE.md),
built with GNU C++ 13.3.0 through CMake Release/shared-library configuration.
Runtime has no meshoptimizer/native-cooker dependency; its reader uses RustCrypto
SHA-256 and Rust data validation. This is an evaluation of proposed ADR-019, not
approval of a final production asset schema/hash policy.

## Producer/reader evidence

Official source archive SHA-256:
`d50a0b5e1323ba4379cdb32f89436c20652b7393445f8e18c88ae64445b67b77`.
The exact native library digest, source digest, compiler, configuration, target
ABI and recipe are recorded in each payload's producer metadata. The synthetic
fixture cooker calls `meshopt_buildMeshlets` and `meshopt_optimizeMeshlet` in an
offline process, then verifies a winding-preserving triangle multiset before writing.
Native sources/library and binary assets stay under ignored build directories.

| Fixture | Encoded bytes | Vertices | Meshlets | Triangles |
|---|---:|---:|---:|---:|
| Grid16 | 7,244 | 289 | 8 | 512 |
| Linked coarse quad | 848 | 4 | 1 | 2 |
| Separate triangle | 736 | 3 | 1 | 1 |

Full grid file digest: `4265ed803affd21257733e67a59070e2d0a057c9e82824e68677726f9f0dd6e0`.
A repeated cook with the same library/configuration reproduces that exact address.
The coarse quad links to the grid's source digest and preserves its XYZ bounds.
This is manually authored coarse planar geometry, not an implemented hierarchy
simplifier/error metric. Three Rust tests verify valid container data; corruption,
version/length/budget failures; every truncated prefix; and rehashed but invalid
positions, ranges, references and padding. Native producer and Rust inspect succeed.

## GPU evidence

`--asset <content-addressed-file>` validates before uploading once through a
bounded staging buffer into device-local storage. Transfer-to-compute/mesh barriers
publish immutable bytes. Slang reads the asset header, writes indirect command/count
and emits decoded meshlets; raw positions/remaps/u8 local triangle indices are
consumed by mesh shaders. No conventional vertex/index buffers or host per-object
lists/count readback. GPU limits for 64 mesh invocations, 64 output vertices/
primitives, output memory/components, downstream groups and storage range are checked.

Observed excerpts:

```text
Cooked asset verified: 7244 bytes, 289 vertices, 8 meshlets, 512 triangles
Immutable GPU mesh uploaded once: 7244 bytes; no runtime native-library dependency
GPU cooked-mesh work initialized: 64-byte device-local command/count; meshlet count at 36; immutable geometry at binding 1; no count readback
Presented 120 frames
Render Lab teardown: 10 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors
```

The WSI smoke cycles empty/first/all meshlet scenarios, runs four labeled recovery
probes and resize/minimize/restore. Final normal smoke and linked-fallback smoke
also pass. Separate capture/replay (validation disabled) verifies all immutable GPU
bytes against the original file, indirect dimensions/count and downstream meshlet
count, plus 81 raster coverage/color samples per scenario against independently
decoded CPU geometry:

| Scenario | Meshlets | Triangles | Verified samples |
|---|---:|---:|---:|
| Empty | 0 | 0 | 81 |
| First | 1 | 64 | 81 |
| All | 8 | 512 | 81 |
| Capacity fallback/all | 1 | 2 | 81 |

With `--asset-budget 2048 --fallback <linked-quad>`, the 7,244-byte grid selects
its validated 848-byte source-linked coarse representation. Source relation and
matching bounds are required. Full high/coarse replay PNG comparison over every
RGB pixel differs by at most **one byte level**. This establishes planar fixture
coverage/color, not general LOD seam/error correctness. This budget covers selected
encoded geometry bytes, not staging/alignment/framebuffer/whole-VRAM usage. Selected
geometry stays resident for the viewer lifetime; this is not asynchronous page eviction.

Over-budget without fallback, unlinked triangle fallback and corrupted content
exit 1 with explicit errors and zero validation diagnostics. Missing asset mesh
shader after successful upload/work resource creation exits 1 and releases resources
with zero teardown diagnostics. All six earlier render modes pass 120-frame smoke
regressions with zero warnings/errors. Preserved headless compute verifies all
256 values; M1 again verifies 526 batches/8,717 commands with zero diagnostics.
Seventeen render shader outputs validate (the recorded Slang material profile
warning remains); local Rust fmt/denied-warning Clippy/tests pass. CI configuration
includes asset-reader tests and native fixture qualification, but remains local
and no hosted CI result is claimed. No GitHub push or review monitoring occurred.

## Remaining M4 work

Authored scene/material/camera interfaces and integration with M1/M2/M3, general
meshlet LOD/hierarchy/error/seams, immutable cell manifests, asynchronous I/O,
staged publication/retirement, real per-arena resident budgets and protected page
fallbacks, terrain, virtual texture feedback/pages, cross-platform execution and
performance. Raw float positions/u8 indices are not a compression/LOD implementation.
No locked ADR changed. Commands: [developer workflow](../DEVELOPMENT.md#m4-offline-cooked-fixtures).

## Subsequent two-page residency qualification — 2026-10-10

The original `--asset` path above is preserved. New `--cell` qualifies the bounded
manifest/lifetime contract against actual GPU replacement. Cell 7's manifest ID
is `3f98084e509f69cf620e23b9a8612d24127edf5fe8bffdfaeeea717d550facdd`.
It references the grid/linked quad and begins with the protected coarse page.
Subsequent file reads run on one bounded worker; one pending upload uses a
separate command pool/fence and coherent staging buffer. Success never waits the
whole queue/device idle. Descriptor replacement is ordered after both upload
completion and the sole frame fence; immutable CPU snapshots retire with that
frame. Teardown still waits swapchain/work resources conservatively.

Actual Vulkan allocation requirements: grid 7,248 bytes (encoded 7,244), quad
848 bytes; maximum overlapping geometry is **8,096 bytes**. Dedicated geometry
budget includes both active and pending allocation. Staging has a separate
64 MiB allocation cap; it uses 7,248/848 bytes here. This is not an arena heap,
whole VRAM accounting or performance result.

| Run | Observed result | Validation including teardown |
|---|---|---|
| Default 16 MiB / WSI | Five replacements, 120 frames, ten generations | 0 warnings / 0 errors |
| Geometry budget 8,096 / WSI | Five replacements, 120 frames, ten generations | 0 / 0 |
| Geometry budget 8,092 / WSI | Five deferrals, protected coarse retained, 120 frames | 0 / 0 |
| 22-frame shutdown | Pending unpublished GPU upload retired during teardown | 0 / 0 |
| Geometry budget 847 | Startup exits 1 before coarse memory allocation | 0 / 0 |

Four separate RenderDoc 1.46 captures (validation disabled) verify selected
immutable GPU bytes, command/count, meshlet count and 81 pixel samples each:
initial coarse/frame 10 (one meshlet/two triangles), fine/frame 30 (eight/512),
replacement coarse/frame 50 (one/two), and protected coarse/frame 30 under
8,092-byte budget (one/two). All target runs present 180 frames. The capture helper
requires explicit expected page content, so unexpected upload timing/page identity
fails qualification instead of being assumed correct.

Local artifacts: `build/qualification/m4-cell-{wsi,tight,exact,pending-shutdown,too-small}.log`
and `m4-cell-renderdoc-{coarse,fine,retired-fine,budget}/report.json` with captures/PNGs.
All seven prior modes pass 120-frame synchronization-validation regressions.
Render Lab's nine unit tests, fmt and denied-warning Clippy pass. No shader change
was required. Work remains local; no hosted CI/push/review result is claimed.

Limitations: bootstrap synchronously validates both fixture files/source relation;
subsequent I/O is asynchronous. One graphics/compute/transfer queue and one frame
are used, so multi-frame readers/dedicated transfer queues are unqualified. The
CPU oracle independently tests retained readers, but the renderer releases its
reader after its actual fence. No cancellable file reads, general dependency graph,
spatial cell selection, production arenas, general LOD/error/seams, authored scene,
terrain or virtual textures are implemented by this unit. M4 stays open.

## Subsequent offline heightfield LOD qualification — 2026-10-10

Official meshoptimizer 1.3 `meshopt_simplify`, called offline through the pinned
bulk ABI, uses LockBorder|ErrorAbsolute, triangle target ratio 0.25 and absolute
native error limit 0.02. Existing meshlet construction then verifies the complete
simplified input triangle multiset/winding. No native runtime dependency is added.

| Fixture | Input/output triangles | Meshlets | Encoded bytes | Independent sampled height difference |
|---|---:|---:|---:|---:|
| Planar grid LOD | 512 / 128 | 2 | 5,528 | 1.67e-16 (5,062 points) |
| Curved terrain LOD | 512 / 128 | 2 | 5,528 | 0.0122542699 (5,059 points) |
| Full curved terrain | 512 / 512 | 8 | 7,248 | Reference |

Both LODs preserve all **64 original boundary edges** by exact index/position
identity. Independent Python checks require positive nondegenerate XY winding,
no edge with more than two incident triangles, identical boundary edge set and
projected area. It compares heights/coverage at every source vertex, both sets
of triangle centroids and a 65x65 grid. Native terrain error is 0.00903258845,
which is a different metric from vertical difference. No analytic/global Hausdorff
bound follows from either these sampled checks or the fixture qualification.

Four Python tests exercise a known 0.1 center peak, strict error rejection,
changed boundary/reversed winding/nonmanifold edges, coverage holes, overlapping
inconsistent heights and invalid/nonfinite inputs. Real terrain cook with sampled
height limit 0.001 exits 1. Repeated terrain LOD cooking reproduces the exact file
ID `2fea2220d96b718f5f4cba1055d89c0f49b91fd98e79932b5c0d433a8266cc21`.
Full terrain ID: `ec117bdb76cfa189ad44957e80117497fb317fa56891edf363341e0679c3a7dd`.
Original grid recipe still produces its previously qualified content ID.

New cell 8 ID: `003dd7d145925e546db628f5333a61c8e542bd55bdf0253ad9f131ec4d80d860`.
Rust inspect and file-backed lifetime example pass on these pages. RX 7900 XTX
stream WSI and explicit 6,000-encoded-byte fallback WSI each present 120 frames,
ten generations, zero warnings/errors including teardown; stream replaces five
pages. Physical geometry allocations are full 7,248 / coarse **5,536** bytes;
staging is separately tracked/capped. Two RenderDoc captures verify full/coarse
GPU bytes/counts, 512/128 selected triangles and 81 XY color/coverage samples each.
The current asset viewer has no depth attachment/perspective camera: GPU captures
verify XY coverage/color and uploaded positions, not terrain depth appearance.

Artifacts under ignored `build/qualification/m4-terrain-*` include cooker reports,
strict-failure log, WSI logs and separate RenderDoc reports/captures/PNGs. Updated
local CI configuration cooks both terrain pages, runs analytic tests and the Rust
reader/lifetime example. Nothing was pushed; no hosted CI result is claimed.
General authored geometry, material/UV preservation, conservative hierarchy error,
projected-error selection, adjacent-cell transition seams and production arenas
remain unqualified. M4 remains open.

## Subsequent fixed geometry arena qualification — 2026-10-10

Current cell streaming supersedes dedicated per-page geometry allocations with
one fixed device-local storage/transfer buffer. A Rust aligned-range allocator
leases subranges; pending uploads and current descriptors own those leases until
upload/frame fences retire. Last-owner release coalesces adjacent ranges. Three
additional tests check retained clones, fragmentation, checked overflow/ticket
exhaustion and 5,000 randomized reservations against an independent occupancy
bitmap. Eleven asset tests and nine renderer tests pass; fmt and denied-warning
Clippy pass for both packages. Range identities are internal, never asset IDs.

Actual RX 7900 XTX storage descriptor alignment is **4 bytes**. The grid reserves
7,244 bytes at offset 848; quad reserves 848 at offset 0. Default arena allocates
16,777,216 bytes once. Both upload copy destinations and descriptor binding 1
include page offsets, and transfer publication barriers cover only that range.
Staging still uses a separately capped allocation per upload (64 MiB maximum).
Physical backing size, live reserved range bytes and staging are distinct charges.
No normal upload/replacement performs queue/device idle waits.

| Run | Observed result | Validation including teardown |
|---|---|---|
| 16 MiB arena / WSI | Five replacements, 120 frames, ten generations | 0 warnings / 0 errors |
| 8,096-byte arena / WSI | Five replacements | 0 / 0 |
| 8,080-byte arena / WSI | Five deferrals; coarse retained | 0 / 0 |
| 8,092-byte backing request | Vulkan needs 8,096 bytes; startup exits 1 within strict cap | 0 / 0 |
| 832-byte arena | Coarse range does not fit; startup exits 1 | 0 / 0 |
| 22-frame shutdown | Pending unpublished range upload retires safely | 0 / 0 |
| Missing asset mesh shader after arena/upload | Construction exits 1; shared allocation released safely | 0 / 0 |

Four RenderDoc captures independently verify range bytes, GPU-generated counts
and 81 XY color/coverage samples each: initial quad offset 0/frame 10, grid
**offset 848**/frame 30, reused quad offset 0/frame 50, and retained quad under
8,080-byte capacity pressure. Expected arena size/range offsets are explicit
capture inputs and checked against replay bytes and resulting counts/pixels.
All seven older render modes pass 120-frame validation regressions. Curved-terrain
cell streaming also passes (five replacements, zero diagnostics) in the arena.
Shader code is unchanged. Captures remain separate from validation.

Artifacts: `build/qualification/m4-arena-*` logs and
`m4-arena-renderdoc-{coarse,fine-offset,reused,budget}/report.json` with captures/PNGs.
The earlier 8,092-byte *dedicated-page* deferral result above remains historical;
the current arena request of that size fails its physical backing cap. Current
commands are updated in DEVELOPMENT. Changes remain local without hosted CI claims.

This qualifies a bounded host range allocator, fixed GPU allocation, range-scoped
upload, descriptor publication and safe reuse on one frame/queue. GPU demand/
allocation queues, multi-frame readers, diverse arena classes, staging reuse,
authored geometry/cameras, general hierarchy/LOD/seams and virtual textures remain
open. No throughput/latency or cross-platform qualification is claimed; M4 stays open.

## Subsequent perspective/depth qualification — 2026-10-10

New optional `--camera-test` for asset/cell paths transforms immutable meshlets
through a fixed Z-up RH perspective rig. Aspect follows swapchain extent; explicit
Vulkan 0..1 depth/Y direction and four float4 push rows are part of the reference
ABI. D32 depth attachment/LESS state and per-generation image lifetime are active.
The GPU still generates indirect counts and decodes geometry; no count readback,
conventional vertex/index buffers, runtime native dependency or CPU object draws.

Slang camera mesh output passes Vulkan 1.3 SPIR-V validation; all 18 render outputs
publish successfully. The earlier resolve compiler profile warning remains; no
new language fallback is used. Three independent CPU tests verify target/near/far
projection (f32 near-plane tolerance 4e-6), aspect/Y direction and center-ray
occlusion with perspective-correct attributes. Twelve renderer tests, fmt and
denied-warning Clippy pass. Runtime checks D32 attachment support and 64-byte
mesh push capacity; pipeline/image ownership handles creation failures.

RX 7900 XTX terrain/cell camera WSI smoke: five page replacements, 120 frames,
ten generations, **zero warnings/errors including teardown**. The near/far camera
fixture and all eight prior rendering paths pass 120-frame regression smokes
with zero diagnostics. Missing camera mesh after geometry arena/upload and D32
image creation exits 1, releasing resources without diagnostics.

Separate RenderDoc capture/replay checks full selected GPU bytes, meshlet/command
counts and 81 perspective color/depth samples each against CPU reconstruction:

| Capture | Selected meshlets | Covered sample points | Depth/color checks |
|---|---:|---:|---:|
| Curved coarse terrain / frame 10 | 2 | 16 | 81 |
| Curved full terrain / frame 30 | 8 | 16 | 81 |
| Empty full-terrain input | 0 | 0 | 81 |
| Near-before-far occlusion | 1 | 14 | 81 |

Replay uses absolute depth tolerance 2e-6 and RGB tolerance two byte levels.
The depth fixture is 884 encoded bytes/eight vertices/four triangles/one meshlet,
ID `f68a89311dff920a430fdb9752219c4996a8a734e8dd9b6d0d974f7f2d71bfff`.
The cooker deliberately orders z=0.65 triangles before z=0.35 triangles after
native optimization, and verifies that order and the complete triangle multiset.
Thus later far geometry must fail depth in the overlap. This diagnostic ordering
is recorded in provenance and is not a generic optimization policy.

Artifacts: `build/qualification/m4-camera-*` shader/cooker/WSI/regression/failure
logs and `m4-camera-renderdoc-{lod,full,empty,occlusion}/report.json` with captures/
PNGs. Each capture target presents 180 frames. Standalone validation and captures
remain separate. CI recipe now includes the occlusion cook, but no hosted result
or push is claimed. Changes are local.

Limits: fixed camera, raw position color, one frame/queue; no controls, authored
node transforms, material/UV import, M1/M2/M3 integration or performance claim.
CPU oracle handles positive-w near/far-unclipped triangles only and rejects other
cases; ordinary GPU clipping is active but broader clipping is unqualified.
The first 2D viewer's earlier absence of depth remains historical. M4 stays open.

## Subsequent static glTF geometry qualification — 2026-10-10

Separate offline Rust tool pins `gltf` 1.4.1; MIT/Apache-2.0 license declarations
and license files were verified in the downloaded crate. Its utility/name features
are enabled without runtime image import. Embedded base64, relative external and
GLB binary buffers pass bounds/stride/padding checks before typed accessor decoding.
Selected static scene nodes bake parent/local transforms, RH Y-up meters to RH
Z-up `(x,-z,y)`, and reflection-aware index winding. GPU-facing mesh schema stays v1.

Four tests pass: analytic nested rotation/translation/reflection, independent
shared-mesh instances, invalid buffer/accessor/URI/tree/transform/material/topology
cases, and external/embedded/padded-GLB geometry parity. Rust fmt and denied-warning
Clippy pass. Source/root buffers are each capped at 64 MiB, output at 8,192 vertices/
2,048 triangles and traversal at 4,096 nodes/128 edges depth. These are logical
input/output bounds, not a total RSS budget or complete Khronos compliance validator.
Unsupported attributes/materials/skin/morph/animation/extensions/cameras fail.

Synthetic source `fixtures/gltf/transformed_scene.gltf` references one quad mesh
from two nodes; parent translation plus reflected/rotated/raised second instance
are baked. Result: **eight vertices/four triangles/one meshlet/2,072 encoded bytes**.
Final qualified cooked ID:
`985ee9f76a7ad0707749a4969de7b2617a4a2cc853c41b92af541053f6a9757f`.
Normalizer metadata includes document/buffer hashes, node ranges/reflections,
exact parser/tool, compiler/OS/arch/debug configuration and source/lock/binary
hashes. Native cooker adds normalized JSON/source/configuration/library digests.
The same normalizer binary/source produces byte-identical normalized JSON on repeat;
rebuilds/other compilers are intentionally distinct producer identities.

Rust container inspect succeeds. RX 7900 XTX camera WSI run: 120 frames, ten
swapchain generations, **zero validation warnings/errors including teardown**.
Separate 180-frame RenderDoc capture (validation disabled) verifies immutable GPU
bytes, command/count, one meshlet and 81 perspective color/depth samples against
CPU reconstruction. Artifacts: `build/qualification/m4-gltf-final-*` reports/logs/
capture/PNG; normalized/cooked files remain ignored under `build/assets`.
CI configuration adds Linux/Windows Rust checks for the tool and Linux offline
import/cook/inspect; nothing is pushed and no hosted result is claimed.

Limitations: geometry-only static subset; copies geometry per instance, no GPU
instance table/material/UV/normal import, authored camera/control, collision/nav,
page-demand/frustum selection or complete static scene database. Names/indices
are source provenance, not durable world IDs. Arbitrary third-party exports and
Windows runtime remain unqualified. M4 remains open. Workflow: DEVELOPMENT and
[tool contract](../../tools/theia-gltf-import/README.md).

## Subsequent GPU meshlet frustum qualification — 2026-10-10

New `--frustum-test` extends the cooked page/camera reference with GPU-resident
candidate selection. One 32-lane compute group intersects homogeneous outside-
plane masks across every referenced vertex of each meshlet, atomically compacts
candidate IDs/count/mask, and writes the indirect task command/count. A shared
outside plane rejects the meshlet; surviving hulls may still rasterize nothing.
Task emits the candidate count, mesh stages decode the compacted ID rather than
group index, and existing perspective/depth drawing consumes selected geometry.
No runtime host list/count readback, CPU per-object sort or individual draw calls.

Mutable work is 256 bytes; command/count at 0..16, candidate count at 36, mask at
40, 32 IDs at 64..192. Both compute and mesh use identical 64-byte camera rows.
Resource-scoped reuse/publish barriers and within-group synchronization cover
command/count, shared state and ID writes. Actual SPIR-V has two Workgroup uint
variables (eight shared bytes). Device limits query 32 compute lanes, shared size,
storage/push ranges plus existing mesh/task profile. Slang's two additional stages
compile/validate; all **20 render outputs** publish. No language fallback.

CPU analytic test verifies wholly outside rejection and crossing-hull retention.
Thirteen renderer tests, fmt and denied-warning Clippy pass. RX 7900 XTX curved
terrain cell WSI presents 120 frames/ten generations, replaces five pages, and
reports **zero validation warnings/errors including teardown**. Nine prior paths
(indirect/clear/mesh/task/spatial/visibility/asset/camera/cell) pass 120-frame GPU
regressions. Missing cull mesh after work/arena/upload/depth construction exits 1
and releases resources with zero diagnostics.

Separate full-terrain replay captures match independently decoded CPU candidate
IDs/masks/counts and 81 perspective color/depth samples each at 960x540:

| View | CPU/GPU candidate IDs | Mask | Covered sample points |
|---|---|---:|---:|
| Translated empty camera | [] | 0 | 0 |
| Narrow left / 20-degree FOV | [0,1,2,3,4,6] | 0x5f | 53 |
| Full / 60-degree FOV | [0,1,2,3,4,5,6,7] | 0xff | 16 |

Atomic compaction order is not a persistent ordering contract; replay compares
sorted IDs and exact masks/counts. An additional cell/frame-30/narrow capture
checks the same IDs/pixels after page publication, with immutable full-terrain
bytes verified at **arena offset 5528**. Every capture target presents 180 frames.
Captures disable validation and remain separate from standalone validation runs.

Artifacts: `build/qualification/m4-frustum-*` shaders/assembly/WSI/regression/
failure logs, `m4-frustum-renderdoc-{0,1,2}/report.json` and
`m4-frustum-cell-renderdoc/report.json` with captures/PNGs. No new hosted CI result,
push or benchmark is claimed; changes remain local. General HZB, spatial-cell
integration, projected-error LOD, feedback/demand streaming, material/visibility
integration and multi-frame readers remain open. M4 remains incomplete.
