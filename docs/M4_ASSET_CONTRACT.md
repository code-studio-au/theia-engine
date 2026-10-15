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

### Independent complete-bundle CPU world ownership

`bundle_residency::WorldResidency` reuses complete-graph admission and one shared
unique encoded-byte pool across several chunks. Application-supplied `u64` chunk
IDs are separate from bundle/resource hashes: different chunks can reference the
same immutable bundle without duplicating encoded owners. `new(bytes, slots)`
requires 1..4096 chunk slots, independently of the byte cap. Staged reservations,
current chunks and retained world/chunk snapshots all retain charged resources.
Caller-owned I/O buffers, decoded temporaries, Arc/map overhead and GPU allocation
remain outside the byte charge.

`publish(additions, removals)` atomically adds/replaces complete named chunks and
removes existing chunks. Replacements need no explicit removal. Duplicate IDs,
add/remove conflicts, unknown removals, foreign staging, slot overflow, empty or
oversized transactions and epoch exhaustion fail before changing current state.
Failure consumes supplied tokens; their reservations release at last-owner drop.
A successful transaction increments the world epoch once; unchanged chunk epochs
retain their last publication value. Snapshot chunk iteration is ascending ID.

This is a CPU lifetime prerequisite, not a world file format, GPU renderer or
streaming scheduler. It performs no I/O, automatic eviction, camera demand or GPU
completion inference. The mixed benchmark still renders its single resident
whole-world bundle. Next connect bounded spatial demand and fence-gated independent
GPU chunk publication. [Qualification](validation/M4_WORLD_BUNDLE_RESIDENCY.md).

### Bounded outdoor chunk-distance demand oracle

`world_demand::Catalog` admits 1..4096 unique chunk IDs with finite inclusive XY
bounds enclosing their full geometry. `plan(eye_xy, actual_resident_ids, policy)`
selects by horizontal point-to-rectangle distance. Absent chunks enter at the load
radius; actual residents remain eligible through a larger/equal retention radius.
Both boundaries are inclusive. Input bounds may overlap. This outdoor 2.5D
prototype ignores altitude and view direction; it is not frustum/occlusion culling
or a cave/interior demand policy. Finite f32 inputs widen before arithmetic to
avoid distance overflow.

Eligible chunks sort nearest first, then ascending ID; `slots` (1..4096) limits
the desired set. Slot pressure may evict an otherwise retention-eligible chunk;
the retention band does not pin owners. Plans expose priority-ordered desired,
load and deferred IDs plus ascending removals. Unknown/duplicate resident IDs,
invalid bounds, nonfinite queries/radii, negative load radius, retention below
load radius and unsupported capacity fail explicitly. Planning is stateless:
failed requests must not be reported as actual residents on the next query.

The policy is an experimental CPU oracle and I/O prerequisite, not an approved
production streaming policy. It reserves no bytes, starts no requests, evicts
nothing and mutates no publication. A consumer must separately track pending work,
protect fallback coverage, reserve coexistence capacity and prove upload/frame
completion before acting. The offline complete-bundle example executes only CPU
transactions against a sufficient shared byte budget.
[Evidence](validation/M4_WORLD_DEMAND.md).

### Isolated independent GPU chunk-upload ownership probe

Render Lab's `--world-upload-test <scene.json>` uses the four checked quadrant
bundles in a bounded (1 MiB) offline scene manifest. It reuses existing complete
v2 admission and buffer/image upload primitives, and the new shared CPU world
owner. Each GPU chunk has independent buffer/image owners; encoded resource
sharing does not imply GPU allocation sharing. The probe performs synchronous
uploads with one pending complete set at a time, retiring every transfer fence
before exposing a ready set. Only CPU metadata publication follows; no descriptors
or graphics work refer to these chunks. Its three presented frames are clear-only.

Physical caps count all completed active, ready and retained-reader buffer/image
allocations plus the local pending reservation; images also have an independent
cap. Staging, queries, targets, driver and container overhead are excluded.
Cancellation and partial allocation failure retire submitted transfers before
releasing source reservations. Removed chunks remain charged while a reader owns
them. This private qualification owner is not a frame-loop streamer or a public
GPU completion API. A rendering integration must additionally prove frame and
descriptor retirement, fallback coverage and shared identity/depth/resolve.
[Evidence](validation/M4_WORLD_UPLOAD_OWNERSHIP.md).

### Two resident geometry chunks: shared visibility/depth

Render Lab's isolated `--world-pair-test <mesh0> <mesh1>` draws two admitted
geometry pages together, each with its own GPU frustum work and indirect task/mesh
counts. Both draws share one identity/depth pass; a diagnostic resolve validates
both geometry pages through separate static storage bindings (4 and 12). No
runtime descriptor array or new buffer-indexing capability is needed. Each page
retains the current <=32-meshlet bound and fixed 256-byte work arena. The existing
camera/mesh profile and two fragment storage bindings are checked against limits.

Frame-local identity bit 30 selects the second input, with low bits meshlet+1 and
zero background. It is separate from the bit-31 visual-proxy namespace and is not
an asset-format or persistent-handle change. Work remains GPU-generated; the CPU
records two fixed chunk draws, never one per meshlet/entity. Default and optional
shared-primitive primary pipelines can coexist with the second chunk's separately
compiled per-vertex interface. No new language fallback is introduced.

This slice uses diagnostic chunk/face tint and a fixed camera rig; authored
materials/UV/textures, LOD, benchmark/proxy modes and streaming inputs are rejected.
It is separate from complete-bundle upload ownership and CPU world demand. Full
bundle composition, protected fallback coverage and fence-gated independent
publication remain to connect.
[Evidence](validation/M4_WORLD_PAIR_RENDERING.md).

### Two complete resident textured chunks

`--world-bundle-pair <v2 bundle0> <v2 bundle1>` extends the isolated pair profile
with both complete factor/UV/table/image graphs. It reuses the qualified complete
upload helper and shared `WorldResidency` encoded pool. Both graphs are admitted
and reserved, all transfer fences retire, then one CPU world transaction publishes
the two named complete chunks before creating graphics descriptors. This is fixed
startup residency, not background or camera-driven publication.

The renderer retains each chunk's own geometry, factors, UVs, texture table and
sampled images. Independent static storage bindings 4/5/6/9 and 12/13/14/15 plus
image arrays 7/16 and samplers 8/17 select the correct graph from the frame-local
bit-30 identity tag. Two indirect task/mesh draws still share identity/depth;
UV reconstruction, material/table mapping and nearest mip-zero sampling run in
one resolve. Each graph has <=32 meshlets and <=16 opaque images. Device limits
must support eight fragment storage buffers, combined image counts plus four
existing target/material images and three samplers; no storage-array indexing
feature or new shader-language fallback is introduced.

`--bundle-byte-budget` caps unique live encoded sources; `--bundle-gpu-budget`
caps aggregate geometry/factor/UV/table and sampled-image physical allocations;
`--texture-budget` additionally caps aggregate image allocations. Caps apply during
initial construction, including coexisting sets and submitted pending transfers.
GPU owners are independent even when CPU bytes share hashes. Work arenas, staging,
targets, descriptors, samplers, queries and driver overhead are excluded from the
bundle physical charge. No partially uploaded set is displayed on construction
failure. Resident geometry can move directly into indirect work ownership without
a duplicate upload. Renderer teardown retires graphics before resource/CPU owners.

The geometry-only pair, normal cooked viewer and textured crowd benchmark remain
separate. This slice has no crowd composition, live chunk replacement, protected
fallback selection, geometry LOD or camera-driven residency.
[Qualification](validation/M4_WORLD_TEXTURED_PAIR.md).

### Independent complete-chunk replacement

`--world-bundle-pair <fixed initial> --world-stream-bundle <replacement>` keeps
presentation slot/chunk 0 resident and alternates slot 1's two distinct v2 graphs
every twenty presented frames. These are qualification slot IDs, not inferred
spatial region identities. One bounded local-file worker/channel and one pending
GPU set reuse complete-graph admission and the shared world CPU encoded pool.
The tested source variants preserve geometry/UVs/material slots while changing
palette and deduplicated image count (four to two and back).

The physical and image caps include both active chunks plus the pending complete
replacement, without GPU deduplication. CPU charges deduplicate all live current/
staged/retained encoded owners. Budget pressure drops partial transfers after
fence retirement and retains both current complete chunks without changing the
world epoch. Source/driver/unsupported-limit errors are fatal with orderly cleanup.
File reads are bounded in bytes, not deadline bounded; shutdown joins the worker.

The sole frame fence retires before polling; every replacement transfer fence
must be ready before publication. Presentation/device retirement and destruction
of the old graphics descriptor generation precede dropping slot 1's GPU owners.
A new complete generation can change image-array count. Only after construction
succeeds does the CPU world replace chunk 1; chunk 0's CPU epoch remains unchanged.
Generation-construction failure is fatal and has no rollback guarantee. This is
conservative retirement/rebuild, not general multi-frame allocation reuse or a
GPU upload/render overlap performance claim.

`--world-stream-report <json>` writes a diagnostic completion record, with at most
64 recent CPU publication events plus a dropped count. `current_cpu_bytes` is the
pool's current live charge, including an admitted pending reservation; it is not
only published bytes or process RSS. Peak complete-set physical/image fields
cover active plus fully submitted complete candidates; failed partial candidates
are excluded from those observations, while per-allocation remaining-cap checks
still prevent overshoot. No renderer scheduling reads GPU counts back.

Camera demand, spatial fallback coverage, four-chunk composition, proxies and the
timed crowd route remain separate. [Evidence](validation/M4_WORLD_CHUNK_STREAMING.md).

### Optional nonuniform storage-descriptor array compute gate

`--world-descriptor-test` is a separate three-frame clear-only qualification. It
requires and enables core `shaderStorageBufferArrayDynamicIndexing` and Vulkan
1.2 `shaderStorageBufferArrayNonUniformIndexing` only for that opt-in test; normal
profiles retain their prior feature requests. Existing runtime-descriptor-array
support plus queried compute/storage limits are checked before device creation.

One compute workgroup has 64 divergent invocations select among four independently
uploaded 16-byte buffers and vary the word offset. A fixed transposed literal
pattern checks all 64 output values after explicit shader-to-host publication,
fence retirement and coherent mapping. The sentinel-to-compute barrier covers
both read/write access declared by `RWByteAddressBuffer`. RAII retires submitted
commands before freeing pipelines/descriptors/buffers on success or failure.

This is a correctness gate for future bounded multi-chunk binding, not a new
production GPU requirement or binding-policy approval. Fragment/mesh integration,
four-world-chunk rendering and performance remain separate qualifications.
[Evidence](validation/M4_STORAGE_DESCRIPTOR_ARRAYS.md).

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

### Pending sampled-image ownership

Renderer `texture::Upload` now separates begin/ready/finish. Begin records and
submits the copy and scoped image transitions without waiting. A pending owner
retains its command pool, dedicated fence, coherent staging allocation and image;
cancellation waits before releasing submitted resources. Finish retires the
fence and creates the nearest-repeat sampler. Indexed startup submits the entire
image set before finishing any image, with aggregate image-budget checks across
pending images. Startup waits before target/descriptor construction. This is
not yet asynchronous whole-bundle frame-time publication. Staging allocations
remain separately counted and individually bounded; images retain their
aggregate physical byte budget.

### Complete indexed-bundle streaming qualification

`--bundle <initial v2> --stream-bundle <replacement v2>` alternates fixed
complete graphs every twenty presented frames with one I/O worker/channel slot
and one pending GPU set. Each source root/leaf read is bounded by its admitted
role length and format cap. Inputs and decoded admission temporaries are outside
the CPU residency charge; file-read duration is not deadline bounded. Shutdown
joins the local-file worker. There is no arbitrary dependency DAG or demand queue.

`--bundle-byte-budget` (default 64 MiB) caps unique retained encoded owners,
including manifests. `--bundle-gpu-budget` (default 32 MiB) caps all active plus
pending geometry/factor/UV/table buffer and sampled-image physical allocations.
`--texture-budget` additionally caps active plus pending image allocations.
GPU owners are not deduplicated across sets; shared content is deduplicated only
in CPU residency. Work buffers, targets, descriptors, samplers, command pools
and staging allocations are excluded from these GPU charges. Staging bytes
are logged separately; uploads retain preceding per-resource staging bounds.

Every buffer/image uses a dedicated transfer fence. Pending transfers drop
before CPU reservations; partial budget failure retires submitted work and
keeps the current complete set. Publication requires all upload fences ready and
the sole frame fence retired. Graphics/presentation are conservatively retired
with device idle; the complete graphics generation is rebuilt, permitting the
sampled-image array count to change. No frame sees a partially replaced set.
Only then does the CPU epoch change. This rebuild is a qualification approach,
not an upload/render-overlap or production latency claim. A source/driver/shader
error terminates the viewer with orderly teardown; only budget pressure defers
and retries. Target-construction failure is fatal and has no rollback guarantee.

The opt-in `--benchmark-cooked --bundle <v2>` viewer now applies the shared
three-phase benchmark camera and timing recorder to this same bounded profile.
Optional `--stream-bundle` retains the twenty-presented-frame replacement schedule
and existing ownership/budgets. It renders one complete set at a time; it does not
mix procedural crowd records or create simultaneous world residency. Reports
distinguish this workload, record budget requests and publication/deferral totals,
and exclude pre-frame transfers from GPU query spans. Frame allocation traces
exclude cooked bundle owners, whose physical budgets remain separate. No binary
contract or authority policy changed. [Workflow](SCENE_BENCHMARK.md#cooked-bundle-route-and-timing-observations).

An optional offline whole-world representation combines the static village with
a coarser continuous ground in one unchanged indexed bundle, within the same
32-meshlet admission profile. An exact continuous rectangular-heightfield gate
bounds vertical deviation from the fine quadrant ground at 0.12 metres. Equal
position/UV rows weld only within a material; authored prop triangles and seams
are preserved. This is a fixed resident test representation, not dynamic LOD,
independent world-chunk publication or general 3D simplification qualification.
[Scope](SCENE_BENCHMARK.md#complete-resident-cooked-world).

The cooked benchmark's optional debug resolve additionally binds its 256-byte
work buffer at fragment storage binding 10. Camera rows remain bytes 0..64;
completed timing/phase values occupy 64..96 and active bundle allocation/texture/
publication metadata occupy 96..112. Queried device limits must support five
fragment storage buffers and the 112-byte push range. The disabled path retains
the original four storage bindings/64-byte range and does not load the HUD module.
Scoped work-buffer barriers include fragment count reads and reuse. Per-frame
`cooked` JSON metadata and CSV fields retain actual active allocation charges,
texture count and publication/deferral totals. This does not read GPU counts back
for scheduling, change any cooked file format or count pending resources as active.

One/two-image alternating sets and source/table/framebuffer consistency pass
local RX 7900 XTX validation/replay. Existing geometry-only streaming and fixed
render modes remain separate. General GPU allocation reuse, geometry arenas for
whole bundles, multi-frame retirement, fallback scheduling, derivative/mip/tile
residency and a streamed world database remain open.

### Continuous rectangular heightfield error gate

Offline `continuous_height_error.py` admits at most 2048 vertices/2048 triangles
per surface, finite represented positions and positive XY winding. Both surfaces
share the source position table. Each triangle lies in the common bounding
rectangle. Exact projected area must equal rectangle area; pairwise exact
intersection checks reject interior overlap and mismatched heights on shared
boundaries. Finite closed triangles with full area and no overlaps cover the
rectangle. This profile excludes caves, folds and arbitrary nonrectangular meshes.

Fractions represent the actual canonical f32 coordinates exactly. Convex triangle
clipping constructs the fine/coarse planar overlay. Height difference is affine
on every overlap polygon; the extrema of its absolute value occur at polygon
vertices. The maximum exact fraction is checked against the represented error
limit without an epsilon. Metadata retains numerator/denominator strings, an
upward-rounded finite float upper bound, overlay counts, profile and checker
source digest. An exact equality may produce a reported float bound one ULP
above the configured limit; admission uses the exact fraction. This is a
continuous vertical bound between encoded surfaces, not a 3D Hausdorff, normal,
silhouette, material or shader-arithmetic bound. It is producer qualification,
not authentication of arbitrary metadata or a runtime proof certificate.

`cook_meshlet_fixture.py --simplify-ratio` retains native LockBorder|ErrorAbsolute
and the existing independent sampled/boundary checks, and adds this gate before
output publication. The mesh binary schema is unchanged; LOD metadata changes
its content identity. Unsimplified position-only fixture hashes are preserved.
Authored material imports also hash their cooker source, so recooking them after
any cooker edit deliberately changes provenance-bound identities. Existing
immutable assets remain valid. Projected-error selection is still outstanding.

### Projected vertical-error CPU selection

`theia_assets::lod::select` takes a common finite f32 AABB enclosing both
continuous surfaces, a nonnegative represented f32 vertical-error bound, f32
clip matrix rows, nonzero extent and finite nonnegative pixel budget. Both
surfaces must share XY correspondence and differ only in Z; this is a caller
precondition derived from the qualified rectangular-heightfield producer.
Finite f32 products are exact in f64; subsequent sums and positive bound
operations round outward. The oracle guards the whole box against nonpositive
W and Vulkan Z=0/Z=W clip boundaries and chooses the fine baseline if crossed.

With minimum W `d`, maximum absolute clip X/Y `a`, vertical matrix coefficient
`c` and W's vertical coefficient `b`, projected NDC displacement is bounded by
`E * (abs(c) + a * abs(b) / d) / d`. Both endpoint denominators are at least `d`
because both surfaces lie inside the box. Pixel bounds scale by width/2 and
height/2. Coarse is allowed only when both bounds fit the budget; finite-input
numeric overflow retains fine. `error_f32` rounds a producer f64 upper bound
upward rather than losing its guarantee during conversion.

`--lod-reference <fine coarse width height view pixels>` checks hash-named
mesh containers, source links, triangle counts, boundary/profile metadata and
identical common position bits. It consumes the producer's declared bound and
explicitly reports `proof_recomputed=false`. Output ends with a JSON record
after asset diagnostics. Arbitrary metadata is not authenticated or proved.
This is CPU oracle selection only: no GPU page choice, shader-rounding guarantee,
LOD hysteresis, depth/disocclusion, silhouette/material error, general 3D
hierarchy or texture LOD contract is implied. Geometry streaming remains separate.

### GPU resident heightfield LOD selection

`--lod-test <fine coarse>` uses the preceding source-link/common-position/profile
admission, additionally limiting positions to 2048 and meshlets to 32 per page.
Both immutable buffers upload once to fixed storage bindings 1/2.
`--lod-budget` (default 4 MiB) caps their combined actual physical allocation
bytes; work, staging, targets, descriptors and command pools remain separate.
Failure to admit/upload both pages terminates initialization safely; this path
is independent of geometry/bundle streaming and has no runtime fallback loader.

`--lod-pixels` (default 6) supplies the visual projected-error budget. Compute
uses 112 push bytes (camera rows/common bounds/error/budget/extent); the mesh
module is separately compiled with a 64-byte camera block. Three storage
descriptors, push sizes and both buffer ranges are device-checked. Compute
writes selection at work byte 44, float2 pixel bounds at 48, reason at 56 and
minimum W at 60, then culls the selected page and emits counts/masks/compact IDs.
Selected page meshlet count is at 32. A group memory barrier publishes lane-0
selection before per-meshlet loads; existing scoped compute-to-indirect/task/mesh
barriers publish all work to raster consumers. The CPU passes one configuration,
not a per-object draw list or count readback.

The bounded f32 policy admits matrix/common-bound magnitudes <=64, extent
1..16384 and errors exactly zero or 1e-6..1. It adds absolute/relative arithmetic
margins, keeps fine at positive-W/near/far boundaries and unsupported profiles,
and chooses coarse only when both padded pixel bounds fit. Reasons: 0 coarse,
1 clip boundary, 2 pixel budget, 3 unsupported numeric profile. GPU choices may
be more conservative than the mathematical CPU oracle. Executed cases verify
GPU bounds are not below that oracle and never choose CPU-forbidden coarse.
This is qualification of the tested numerical profile; it is not a proof of
all shader arithmetic, rasterization error or cross-vendor behavior.

The mesh stage uses the same GPU selector for header/ref/triangle loads; task
counts and culling IDs belong to that page. No shader array indexing feature is
added: each load branches between two fixed descriptor bindings. Existing
single-page camera/cull/identity/material paths and headless test remain intact.
General hierarchy/page demand, LOD hysteresis/morphing, 3D/silhouette/material
errors, allocation reuse and multi-frame retirement remain open.

### Bounded CPU LOD hysteresis

`lod::select_hysteresis` uses caller-owned prior level for one view/page pair.
An entry fraction [0,1] reduces the fine-to-coarse threshold; coarse-to-fine keeps
the original hard budget and clip guards. Both entry arithmetic steps round inward.
Fraction zero preserves `select` exactly. The caller starts/resets to Fine when
context changes. This is independent of GPU history and streamed page demand.
[Contract qualification](validation/M4_LOD_HYSTERESIS_CONTRACT.md).

### GPU resident LOD history

The bounded two-page renderer now supports `--lod-hysteresis` (default zero) and
retains prior level at work byte 44. First dispatch resets fine before any history
load; resized views retain history with recomputed bounds. Compute reuse orders
storage reads/writes. Viewport Z/W carry entry fraction/reset in the existing
112-byte push block; mesh stays 64 bytes. Coarsening uses an inward-margin entry
threshold and refinement preserves hard limits/clip/profile guards. Reason 4 denotes
entry-threshold retention. Diagnostic prior/entry/hard/reset occupy bytes 16–28.
`--lod-hysteresis-test` drives eight budget phases with an explicit reset probe.
[GPU/history qualification](validation/M4_GPU_LOD_HYSTERESIS.md). General hierarchy,
page demand, morphing and 3D/material/silhouette errors remain open.

### Separate complete authored mip-chain profile

`texture_mips::TextureMipChain` accepts complete base-first indexed/smallest-first
physical RGBA8_SRGB KTX2 levels through 1x1, count 1..11, dimensions <=1024 and
default 4 MiB encoded budget. It shares strict DFD/metadata validation with the
unchanged single-mip runtime gate. Exact level sizes/offsets/opaque alpha are checked;
texels borrow immutable bytes. The offline `cook_texture_mips.py` qualifies explicitly
authored raw levels using pinned KTX 4.4.2 and verifies every byte, without filtering.
[Contract evidence](validation/M4_TEXTURE_MIP_CONTRACT.md). GPU adoption, mip selection,
derivatives, filtering and tiled/partial residency remain separate gates.

### GPU complete-chain fixed views

`--texture-mip-test <file level>` admits the separate complete authored chain before
Vulkan setup, rechecks immutable bytes and device mip/extent limits, then uploads
all levels with one staging allocation/transfer and full-range barriers. One image
view exposes the selected mip; the unchanged Slang fragment samples view-relative
level zero with nearest repeat. No per-frame host mip choice or runtime readback.
`--texture-mip-reference` provides a readonly CPU sampling oracle. Existing single-
mip/table/bundle paths stay separate. [GPU evidence](validation/M4_GPU_TEXTURE_MIPS.md).
Automatic mip choice, gradients/filtering and tiled/partial residency remain open.

### CPU perspective UV footprint

`texture_footprint::differential` computes rational UV interpolation and analytic
per-pixel gradients from represented f32 clip/UV data in f64. Bounds/near-far/W/
area/denominator guards define a deliberately limited profile. Wrapping occurs
after gradient evaluation. `choose_mip` uses maximum Jacobian-column length in
base texels, exact power-of-two thresholds and complete-chain clamping; it is an
isotropic prototype policy, not filtering quality. Render Lab's readonly
`--texture-footprint-reference` connects this CPU model to admitted scene data.
[Contract evidence](validation/M4_UV_FOOTPRINT_CONTRACT.md). GPU arithmetic/selection
and tiled/partial residency remain unqualified at this checkpoint.

### GPU analytic automatic mip selection

`--texture-auto-mips <file>` exposes the complete resident chain and selects an
explicit level using analytic per-primitive perspective UV gradients before wrap.
The bounded CPU footprint policy is implemented in Slang; unsupported gradients
sample mip zero. An optional fragment-storage diagnostic buffer qualifies GPU
outputs without runtime readback and has its own capability/budget checks.
`--texture-auto-reference` provides the readonly colour/mip oracle; the footprint
reference now reports supported gradients or explicit mip-zero fallback.
[GPU qualification](validation/M4_GPU_AUTOMATIC_MIPS.md). Filtering quality,
tiled/partial residency and full-chain streamed replacement remain open.

### CPU bilinear-repeat reference

`texture_filter::linear_repeat` models one opaque sRGB authored mip with ideal
four-tap weights and RGB decode before interpolation, top-left normalized
coordinates and half-texel centres. Inputs/alpha/ranges are bounded and checked.
This readonly CPU oracle does not generate mips or change renderer sampling.
[Qualification](validation/M4_LINEAR_TEXTURE_REFERENCE.md).

### Opt-in GPU within-mip linear sampling

`--texture-linear-filter` requires automatic-mip mode and queried RGBA8_SRGB
linear-sampling support. Min/mag filters become LINEAR; mip choice remains the
existing explicit integer policy with nearest mipmap mode. Default/fixed/table
paths stay nearest. `--texture-linear-reference` uses the CPU bilinear oracle
before linear factors. Patterned GPU replay and budget/cleanup checks qualify
this bounded path; no trilinear/mip-generation/production-quality claim.
[Evidence](validation/M4_GPU_LINEAR_FILTER.md).

### CPU protected mip-tail plan

`texture_residency::MipTail` borrows a complete admitted chain, validates public
level fields, and fits the finest contiguous coarse suffix through 1x1 to a dense
texel budget. Source footprint dimensions stay separate from resident dimensions;
selection clamps desired source levels to the resident suffix and returns relative
image levels. This is not physical allocation accounting, partial I/O or GPU
publication. [Contract qualification](validation/M4_MIP_TAIL_CONTRACT.md).

### GPU conventional protected mip tails

`--texture-tail-budget` with automatic mips fits a coarse suffix before Vulkan,
rechecks full immutable source data and uploads only resident levels. A separate
80-byte fragment push profile carries original dimensions/count and first source
level; footprint uses original dimensions, sampling subtracts the resident offset.
Actual image caps remain independent of dense texel budgets. Diagnostic reason 4
marks residency coarsening; unsupported gradients request source zero then clamp.
Tail records' byte 44 carries relative mip; nontail reserved zero remains intact.
[GPU qualification](validation/M4_GPU_MIP_TAIL.md).

The bilinear CPU oracle additionally supports a bounded per-weight quantization
interval using queried device fraction bits. Linear reference commands optionally
accept trailing `--filter-precision`; nearest/identity/depth/mip checks are preserved.
This models a diagnostic error envelope, not complete cross-vendor arithmetic.

### Native generated colour mip profile

`tools/cook_filtered_texture.py` accepts opaque power-of-two RGBA8 sRGB source JSON,
uses a temporary lossless PNG envelope and delegates iterative box/clamp generation
to pinned offline KTX 4.4.2. Independent checks preserve the base exactly and verify
per-step linear-light averages with <=1 encoded RGB byte error and opaque alpha.
The existing packager/strict Rust reader remain unchanged. This bounded recipe is
separate from explicit authored levels and is not a general filtering policy.
[Qualification](validation/M4_FILTERED_MIP_COOKING.md).

### Asynchronous conventional tail refinement

`--texture-refine-budget` adds one finer-suffix request to the initial automatic
tail. Background full-file I/O rechecks immutable source identity; owned transfer
readiness and conservative graphics-generation retirement gate image/sampler and
source-metadata publication. Actual active-plus-pending image requirements share
`--texture-budget`, separately from staging/dense caps. Budget deferral and changed
source rejection retain the protected coarse image. This is a single qualification
request, not tiled GPU feedback, retry scheduling or overlapped generation reuse.
[Evidence and limitations](validation/M4_ASYNC_MIP_REFINEMENT.md).

### Mixed resident benchmark integration

The mixed benchmark retains the immutable whole-world v2 bundle and adds a
separately owned GPU proxy arena. Two fixed indirect draws share visibility/depth;
resolve distinguishes cooked meshlets from proxies using a frame-local identity
high bit. This tag is not an asset-format change or a persistent/network entity ID.
The proxy arena is included in tracked DRAW allocation charges, independently of
active bundle allocation metadata. The first mixed slice requires resident content
and rejects bundle replacement; independent world-chunk residency is still open.
[Usage and limits](SCENE_BENCHMARK.md#mixed-resident-world-and-visual-proxies).
