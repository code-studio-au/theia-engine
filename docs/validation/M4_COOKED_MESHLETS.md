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
