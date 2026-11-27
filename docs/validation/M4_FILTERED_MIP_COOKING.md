# M4 native box-filtered mip cooking (2026-10-14)

`tools/cook_filtered_texture.py` qualifies a bounded native-generated colour mip
chain. Input is the existing schema-1 dense opaque RGBA8 sRGB JSON, with power-of-two
dimensions 1..1024 and a complete-chain byte preflight. Nonopaque/noninteger texels,
odd dimensions and chains exceeding the existing encoded budget reject. This is
separate from the explicit-authored-level cooker; authored mip bytes remain supported.

The existing single-level producer first verifies pinned KTX 4.4.2 tool/source/library
hashes, Release configuration and disabled ETC unpacker. Raw source pixels are
losslessly wrapped in a temporary noninterlaced 8-bit RGBA PNG with sRGB metadata.
The official tool cannot combine generation with `--raw`, so this envelope only
bridges native input decoding. Native generation requests a complete chain with
box filter, scale 1, clamp boundaries, sRGB/BT709 and top-left orientation; the
resulting level count is independently checked.

Native KTX performs the production filtering. Its pinned
[resampler](https://raw.githubusercontent.com/KhronosGroup/KTX-Software/v4.4.2/tools/imageio/image.hpp)
decodes RGB before filtering, keeps alpha linear and re-encodes/quantizes output.
The pinned [generator](https://raw.githubusercontent.com/KhronosGroup/KTX-Software/v4.4.2/tools/ktx/command_create.cpp)
iteratively resamples the preceding level. The Python verifier independently checks
exact base texels, opaque alpha, level ranges/dimensions and each lower pixel against
f64 linear-light 2x2 (or 2x1/1x2) averages of the actual previous level, then sRGB
encoding. The permitted per-step RGB error is one byte; all qualified native outputs
below have observed maximum error zero. This is a reference check, not a Python
production mip generator or proof for arbitrary kernels/non-power-of-two images.

The existing complete-chain packager preserves native DFD/texel bytes, appends
source/tool/compiler/cooker hashes plus the PNG envelope hash and exact recipe,
and relocates level offsets. Official KTX validation precedes hash-named publication.
All five resulting files independently pass the strict pure-Rust complete-chain
reader with filename/content hash, layout/alpha/metadata and cooked provenance.

| Native source | Generated levels | Encoded bytes | Observed max RGB error |
|---|---:|---:|---:|
| 8x8 RGB checker | 4 | 1480 | 0 |
| 8x4 varied rectangle | 4 | 1312 | 0 |
| 1024x1 varied thin | 11 | 9496 | 0 |
| 1x1024 varied vertical | 11 | 9496 | 0 |
| 1x1 singleton | 1 | 1072 | 0 |

All 31 levels are checked. Square and thin repeated cooking is byte-identical.
The committed `fixtures/textures/opaque_8x8_filter_source.json` has black/white
patterns: lower RGB samples become 188, distinguishing linear-light averaging from
the incorrect encoded-space value 128. Its cooked chain SHA-256 is
`ef3d361f1de1d4b6262f4f2f70158e73377bba07aaaba9edbcf218cda8e9fc05`;
the thin chain is `5c60afd7d8b0bf4cb10f8db4b3aae87b985d2456d7c3ae4e7fedfee247970e69`.
Hashes/recipes for every case are in the local native reports.

Five producer tests check the 188/128 discriminator, explicit one-byte versus larger
errors, exact base/alpha/header/range/truncation rejection, strict source profile
and PNG CRC/top-left scanlines. All 13 texture-related producer tests and Python
syntax/whitespace checks pass. No Rust or shader changes occur in this unit.

RX 7900 XTX/RADV Mesa 26.2.4/X11, RenderDoc 1.46 integration:

- Three square fixed-view captures sample levels 0/2/3, verify all four raw GPU
  levels and 85 colour/depth probes each. Each view presents 120 validation WSI frames.
- Three thin automatic captures check empty/narrow/full views, all 11 raw GPU levels,
  81 diagnostic records each and 81/83/85 pixels. Covered samples select levels 2/4/5.
- Both harnesses pass exact image limits (4096 square, 9472 thin), one-byte-short
  clean rejection and zero Vulkan diagnostics through checked teardowns. Square
  invalid level/alpha rejects before Vulkan; thin missing-shader cleanup also passes.

Capture validation is disabled and separate from standalone validation. An initial
square capture timed out while the desktop display was in DPMS-off state; it is not
counted as a pass. After waking the display, all six captures and standalone checks
complete. Lower levels not listed as sampled have raw upload-byte proof only.
Uniform filtered lower colours alone do not distinguish all sampler level mappings;
the preceding authored-pattern qualification retains that independent evidence.

Reproduction:

```sh
python3 tools/cook_filtered_texture.py \
  --ktx build/qualification/ktx-tool/source-build/Release/ktx \
  --build-dir build/qualification/ktx-tool/source-build \
  --source-archive build/qualification/ktx-tool/ktx-source-v4.4.2.tar.gz \
  --source fixtures/textures/opaque_8x8_filter_source.json \
  --output-dir build/qualification/m4-filtered-mips/square \
  > build/qualification/m4-filtered-square-cook.json
```

Feed this report to `tools/qualify_texture_mips.py`; generated thin chains can use
`tools/qualify_automatic_mips.py` without a linear-filter distinction requirement.
Local ignored evidence: `m4-filtered-mips/report.json`, per-source native/admission/
repeat logs, `m4-filtered-gpu-{square,thin}/report.json`, and
`m4-filtered-{producer-tests,all-producer-tests}.log` under `build/qualification/`.

This qualifies one bounded colour production recipe, not general texture quality,
glTF filtered-image import, normal/mask/HDR/alpha handling or a final engine filter
policy. Windows native tooling/GPU, odd-dimension kernels, compression, tiled demand,
async tail publication, multi-frame reuse and performance remain open. No locked
ADR changes; M4 remains open.
