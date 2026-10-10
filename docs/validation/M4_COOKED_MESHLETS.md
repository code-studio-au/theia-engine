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
