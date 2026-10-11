# M4 prototype immutable meshlet contract

This evaluates proposed ADR-019 under the approved mature-offline-tool policy. It
is a qualification format, not an approved final asset schema or a complete world
streamer. Runtime reader is pure Rust; native meshoptimizer is used only offline.

## Bounded resident texture profile

The initial texture container is standard KTX2, with full-file SHA-256 naming.
`theia_assets::texture::Texture` accepts only opaque 2D R8G8B8A8_SRGB (VkFormat
43), one mip, one face, no array/depth/supercompression, dimensions 1..1024 and
an encoded budget of 4 MiB. The 104-byte header/one-level index is followed by
the canonical 92-byte RGBSDA/BT709/sRGB DFD, bounded key/value metadata, then
dense RGBA8 texels. Section offsets/lengths must match this contiguous profile;
other valid KTX2 layouts fail explicitly. Checked ranges, canonical zero padding,
opaque alpha bytes and an optional expected full-file hash are required.

Metadata is limited to 16 KiB and unique UTF-8 `KTXorientation=rd`, nonempty
`KTXwriter` and optional `THEIAprovenance` keys. Orientation is top-left, channels
remain RGBA, sRGB applies to RGB, alpha is straight/opaque. Swizzles, unknown
keys and other colour descriptors are rejected. Decoded texels borrow immutable
encoded bytes; there is no decompression/allocation/native runtime dependency.
The encoded budget excludes Vulkan allocation alignment and other GPU resources.

`tools/cook_texture.py` takes bounded schema-1 JSON containing integer width,
height and row-major `rgba` arrays; alpha must be 255. Pinned KTX-Software 4.4.2
writes raw RGBA8 sRGB/BT709 data and validates the packaged result. The packager
adds namespaced provenance and relocates the sole level without rewriting its
DFD/texels. Exact source pixels are compared before publication. Provenance
records source/tool/library/compiler/cooker hashes and the recipe. Same tools
and input produce identical bytes. This is a synthetic raw-texture source gate;
glTF image decoding and source material-to-texture assignment remain separate.

KTX core is Apache-2.0 with third-party licences recorded in its source release.
The qualification build disables `KTX_FEATURE_ETC_UNPACK`: its separately
licensed Ericsson decoder is not compiled. Tools and sources stay under ignored
`build/`; no native library or source is shipped in Rust packages. The pinned
source hash and executable/library evidence are recorded in
[texture qualification](validation/M4_TEXTURES.md). GPU upload/sampling is not
established by this CPU contract.

The subsequent fixed-asset `--texture-test <ktx2> --uv-test <uv> --materials <mat>`
qualification uploads one RGBA8_SRGB sampled image and applies it to all factors.
The two sidecars independently bind exact geometry; the texture is explicitly
supplied, not resolved from an authored material dependency. Cell/fallback
combinations are rejected. One nearest-repeat sampler clamps mip selection to
zero; the Slang shader uses explicit SampleLevel at zero. This qualifies sRGB
RGB decoding before multiplication by linear unlit factors, source top-left
orientation and wrapping. It does not qualify derivatives, filtering/mips,
alpha, multiple textures, authored glTF images or virtual textures.

`--texture-budget` caps actual Vulkan image memory requirements (default 4 MiB)
before allocation. Staging has its own 4 MiB cap. Query image format/dimensions,
upload with a dedicated fence, transition TRANSFER_DST to SHADER_READ_ONLY with
TRANSFER_WRITE -> FRAGMENT sampled-read synchronization. Successful upload uses
no device/queue idle wait. Error guards retire submitted work before staging or
image destruction. Renderer retains image/sampler across swapchain generations
and retires frame/descriptor users before teardown. Binding 7 is the sampled
image, binding 8 its sampler; UV/material storage bindings remain 6/5.

### Immutable fixed textured bundle

THEIABND v1 is exactly 176 bytes: eight-byte magic, u32 version 1, u32 role count
4, then four 40-byte records (SHA-256/u64 encoded length), ordered geometry,
material factors, UVs, texture. Full-file SHA-256 names the manifest. Lengths
respect each reader's header minimum and existing encoded budget; duplicate
hashes, extra/truncated records and unknown versions/counts fail. This format
does not change THEIACEL or other asset formats.

`Bundle::decode` checks manifest structure; `Bundle::validate` requires all four
exact child lengths/hashes, decodes every container, validates material/UV binding
to geometry, and requires cooked texture provenance. Validation returns owned
decoded mesh/material/UV values and a borrowed immutable texture view. It does
not reserve storage or publish GPU resources. Reader budgets exclude aggregate
decoded memory, I/O buffers and Vulkan allocations.

`tools/package_texture_bundle.py` packages existing hash-named, colocated files;
it checks filename/role/size, while Rust performs semantic admission. No implicit
copying or overwrite of child assets occurs. `--bundle <manifest>` validates the
complete set before Vulkan initialization and derives sibling paths from fixed
hash/extension roles. Explicit asset/cell/fallback/material/UV/texture paths are
mutually exclusive with bundle selection. Existing loaders recheck immutable
bytes before upload; admission does not make later file mutation trusted.

The viewer retains the admitted fixed resource set through swapchain recreation
using preceding upload/frame retirement contracts. This is one geometry and one
shared resident texture, not a general dependency DAG, atomic multi-resource
streaming publication, budget reservation/deduplication, world instance database
or per-material image assignment. Missing children fail; no silent fallback.

### Complete-bundle CPU residency

`bundle_residency::Residency` owns one current complete set and tracks unique
encoded resource owners by content hash. `stage(manifest, expected_id, roots,
leaves)` semantically admits the entire fixed graph before checking a complete
reservation. Version 1 accepts zero extra leaves; version 2 requires every
canonical table leaf. The reservation includes the manifest and all roots and
leaves, deduplicated across profiles, stages, current sets and retained readers.
Failures leave current publication and live-byte accounting unchanged. Dropping
an opaque staged token cancels its reservation. Foreign store tokens fail.

`publish(Some(token))` atomically replaces the CPU set with a checked epoch;
`publish(None)` publishes an empty set. Snapshots retain all their immutable
bytes and can safely outlive the store. Releasing the last owner permits budget
reuse. This is a unique encoded-byte budget, excluding caller-owned inputs,
decoded admission temporaries, Arc/map overhead and Vulkan physical allocations.
The API owns no I/O worker, GPU fences or descriptors, and makes no GPU
publication claim. Geometry-only THEIACEL residency remains separate.

### Indexed bundle and multiple authored images

THEIABND v2 retains the 176-byte four-root layout, replacing the final KTX2
root with a THEIATEX texture table (88 bytes..1 MiB). Version 1 is unchanged.
`validate_table_roots` checks all roots and exact geometry/material/UV/table
bindings; callers must additionally run `table.validate_textures` against every
child to admit the complete set. The bundle example and viewer do both before
GPU initialization. `package_texture_bundle.py --texture-table` selects v2;
`--texture` selects v1. Explicit render modes and individual resource paths
cannot be combined with `--bundle`. No arbitrary dependency graph is admitted.

Opt-in `--unlit-textures` emits normalized schema 5: a `textures` array and one
`texture_materials` image index per dense material. Each source image retains
schema-4 PNG/sampler restrictions. At most 16 distinct source texture bindings
are accepted; exact encoded PNG SHA-256 deduplicates image aliases. Different
encodings of identical pixels are separate images. Source texture bindings
remain in provenance. Unique encoded PNG bytes plus source buffers fit 64 MiB;
normalized JSON output also fits 64 MiB. Every image must be referenced.
Cooking validates mappings/digests, decodes each unique PNG offline, sorts KTX2
hashes through THEIATEX, and emits a v2 bundle. Failed cooking may leave cached
immutable children but publishes no usable new bundle; filesystem transaction
or crash durability is not claimed. Renderer execution currently qualifies one
and two images. Residency remains fixed for the lifetime of the viewer.

### Bounded shared authored PNG import

Opt-in glTF `--unlit-texture` produces normalized schema 4, retaining geometry,
UV and opaque double-sided unlit factor restrictions. Every selected material
must reference the same source texture, TEXCOORD_0, explicit nearest min/mag
filters, repeat S/T, and no texture/info/sampler/image extensions. This gives
the fixed bundle's single texture an authored binding rather than an arbitrary
viewer argument. Other source texture indices, missing base-colour textures,
texture transforms and other sampler profiles fail.

Selected image must be an unescaped relative external PNG confined to the
source directory, including canonical symlink resolution. Embedded image URIs/
buffer views, network/absolute/traversal paths and other MIME types fail. Encoded
PNG is <=4 MiB, dimensions 1..1024, noninterlaced RGBA8; aggregate buffers plus
image are <=64 MiB. Rust checks the bounded envelope and preserves encoded
source bytes/hash; full PNG decoding/CRC/opaque-alpha validation happens through
the pinned offline KTX tool. These are encoded limits, not process RSS limits.

Normalized `texture` records include PNG base64/digest/dimensions, source image/
texture indices and fixed sampler policy; `texture_materials` has one zero index
per dense factor. Cooker checks exact source/hash/dimensions and every mapping,
then produces KTX2 and the existing four-role bundle. Existing binary asset
versions remain unchanged. PNG decoding happens offline and its recipe/tool/
library/source digests enter texture provenance; the geometry also records its
authored image dependency. No runtime PNG/glTF/native library is introduced.

The general PNG producer verifies decoded opacity/dimensions and reports
`texels_verified=false`: arbitrary compressed source bytes are not a direct raw
texel oracle. Qualification independently compares this synthetic fixture's
known 16 RGBA bytes to native output, then compares GPU texels/pixel sampling.
Raw-JSON cooking still compares every source texel directly. This profile does
not qualify arbitrary third-party authoring exports, multiple per-material
textures, mips/derivatives/filtering, alpha or dependency streaming.

### Dense resident material texture table

THEIATEX v1 has an 88-byte header: magic, u32 version 1, u32 metadata length,
u32 material count, u32 texture count, exact THEIAMAT SHA-256, payload SHA-256.
Payload is UTF-8 provenance padded to four bytes, hash-sorted unique 40-byte
texture dependency records (SHA-256/u64 length), then one u32 texture index per
dense material. Full-file hash names `<hash>.theia-textures`. Budgets: table
1 MiB, metadata 64 KiB, 1..256 materials matching the decoded material page,
1..16 textures each 104 bytes..4 MiB. Every dependency must be referenced.

Reader verifies checked exact lengths, both digests, material binding, canonical
padding/UTF-8, texture order/count/lengths and mappings. Complete child admission
requires every exact texture length/hash, bounded KTX2 semantics, opaque texels
and cooked provenance. Borrowed texture views retain immutable encoded owners.
This is CPU admission, not GPU descriptor publication/residency accounting.
Aggregate encoded texture bytes can approach 64 MiB; decoded CPU ownership and
physical GPU allocation budgets remain separate.

`tools/cook_texture_table.py` accepts schema-1 assignments with exact
`material_sha256`, `textures` file paths and `material_textures` source indices.
It sorts dependency hashes and remaps every material index, preserving physical
choices independently of source path order. Source/cooker/recipe hashes enter
provenance; a changed source ordering changes the full content address even when
canonical mapping bytes match. The cooker checks source structure/filenames;
Rust validates full child semantics. Sampling stays opaque sRGB, nearest repeat,
mip zero. The existing single-texture bundle/shader formats remain unchanged.

`--texture-table <table> --asset <mesh> --materials <factors> --uv-test <uv>`
now admits the entire fixed set before Vulkan initialization and uses an indexed
GPU path. Single-texture/cell/fallback/bundle combinations fail explicitly.
Immutable loaders recheck bytes before uploads, so later file mutation is not
trusted. Device sampled-image descriptor limits include the four existing target
slots plus the table images; image dimensions/format and table storage range
are queried. CPU profile allows <=16 textures subject to device limits; executed
GPU qualification covers counts one/two, including three-material reuse.

Resident images retain the dedicated-fence upload/image transition/ownership
contract. `--texture-budget` is aggregate actual image allocation bytes for this
profile; each new image must fit the remaining capacity before allocation. Table
storage has a separate 1 MiB allocation cap; coherent staging retains its 4 MiB
cap. These limits exclude samplers, descriptors, geometry and render targets.
Partial failure retires upload work and releases already uploaded images.

Fragment binding 7 is the immutable sampled-image descriptor array, binding 8
the fixed nearest-repeat sampler, binding 9 the raw table buffer. The shader
looks up cooked primitive -> material -> dense texture index, applies explicit
NonUniformResourceIndex, samples level zero and multiplies the linear unlit
factor. There is no per-frame host material lookup or descriptor sorting.
Recreation/teardown retires frame/descriptor users before table/image owners.
This does not yet import authored multiple PNGs, publish asynchronous resource
sets, or qualify mip/derivative/filtering/virtual textures.

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

### Optional shared primitive interface qualification

`compile_render_shaders.py --shared-primitive-interface --glslang <compiler>
--output-dir build/shaders-shared` substitutes only identity mesh/fragment
modules. Slang mesh output uses 64 shared vertices/64 primitive records, each
primitive carrying uint2 identity at Location 0. A specifically scoped GLSL
fragment fallback consumes flat `perprimitiveEXT` input at that location; all
compute/task/material resolve and other shaders remain Slang. Both modules must
show PerPrimitiveEXT at Location 0 and no Geometry capability before publication.
Use `--material-test --shader-dir build/shaders-shared` for runtime qualification.
Default compiler invocation and expanded Slang path are unchanged. The lab's
conservative 192-vertex/8192-byte admission remains, even for this alternate pair.
This is not a finalized minimum GPU profile, storage change or performance result.

### THEIAMAT v1 opaque unlit sidecar (CPU/offline prototype)

This independent, content-addressed file leaves THEIAMES/THEIACEL v1 unchanged.
Its filename is full-file SHA-256 plus `.theia-material`. Little-endian header:

| Offset | Type | Meaning |
|---:|---|---|
| 0 | bytes[8] | THEIAMAT magic |
| 8 | u32 | schema 1 |
| 12 | u32 | provenance UTF-8 byte length |
| 16 | u32 | material count, 1..256 |
| 20 | u32 | bound meshlet count, 1..4096 |
| 24 | u32 | total decoded triangle count |
| 28 | u32 | payload byte length |
| 32 | bytes[32] | exact THEIAMES full-file content hash |
| 64 | bytes[32] | payload SHA-256 |

Payload: UTF-8 provenance plus zero padding to four bytes; material float4 factors;
`meshlet_count + 1` u32 triangle-prefix offsets; one u32 material index per triangle,
in meshlet/local-triangle order. Prefixes must exactly equal cumulative counts
from fully decoded bound geometry, starting zero/ending total. Factors are finite
linear RGBA in [0,1], alpha exactly 1. No textures, vertex colors or lighting;
opaque unlit output is the selected factor, following the bounded factor-only
semantics of [KHR_materials_unlit](https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_unlit/README.md).
This is not glTF import support or general material-system approval.

Reader enforces a caller byte budget (default 4 MiB), 1 MiB provenance limit,
checked lengths, version/hash/UTF-8/padding/reference checks and exact geometry
binding. Local triangle lookup returns None outside each meshlet's interval.
Materials may be shared and assignment order need not match material index order.

`tools/cook_material_page.py --mesh <hash.theia-mesh> --assignments <JSON>` reads
schema 1 JSON with `geometry_sha256`, `base_colors` and `triangle_materials`.
Assignments explicitly target this exact cooked file; they are not source glTF
primitive IDs and must not be silently reused after recooking. The cooker records
source and script hashes, shades only opaque linear factors, and has no native
runtime dependency. Rust `--example material` independently validates its output.
The cell loader currently accepts geometry pages only; material residency,
dependency publication, GPU binding and general authoring conversion remain open.

### GPU opaque factor resolve

`--asset <mesh> --materials <sidecar>` validates the sidecar against the selected
full mesh hash and uploads one immutable storage buffer. This implies the cooked
frustum/visibility path. It requires fixed geometry; `--cell` and `--fallback`
combinations fail explicitly because material dependency streaming is not yet
implemented. The buffer's actual allocation is capped at 4 MiB independently of
geometry allocation, and checked against device storage-buffer range limits.

Fragment binding 5 addresses material records, meshlet-prefix offsets and local
triangle assignments. Background ID zero retains the existing background color;
otherwise resolve returns the authored opaque linear factor directly. This
factor-only shader adds no lighting, sampled texture, world-position modulation
or authored UV. Startup transfer publishes fragment reads; the dedicated upload
fence retires staging. The renderer owns the material buffer until graphics
swapchain targets/descriptors are retired, including construction-error cleanup.
It performs no runtime host triangle/material lookup or readback.

`--authored-reference <mesh material x y width height view>` returns independent
CPU identity/depth/color JSON. Capture uses `THEIA_CAPTURE_MATERIAL_PAGE` and
checks immutable GPU bytes plus an interior sample per rasterized identity,
falling back to an edge pixel for tiny triangles, alongside the 81-point grid.
These readbacks are qualification tooling only.

### Source glTF opaque unlit factor normalization

Opt-in normalizer `--unlit-factors` emits offline normalized schema 2 with source-
order triangle assignments and dense base-color factors. Every selected primitive
requires explicit KHR_materials_unlit, doubleSided true, OPAQUE alpha mode and
factor alpha 1; base-color textures/other attributes/extensions fail. Source
material indices/ranges are provenance only; reuse maps to one dense factor.
Reflection changes winding, preserving source triangle material assignment.
No THEIAMES/THEIAMAT/THEIACEL binary version changes or runtime parser dependency.

The cooker compares complete winding-preserving cyclic triangle multisets,
then maps source assignments into meshlet/local-triangle order. Duplicate indexed
triangles with distinct material assignments fail as ambiguous; same-material
duplicates retain coverage. Geometry metadata records normalized source hash,
normalizer source/manifest/lock/binary/compiler configuration and cooker/remapper
source hashes. The material sidecar binds the exact resulting geometry hash.
Source material edits consequently affect geometry provenance/content identity;
independent geometry/material deduplication is not implied by this prototype.
[Exact limits and workflow](../tools/theia-gltf-import/README.md).

### THEIAUVS v1 dense TEXCOORD_0 page

Independent content-addressed `.theia-uv` file, unchanged binary geometry/material/
cell formats. All fields little-endian:

| Offset | Type | Meaning |
|---:|---|---|
| 0 | bytes[8] | THEIAUVS magic |
| 8 | u32 | schema 1 |
| 12 | u32 | provenance UTF-8 byte length |
| 16 | u32 | vertex count, exactly bound geometry positions, 1..65536 |
| 20 | u32 | payload length |
| 24 | bytes[32] | exact full THEIAMES hash |
| 56 | bytes[32] | payload SHA-256 |

Payload is provenance/zero padding to four bytes, then dense finite float2 UVs in
unchanged global vertex order. Default byte budget 4 MiB; metadata limit 1 MiB.
Reader checks counts, checked lengths, version, UTF-8/padding, digests, binding
and finite coordinates. Negative/repeated coordinates are valid data; this is
not a normalized [0,1] promise or a sampler wrapping policy.

Normalizer `--unlit-uv` requires explicit supported unlit factors and float2
TEXCOORD_0 with POSITION-matching count on every selected primitive. Normalized
schema 3 adds `texcoords` alongside schema 2 factors/triangle assignments. Source
UVs are not transformed/flipped with world axes or reflected nodes; coincident
positions with distinct UVs remain separate rows. Integer/normalized UVs, extra
sets/attributes, base-color textures and other extensions remain unsupported.
Native coverage checks preserve global index identity before publishing the UV
page; the sidecar binds the full resulting geometry hash. No position welding or
LOD UV/seam correctness is implied. Source convention follows the
[glTF attribute contract](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#meshes-overview).

`--uv-test <page>` on a fixed `--asset` uses immutable fragment binding 6 plus
binding 4 geometry and 64-byte camera push constants. It resolves identity to
three global vertices, reconstructs UVs with screen barycentrics divided by clip
W, checks interpolated depth, and displays diagnostic
`(clamp(.4*u+.1), clamp(.4*v+.1), .5)`. The diagnostic loader limits magnitudes to
64; finite UVs outside that lab range remain valid reader data. Geometry+UV
binding, upload fence, Transfer->Fragment storage publication, physical 4 MiB
allocation cap and swapchain-before-buffer retirement are checked. Cell/fallback/
material combinations fail explicitly for this diagnostic mode. No authored
texture sampling, derivatives, mip LOD, sampler policy or dependency streaming.

`--uv-reference <mesh uv x y width height view>` independently returns CPU UV,
identity/depth/color JSON. Capture `THEIA_CAPTURE_UV_PAGE` verifies actual GPU
UV bytes and the same grid/all-visible-triangle probes as authored factors.
Color checks quantify encoded UV agreement (two sRGB byte levels), not a direct
floating-point UV target accuracy measurement.
