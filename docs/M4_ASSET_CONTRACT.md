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

Integrate authored geometry/material/camera contracts and the M1/M2/M3 data paths;
qualify general meshlet LOD/error/seams, immutable cell manifests, asynchronous
I/O/staged publication/retirement, actual per-arena resident budgets/protected
fallbacks, terrain and texture page residency. No M4 completion or GI work follows
from this first cooked-asset slice.
