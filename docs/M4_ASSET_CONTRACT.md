# M4 prototype immutable meshlet contract

This evaluates proposed ADR-019 under the approved mature-offline-tool policy. It
is a qualification format, not an approved final asset schema or a complete world
streamer. Runtime reader is pure Rust; native meshoptimizer is used only offline.

## Version 1 byte layout

All integer/float words are little-endian. The 76-byte header is:

| Byte offset | Value |
|---|---|
| 0 | Eight bytes `THEIAMES` |
| 8 | Schema version 1 / u32 |
| 12 | Unpadded producer metadata byte length / u32 |
| 16,20,24,28 | Vertex count, meshlet count, global-reference count, local-index byte count / u32 |
| 32,36 | Maximum vertices/triangles per meshlet / u32, each 1..64 in this prototype |
| 40 | Complete padded payload length / u32 |
| 44 | SHA-256 of the payload / 32 bytes |

Payload: UTF-8 producer metadata padded with zero bytes to four-byte alignment;
float32 XYZ positions; four-u32 meshlet records; u32 global vertex references;
u8 local triangle indices padded with zero bytes to four-byte alignment. Meshlet
records contain vertex-reference offset, local-index byte offset, vertex count
and triangle count. Each triangle has three local indices. Padding supports
aligned GPU word loads; it is not part of the logical triangle-index length.

The full-file SHA-256 is its content address and filename. The producer metadata
records source digest, exact native-library digest, compiler/version, tool/source
release digest, build recipe, configuration and target ABI. Metadata is opaque
UTF-8 to the general reader; this producer uses sorted compact JSON. Changing the
producer, source, configuration or data changes the content address.

Reader validates encoded-byte budget (default 64 MiB), schema/length/digests,
canonical padding, finite positions, meshlet ranges and local/global indices
before upload. The lab requires a 64-hex filename matching full-file digest and
at most 32 meshlets; GPU storage/mesh limits are queried separately. Encoded-byte
limits are not a complete decoded-memory or physical VRAM budget.

## Qualification producer and fallback

Official meshoptimizer 1.3 builds/optimizes meshlets in an isolated offline Python
process using a bulk native API. Fixture cooker verifies a winding-preserving
triangle multiset against its input before publishing. Native source/library and
outputs stay under ignored `build/`; no C++ source/library enters runtime packages.

Synthetic grid: 289 vertices, 512 triangles, eight meshlets, 7,244 encoded bytes.
The manually authored coarse quad has four vertices/two triangles/one meshlet,
848 bytes, and metadata linking to the grid's source digest. Capacity fallback
requires that source link and identical XYZ bounds. Executed flat-fixture pixel
comparisons verify equivalent planar coverage/color; bounding boxes alone do not
prove general LOD coverage. A general hierarchy/error/seam cooker remains open.

The viewer initializes one selected immutable GPU buffer once, retires staging,
and lets Slang read header/counts and decode meshlets. Empty/first/all scenarios
are diagnostic inputs, not a CPU per-object list. Capacity fallback is explicit;
corruption, unknown schema, missing fallback, and incompatible source links fail.
The selected encoded geometry stays within the chosen byte budget. Staging,
allocation alignment, pipeline/framebuffer memory and multi-asset residency are
not included in that budget.

## Next M4 gates

### Bounded cell/page CPU lifetime prototype

`THEIACEL` v1 manifests use little-endian words: eight-byte magic, u32 version
and page count, u64 logical cell identity, then 40-byte records containing a
32-byte full mesh-file SHA-256 and u64 encoded length. Records must be strictly
hash-sorted, unique, and number 1..4096; each page is 76 bytes..64 MiB. The
manifest's full SHA-256 is its filename/content address. Cell identity is
independent of a page's physical allocation. `tools/package_cell.py` packages
existing hash-named files; the Rust reader additionally validates each mesh
container before staging. This prototype has no dependency graph, spatial bounds,
page offsets, material pages or automatic fallback selection.

`theia_assets::residency` is a **CPU lifetime oracle**. Staging reserves immutable
encoded bytes, deduplicated by content ID within one store. Publication produces
a new read-only snapshot at a checked epoch. Removing a page from the current
snapshot does not release its allocation while old snapshots or staged tokens
retain it. Dropping the last owner releases its byte charge; failed publication
leaves the current snapshot unchanged and consumes/discards its staged token.
Tokens from another store are rejected. Rust ownership supplies reader release;
there is no GPU fence/completion proof or asynchronous I/O in this unit.

The budget covers unique live encoded page bytes, including staging and retained
readers. It excludes decoded vectors, snapshot maps, Arc overhead, file-read
buffers, Vulkan allocation alignment and GPU memory. It is not the actual
per-arena budget required by M4. The file-backed example verifies a retained
7,244-byte grid blocks another page within that budget, then releases it and
publishes an 848-byte quad. This tests retirement, not automatic LOD fallback.

### Two-page Vulkan publication prototype

Render Lab `--cell` accepts exactly two mesh pages of distinct sizes, validates
the coarse source link/bounds, and boots the smaller page. Every 20 presented
frames it requests the other page on one bounded file worker. One upload owns
its staging/destination buffers, command pool and dedicated fence; completion
publishes transfer writes to compute/mesh reads. The frame loop polls that fence
and keeps the current descriptor while upload is incomplete. It updates binding
1 only after the sole frame fence retires, releases its CPU snapshot, and frees
the old GPU allocation after replacement. Shader counts remain GPU-generated.

`--resident-budget` (default 16 MiB) bounds actual dedicated geometry allocation
bytes, including the current page and pending replacement. Staging is separately
bounded to one allocation of at most 64 MiB. Other Vulkan memory is excluded.
This is a physical allocation budget prototype, not a preallocated arena or whole
VRAM manager. Capacity rejection preserves the current page; other upload errors
remain fatal. At startup both fixtures are synchronously validated to qualify the
coarse relation; subsequent replacement reads and uploads are asynchronous.
Only one frame and one graphics/compute/transfer queue are qualified. A worker
read cannot be cancelled mid-read; teardown joins it and retires upload/frame
work before destroying their allocations. CPU store retains at most 128 MiB of
encoded pages, separately from decoder/file-read overhead.

### Fixed geometry arena update

The current cell path supersedes per-page geometry allocations with one fixed
device-local storage/transfer buffer. Its Vulkan allocation requirement must fit
`--resident-budget`; there is no silent rounding above the budget. Host range
leases round page lengths to max(4, minStorageBufferOffsetAlignment), retain
ownership across pending uploads/readers, and coalesce free ranges only after the
last lease releases. Descriptor binding 1 uses each range's offset/encoded length;
shader byte offsets remain relative to the page. Copy/publish barriers are scoped
to that range. GPU handles and range offsets never become persistent asset IDs.

Physical backing allocation stays fixed through replacement. Active/pending range
charges are distinct from that allocation's size; staging remains separately
bounded to 64 MiB. On this GPU descriptor alignment is 4, so grid/quad ranges
charge 7,244/848 bytes despite the old grid dedicated allocation needing 7,248.
An 8,096-byte arena permits replacement; an 8,080-byte arena preserves coarse.
The 8,092-byte arena request fails because Vulkan rounds backing to 8,096 bytes.
Range exhaustion/fragmentation preserves the current page. The CPU arena oracle
tests retained clones and randomized nonoverlap/reuse independently. Actual GPU
reader retirement still uses the sole frame fence; multi-frame overlap is open.

Integrate authored geometry/material/camera contracts and the M1/M2/M3 data paths;
qualify general meshlet LOD/error/seams, immutable cell manifests, asynchronous
I/O/staged publication/retirement, actual per-arena resident budgets/protected
fallbacks, terrain and texture page residency. No M4 completion or GI work follows
from this first cooked-asset slice.

### Fixed perspective camera reference

`--camera-test` leaves page schema/data unchanged and adds a dedicated camera mesh
entry. Four float4 rows (64 bytes) are pushed to the mesh stage, with explicit
layout and queried capacity. The fixed Z-up rig, RH perspective, Vulkan depth
0..1, positive viewport/Y flip and actual aspect are specified in DEVELOPMENT.
One D32 depth attachment belongs to each swapchain generation; barriers order
prior depth access/discard and current fragment tests. Depth uses LESS/clear one.
The fragment path still outputs diagnostic position color, without M3 material
resolve. Existing default XY and M3 reference pipelines remain separate.

CPU oracle interpolates screen-space depth and perspective-correct world attributes
and selects the nearest triangle. Its analytic camera/center-ray tests and replay
checks qualify unclipped positive-w fixtures; it rejects near/far clipping cases.
This is not an authored camera schema, input controller, general CPU rasterizer,
complete static scene database or visibility/material integration.

### Bounded static glTF normalization

The offline Rust tool evaluates `gltf` 1.4.1 with a validated position-only subset;
THEIAMES v1 remains unchanged. Each static mesh instance bakes parent/local matrix
composition and explicit RH Y-up to Z-up conversion. Negative world determinant
swaps indices 1/2 to retain intended face orientation. Instances currently
materialize separate dense vertex/index ranges; no GPU instance table is implied.
Original node indices/names are provenance only, not persistent world identities.

Normalized JSON schema 1 carries positions/indices and document/buffer/tool hashes,
node ranges/reflection records, compiler/target/debug configuration and exact
normalizer source/lock/binary digests. The cooker incorporates that provenance and
JSON digest alongside existing meshoptimizer/library/configuration hashes. Native
meshlet triangle-multiset checks apply to the normalized geometry. The normalizer
has no runtime dependency and rejects unsupported source features instead of
silently discarding them. Bounds, file/geometry caps and supported containers:
[offline tool contract](../tools/theia-gltf-import/README.md).

### GPU meshlet frustum work ABI

`--frustum-test` uses the existing immutable page and fixed arena with separate
256-byte mutable work storage. Words at bytes 0/4/8 are task group XYZ, byte 12
is indirect draw count, byte 36 candidate count, byte 40 candidate mask; up to
32 compacted meshlet IDs begin at byte 64. Compute uses 32 lanes/eight shared
bytes and the same 64-byte camera rows as mesh projection. It rejects a meshlet
only when all its referenced vertices lie outside one common homogeneous clip
plane. This is conservative; a surviving candidate may still rasterize nothing.
No cooked bounds/hierarchy, HZB or LOD error selection is implied.

Within-group synchronization publishes shared count/mask and ID stores; scoped
Sync2 publishes work to indirect/task/mesh reads. Mesh group index addresses the
compacted ID list, not directly the page's meshlet array. Counts/lists are never
read by the renderer host. CPU/reference/RenderDoc readbacks are diagnostic only.
Frame fence and upload/arena lease rules remain unchanged across page replacement.
This extends static meshlet selection; M2's spatial table and M3's independent
visibility/material fixture are not yet a unified scene pipeline.

### Cooked identity and diagnostic resolve

`--material-test` reuses the frustum work ABI, camera and immutable geometry
range. Identity is R32G32_UINT `(meshletID + 1, localTriangleID)`; zero first word
is background. These IDs are frame/page-local, never stable gameplay handles.
The identity pass emits three vertices per triangle with identical flat IDs,
up to 192 output vertices/64 triangles. Its qualified device profile explicitly
requires 192 mesh output vertices and 8192 output-memory bytes. This is a bounded
Slang interface qualification workaround, not a general meshlet storage change.

Resolve binding 4 is the same immutable storage-buffer range, published to
fragment reads by upload Sync2 and rebound only after the sole frame fence.
Sixty-four fragment push bytes contain the same camera rows as compute/mesh.
Resolve decodes selected triangle vertices, reconstructs perspective-correct
world position, checks interpolated depth and samples the existing two-texture
array using meshlet parity. This palette is diagnostic; no authored UV, normal,
material, derivative or virtual-texture contract is added. M3 standalone remains
available. Shared-vertex per-primitive fragment input remains unqualified.
