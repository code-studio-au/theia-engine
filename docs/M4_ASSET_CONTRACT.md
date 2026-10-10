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

Integrate authored geometry/material/camera contracts and the M1/M2/M3 data paths;
qualify general meshlet LOD/error/seams, immutable cell manifests, asynchronous
I/O/staged publication/retirement, actual per-arena resident budgets/protected
fallbacks, terrain and texture page residency. No M4 completion or GI work follows
from this first cooked-asset slice.
