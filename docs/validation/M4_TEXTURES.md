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
