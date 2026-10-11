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
