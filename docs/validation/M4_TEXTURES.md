# M4 bounded texture qualification

## Offline KTX2 contract (2026-10-11)

This qualifies a synthetic raw source and CPU reader. GPU sampling, authored glTF
images, material dependencies, mipmaps, compression and virtual textures are not
implemented by this unit. M4 remains open.

Official KTX-Software 4.4.2 source archive:
<https://codeload.github.com/KhronosGroup/KTX-Software/tar.gz/refs/tags/v4.4.2>,
SHA-256 `9412cb45045a503005acd47d98f9e8b47154634a50b4df21e17a1dfa8971d323`.
Qualification uses CMake Release/GNU C++ 13.3.0 with tools ON, tests/docs OFF,
`KTX_FEATURE_ETC_UNPACK=OFF` and `KTX_GIT_VERSION_FULL=v4.4.2`. The version
override is required for the source archive without Git tags; an initial build
reported `exitcode-128-NOTFOUND` and was rejected by the producer. The official
prebuilt CLI was inspected initially but is not the qualified producer.

Core licence is Apache-2.0; the source includes third-party notices/licences.
The separately licensed Ericsson `external/etcdec/etcdec.cxx` is excluded: the
CMake cache and all compile commands are checked before cooking. No native
source/library is committed or linked to runtime. Source licence text and recipe
stay under `build/qualification/ktx-tool/` for this local qualification. See the
[upstream licence](https://github.com/KhronosGroup/KTX-Software/blob/v4.4.2/LICENSE.md)
and [KTX2 specification](https://registry.khronos.org/KTX/specs/2.0/ktxspec.v2.html).

Qualified executable SHA-256
`046ee04d58e22707216f7a6ad066781ed2516b5f07cb37a7fb0ac5d4a97c573b`;
libktx SHA-256
`009f8577d2312976afc6fc7eb855f3be7d7da89c930ef2b4fa09b4cc1e187dcf`.
Source-built `ktx --version` reports v4.4.2. Encoded tool/build digests are
provenance, not portable binary hashes expected from other compiler builds.

The committed two-by-two fixture uses four asymmetric synthetic opaque colours.
The tool writes R8G8B8A8_SRGB, BT709 primaries, top-left orientation and one mip.
The packager verifies all 16 raw texel bytes, preserves native DFD/image bytes,
adds `THEIAprovenance`, and validates the final KTX2 through the official CLI.
Validation exits successfully with expected warning 7010 for the custom key;
this is not claimed as warning-free KTX validation. Native diagnostics are sent
to stderr so the producer's stdout remains a JSON report.

Final cooked asset is 916 bytes, SHA-256
`9f865467383ceb97a8880ebf720560b57fba195c66abd60d60998878dd306674`.
Independent Rust example verifies hash filename, 2x2 dimensions, 16 opaque
RGBA8 sRGB texel bytes, orientation/DFD/ranges and cooked provenance. Repeated
native cooking with the same tools/input is byte-identical. The reader borrows
immutable texel bytes without a native dependency or decoded-image allocation.

Three new Rust tests bring the asset suite to 21. They cover exact official
fixture bytes, every truncation, budgets/hashes, unsupported dimensions/layout/
DFD/channels/supercompression, metadata ranges/padding/UTF-8/duplicates,
orientation/swizzles and opaque alpha. Two Python tests cover nine invalid
source profiles and malformed native layouts/oversized metadata while verifying
deterministic metadata relocation. All ten Python tests, Rust tests, formatting
and denied-warning Clippy pass. Runtime GPU code/shaders are unchanged.

Artifacts: `build/qualification/m4-texture-{cook.json,cook.log,reader.log,
tests.log}` and `ktx-tool/{configure.log,build.log,cli-build.log,source-build/}`.
CI configuration includes source build, native production and Rust inspection;
hosted CI and Windows native tool/GPU execution have not run. No push or
performance claim. Next qualification is bounded resident Vulkan upload/sampling
against the existing perspective UV reference.

## Resident Vulkan texture sampling (2026-10-11)

The same 916-byte KTX2 asset now feeds one resident Vulkan R8G8B8A8_SRGB image.
Fixed geometry/UV/factor paths are the preceding authored UV fixture and terrain
pairs, unchanged hashes. Physical-device image format/dimension limits are
queried. Actual image memory requirements are checked before allocation:
2x2/16 texel bytes requires 4,096 image bytes and 16 coherent staging bytes on
the RX 7900 XTX. The default physical cap is 4 MiB; an explicit 4,095-byte cap
fails after image creation without a Vulkan warning/error or leaked live object.

A dedicated upload fence protects staging/image ownership. Scoped Sync2 image
barriers transition UNDEFINED -> TRANSFER_DST -> SHADER_READ_ONLY and publish
transfer writes to fragment sampled reads. Successful upload has no queue/device
idle wait. Renderer owns image and nearest-repeat sampler across swapchain
recreation, retires frame/descriptor users first, and releases children before
the device. Missing resolve shader fails after uploads/target construction with
zero Vulkan diagnostics through cleanup.

Slang fragment resolve shares the qualified perspective UV reconstruction,
samples explicit mip zero and multiplies sampled linear RGB by the authored
linear unlit factor. Bindings 7/8 hold the image/sampler; 5/6 retain factors/UVs.
One supplied texture applies to all material assignments; no authored source
material-to-texture table exists yet. Other rendering/identity modes remain
available. UV and factor pages both validate exact geometry binding.

CPU nearest-repeat reference independently decodes sRGB with the standard
piecewise transfer function. A new analytic test checks known texel values,
top-left orientation, integer boundaries and negative/repeated coordinates.
Fourteen renderer tests, formatting and denied-warning Clippy pass. Both default
and shared-identity compiler builds compile/SPIR-V validate all 26 shader outputs.

Standalone synchronization-validation regression on RX 7900 XTX/RADV passes 15
paths, each 120 presented frames and ten swapchain generations with zero
diagnostics including teardown: imported texture, terrain texture, shared
texture, UV, authored factors, M3 visibility, indirect, clear, mesh, task,
spatial, asset, camera, frustum and diagnostic material cell. Separately recorded
failure paths reject the physical allocation cap and missing shader safely.

RenderDoc 1.46 capture validation remains disabled, separate from standalone
validation. Each target presents 180 frames. Captures verify exact GPU texture
texels, geometry, UV and factor bytes, GPU indirect work and visibility identity,
and compare CPU depth/sampled colour at 2e-6 depth/two RGB-byte tolerances:

| Texture case | Samples | Visible triangle probes |
| --- | ---: | ---: |
| Imported full | 85 | 4 |
| Imported empty | 81 | 0 |
| Imported narrow | 83 | 2 |
| Imported shared identity | 85 | 4 |
| Terrain | 593 | 512 |
| Terrain shared identity | 593 | 512 |

The original UV diagnostic replay passes 85 samples; original M3 replay passes
84 samples. Captures exercise reflected geometry and UVs outside [0,1] with
nearest repeat. Reports retain expected UV and actual identities; actual GPU
texel readback matches all source bytes, including row/channel ordering.

Artifacts: `build/qualification/m4-texture-gpu-{shaders.log,shared-shaders.log,
smoke.log,terrain-smoke.log,allocation.log,missing-shader.log}`,
`m4-texture-gpu-regression-*.log` and
`m4-texture-gpu-{full,empty,narrow,shared,terrain,terrain-shared,uv-regression,
m3-regression}/report.json` with captures/images. No hosted CI, push, benchmark
or Windows GPU claim. Next: bounded authored image import/material dependencies;
general mip/derivative/filtering, compression and virtual textures remain open.

## Immutable fixed dependency admission (2026-10-11)

THEIABND v1 records the existing geometry, material, UV and resident texture
content hashes/encoded lengths in four ordered roles. The 176-byte manifest
SHA-256 is `b664b76433f155aa558d3ad2c8f9e60269ebd826696c5a2d2dcde0b728f8e3c7`.
All child hashes are unchanged from the preceding UV/texture qualification.
Repeated packaging produces identical bytes. An independent Rust example admits
all four containers and both exact geometry bindings.

Three new Rust tests bring the asset suite to 24. They verify complete admission,
missing/swapped roles, all manifest truncations, unknown role/version/counts,
duplicate hashes/role budgets, and freshly hashed foreign-UV/non-opaque children.
Renderer's 14 tests and both affected packages' formatting/denied-warning
Clippy pass. No shader or GPU upload/resolve code changes for this unit.

`--bundle` validates all child bytes before Vulkan initialization, then uses the
existing immutable loaders/uploads. Five isolated malformed file sets (missing
texture, truncated material, corrupt texture, freshly hashed foreign UV,
invalid manifest role count) exit unsuccessfully before validation-layer/GPU
initialization. Existing assets are preserved; negative copies stay under
`build/qualification/m4-bundle-negative/`.

RX 7900 XTX standalone default/shared bundle WSI tests each present 120 frames
and ten swapchain generations with zero diagnostics through teardown. Separate
RenderDoc captures (capture validation disabled) each present 180 frames, verify
expected manifest dependencies and exact GPU geometry/material/UV/texture bytes,
work/identity/depth, and all four visible triangle probes plus 81 grid colour
samples. They use the preceding CPU perspective/sRGB/nearest-repeat oracle and
existing 2e-6 depth/two RGB-byte tolerances.

Artifacts: `build/qualification/m4-bundle-{package.json,reader.log,tests.log,
renderer-tests.log,smoke-default.log,smoke-shared.log}`, five negative logs,
and `{full,shared}/report.json` with captures/images. CI adds packaging/Rust
admission after native cooking; hosted execution is untested. No push/benchmark
claim. This fixed set does not implement asynchronous DAG publication,
residency reservation/deduplication or authored per-material texture selection.
Next is bounded authored image import; M4 remains open.

## Bounded authored shared PNG (2026-10-11)

The offline glTF normalizer adds opt-in `--unlit-texture` normalized schema 4:
existing opaque double-sided unlit factors/float2 UVs plus one shared external
PNG base-colour texture with explicit nearest-repeat sampler. It preserves
source image/texture indices, PNG bytes/hash, material-to-shared-texture mapping
and geometry provenance. Other source textures/sampler/URI/image/extension
profiles fail explicitly. Rust checks the source envelope; pinned KTX performs
full PNG decoding/CRC checking offline. No runtime parser/library or binary
asset format version changes.

The two transformed/reflected quads now reference a synthetic asymmetric RGBA8
2x2 PNG, SHA-256
`0016f27c6cc45584834ced00a220fe38aed4ab2ae65daa41a5d8d3c71054b93e`.
All 16 native decoded texel bytes match the independently known raw fixture.
Final content addresses:

| Role | Bytes | SHA-256 |
| --- | ---: | --- |
| Geometry: 8 vertices/4 triangles/1 meshlet | 2944 | `43e0a58528d4be9f0da385f9e5acc3d160a4add56ab8c3dada90025b5cd97f86` |
| Two unlit factors/triangle assignments | 444 | `de25e6d1155a58a16a81948bcaf9a0d7677429da3aaa43d347e5466a7e4d5b78` |
| Eight dense UV rows | 416 | `ceefc62001a3e74e397b913fa3beddbe5d9717da137916d99446999fd7be5dc1` |
| Opaque 2x2 KTX2 texture | 932 | `1460604394ce91a7be0ad21dd04197b7e83bbaac372f976f01610a209dd58aff` |
| Complete fixed bundle | 176 | `e3e78a89e0f755c2d7f21593ca88cf69c6f0570a7072e9c2aa88fc6e3e9983c6` |

All five independent Rust readers accept the final set. The general PNG
producer's `texels_verified=false` distinguishes decoded-opacity checking from
raw-source equality; this fixture's exact raw equality is checked separately.
Official KTX validation succeeds with the preceding custom-key warning recorded.
Repeated normalization/native cooking with the same tools/input is identical.
Original grid/quad/triangle content hashes are unchanged; legacy position-only,
factor-only and UV import/cook/Rust-reader sequences still pass.

Ten normalizer tests pass, adding source bytes/material/sampler bindings plus
invalid sampler/UV/index/image URI/MIME/extension/PNG envelope cases. Eleven
Python LOD/material/texture tests pass, including PNG-envelope bounds. Rust
formatting and denied-warning Clippy pass. Separately executed failures reject
CRC-corrupt, truncated and non-opaque PNGs before immutable texture publication;
five normalized mapping/hash/dimension/sampler cases fail before bundle
publication. A canonical symlink escape fails before reading the outside image.

Final-bundle RX 7900 XTX/RADV synchronization-validation WSI presents 120 frames
and ten generations with zero diagnostics including teardown. Separate default
and shared identity captures each present 180 frames and verify manifest/actual
GPU dependency bytes/texels, work/identity/depth and 85 sampled colours, including
all four visible triangles. Three samples per capture have UVs outside [0,1],
exercising repeat. Existing 2e-6 depth/two RGB-byte tolerances apply; capture
validation is disabled and separate from standalone validation. Runtime/shader
code is unchanged from the preceding qualified resident texture/bundle path.

Artifacts: `build/qualification/m4-gltf-image-{import.log,cook.json,cook.log,
readers.log,smoke.log,repeat-regression.json}`, negative logs under
`m4-gltf-image-negative/`, and `{full,shared}/report.json` with captures/images.
CI adds the authored-image sequence; hosted CI/Windows native tool and GPU
execution are untested. No push or benchmark claim. Next: bounded per-material
texture selection; mip/gradient/filtering and asynchronous residency remain open.

## CPU material-to-texture table (2026-10-11)

THEIATEX v1 adds a dense mapping from the exact material-page hash to a complete
bounded set of hash-sorted immutable textures. Cap 16 textures/256 materials,
table 1 MiB, metadata 64 KiB; each child retains its existing 4 MiB KTX2 budget.
Every texture must be referenced. This unit changes no runtime/shader code and
establishes no GPU table lookup, residency publication or new GPU capability.

File-backed fixture uses the preceding authored geometry/material pair,
its `146060...` PNG texture and a second synthetic raw 2x2 texture:
916 bytes, SHA-256
`ffd46b075fe9a6be2e61989c19f57c2ec330cab01575556088f110abb343539c`.
Native raw cooking verifies all 16 source bytes with the same pinned KTX recipe.
Texture table is 468 bytes, SHA-256
`52ca7edc46470ad0b2f9c73c11a5c7884fd8e8c17d95b2c6142b2788a2c9f0ca`,
bound to `de25e6...` material content. Independent Rust example admits both
opaque children and the canonical material mapping [1,0], deliberately distinct
from descriptor/hash order. Repeated table cooking is identical.

Three new Rust tests bring the asset suite to 27: dense reuse with three material
indices/two textures, full truncation/hash/count/binding checks, sorted unique
dependencies, metadata/size/index errors, unreferenced dependencies and freshly
hashed invalid-alpha children. Child lengths/hash are checked at complete
admission, separately from structural manifest decoding. Two Python producer
tests verify source-order permutation preserves physical material choices,
determinism and invalid source binding/mapping/duplicates/missing files.
All 13 Python tests, asset tests, fmt and denied-warning Clippy pass.

Artifacts: `build/qualification/m4-texture-table-tests.log` and
`m4-texture-table/{alternate-cook.json,assignments.json,cook.json,reader.log}`.
CI configuration adds native second-texture/table production and independent
Rust admission; hosted execution is untested. No push/benchmark claim.
Next qualification is indexed GPU material texture selection; authored multiple
PNG conversion, mip/gradient/filtering and asynchronous residency remain open.

## GPU indexed resident texture selection (2026-10-11)

Render Lab now connects THEIATEX to a fixed geometry/factor/UV set through
`--texture-table`. Complete source-set admission happens before Vulkan setup;
loaders recheck immutable bytes before upload. Device sampled-image descriptor
limits (including four target slots), image format/dimensions and table storage
range are queried. A sampled-image runtime array at binding 7, nearest-repeat
sampler at 8 and immutable table buffer at 9 feed material-indexed Slang resolve.
No per-frame host material lookup or descriptor sort is performed.

Both shader variants compile/SPIR-V validate 27 outputs. Fragment disassembly
records RuntimeDescriptorArray, ShaderNonUniform and
SampledImageArrayNonUniformIndexing capabilities, NonUniform decoration on the
sampled image, and OpImageSampleExplicitLod with level zero. Fourteen renderer
tests, formatting and denied-warning Clippy pass. Default/single-image paths
retain their existing shaders; only the indexed entry is new.

Each 2x2 resident image costs 4096 actual Vulkan allocation bytes/16 staging
bytes on this RX 7900 XTX. Two images succeed with an exact aggregate 8192-byte
cap. At 8191 bytes, the first image finishes its dedicated-fence upload, then
the second allocation is rejected; teardown releases the completed first image
with zero validation warnings/errors. Missing indexed fragment shader fails
after successful uploads/target construction and also cleans up without
diagnostics. Table/staging/target/geometry/sampler/descriptor allocations are
separate from this image-only aggregate budget.

Four isolated malformed sets (missing texture, corrupt texture, foreign material
binding, freshly hashed invalid mapping) fail before Vulkan initialization.
The preceding immutable source files remain untouched. Images/table/descriptor
owners retain the dedicated upload and frame/swapchain retirement contracts.

Standalone RX 7900 XTX/RADV synchronization-validation passes 19 configurations,
each 120 presented frames/ten generations with zero diagnostics through teardown:
14 preceding paths (bundle, single texture, UV, factors, visibility, indirect,
clear, mesh, task, spatial, asset, camera, frustum, diagnostic material cell),
plus indexed default/shared/terrain and exact-budget one/two-image cases.
The preserved headless compute independently verifies all 256 GPU values.

Separate RenderDoc captures disable capture validation; each target presents
180 frames. They verify exact GPU geometry/factor/UV/table bytes, the complete
texture texel set, GPU work/identities and CPU depth/sampled colours at the
preceding 2e-6 depth/two RGB-byte tolerances:

| Indexed case | Samples | Visible triangle probes |
| --- | ---: | ---: |
| Imported full | 85 | 4 |
| Imported empty | 81 | 0 |
| Imported narrow | 83 | 2 |
| Imported shared identity | 85 | 4 |
| Terrain | 593 | 512 |
| Terrain shared identity | 593 | 512 |
| One-texture table | 85 | 4 |

The two-material table retains `52ca7e...`, selecting [1,0] independently of
descriptor/hash order. Terrain's three-material table is 472 bytes, SHA-256
`59f9f597e154de591724153c3eac9481d001298c7c8c88986ec64903e48c698b`,
bound to `5a004f...` factors and selecting [1,0,1]. Both terrain replays observe
all (material,texture) pairs (0,1), (1,0), (2,1), proving reuse across distinct
materials/multiple meshlets. Reports retain sampled material/texture indices.
Original single-image/bundle replay passes 85 samples; original M3 replay
passes 84. These qualify texture counts one/two; execution of all counts up to
the CPU profile cap 16 is not claimed.

Artifacts: `build/qualification/m4-texture-table-gpu-{shaders.log,
shared-shaders.log,frag.spvasm,smoke.log,shared.log,budget-exact.log,
budget-reject.log,missing-shader.log,terrain-smoke.log,mono-smoke.log,
headless.log}`, regression logs, negative logs under
`m4-texture-table-negative/` and
`m4-texture-table-gpu-{full,empty,narrow,shared,terrain,terrain-shared,mono,
regression-single,regression-m3}/report.json` with captures/images.
No hosted CI, push, performance or Windows GPU claim. Multiple authored PNG
normalization/bundling is next; asynchronous residency and mip/derivative/
filtering/virtual textures remain open.
