# M4 bilinear sampling within automatic mips (2026-10-14)

Render Lab adds opt-in `--texture-linear-filter` to `--texture-auto-mips`.
The device selector queries optimal-tiling RGBA8_SRGB linear-sampling support;
unsupported devices fail qualification rather than silently substituting another
filter. Sampler min/mag become LINEAR, while mipmap mode stays NEAREST and the
unchanged Slang resolve selects an explicit integer mip. Complete-chain upload,
view range, repeat addressing, allocation caps, synchronization and RAII
remain as previously qualified. Existing default automatic/fixed/single/table
modes keep their nearest samplers. The flag requires automatic-mip mode and rejects
invalid use before Vulkan setup.

`--texture-linear-reference <mesh material uv texture x y width height view>`
combines the existing raster/UV/mip oracle with the independent ideal f64 bilinear
reference. RGB decode precedes interpolation, then linear material factors apply.
The GPU diagnostic gradient/mip records remain unchanged; RenderDoc independently
checks raw uploaded levels, identities/depth, all 81 records and sampled colours.
Colour tolerance stays two encoded RGB bytes, depth 2e-6. A filtered harness must
include at least one checked colour differing from nearest by over four bytes;
constant/smooth fixtures alone cannot claim filtering qualification.

RX 7900 XTX/RADV Mesa 26.2.4/X11, RenderDoc 1.46 replay:

| Fixture / interface | Views / checked pixels | Distinguishable filtered pixels | Sampled mips |
|---|---|---|---|
| Authored 8x8 checker / default | 0/81, 1/83, 2/85 | 0, 25, 6 | none, 0, 0 |
| Authored 1024x1 alternating / default | 0/81, 1/83, 2/85 | 0, 24, 5 | none, 2, 4/5 |
| Authored 8x8 checker / shared | 0/81, 1/83, 2/85 | 0, 25, 6 | none, 0, 0 |
| Authored 8x8 checker / wide fallback | 0/83, 1/83, 2/83 | 38, 83, 38 | 0 |

All twelve captures pass. Every checker capture verifies all four raw levels;
each thin capture verifies all eleven. Covered supported grid probes are 23 in
narrow view and two in full view; empty records are cleared. Wide-quad probes
retain unsupported-gradient reason 2, zero footprint and mip zero. The original
smooth thin chain also matched the filtered reference, but its harness intentionally
failed the filter-distinction gate; that run is not counted as completed qualification.
The alternating fixture closes that gap without relaxing tolerances.

The checker source is committed as `fixtures/textures/opaque_8x8_filter_mips.json`;
its native cooked chain is 1340 bytes with SHA-256
`9c92b1415664d7167e64d9e3b2751b825ac884899c2d4d408bab63fc76b9bcc2`.
`tools/cook_texture_filter_fixture.py` independently authors alternating texels at
every level for square/odd/thin/vertical profiles, then delegates to the existing
pinned KTX 4.4.2 cooker and exact-byte/official-validation gate. It does not generate
filtered lower levels from a base image. The tested thin chain is 9356 bytes,
SHA-256 `f1cfec0a4871f82b9ff9486c61b550eda7810f8f3615bfec8b7243f3f62753a0`;
complete repeated cooking is byte-identical.

Each final harness passes 120-frame WSI validation, another 120 frames at the exact
physical image cap, one-byte-short rejection and post-upload missing-shader cleanup.
Image charges are 4096 checker and 9472 thin; all checked teardowns report zero
warnings/errors. Capture validation is disabled, separate from these standalone
checks. Image budgets exclude probe/other buffers, staging, targets and descriptors.

Three additional checker captures with the default nearest sampler pass unchanged
sampling references, WSI and budget/cleanup checks. Preserved fixed-mip viewing and
16-image complete-set streaming each present 120 frames with zero diagnostics.

CPU assets 54 tests, renderer 23 tests, eight producer tests, formatting, denied-
warning Clippy and Python syntax/whitespace checks pass. All 69 existing modules
in the two shader output directories revalidate for Vulkan 1.3 (including retained
outputs); no shader source changes or new Slang compilation are claimed. Windows
GNU cross-link/Clippy passes all 11 packages; Wine executes 145 CPU tests. Native
Windows/MSVC/GPU, cross-vendor arithmetic and the prior Wine Quinn limitation
remain unqualified.

Owned Weston 13 nested on X11 executes actual Wayland surfaces in six preserved
modes plus thin-chain bilinear sampling, 120 frames each with zero Vulkan
diagnostics. No Wayland replay/pixel or native-seat claim; the existing Winit
programmatic restore exclusion and compositor layer-finalization diagnostic remain.

Reproduction after building the existing shader interfaces and renderer:

```sh
python3 tools/cook_texture_filter_fixture.py \
  --ktx build/qualification/ktx-tool/source-build/Release/ktx \
  --build-dir build/qualification/ktx-tool/source-build \
  --source-archive build/qualification/ktx-tool/ktx-source-v4.4.2.tar.gz \
  --profile thin --output-dir build/qualification/m4-linear-thin-pattern
python3 tools/qualify_automatic_mips.py \
  --reference build/qualification/m4-multi-image-cook.json \
  --texture-cook build/qualification/m4-linear-thin-pattern/cook.json \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --output build/qualification/m4-linear-gpu-thin-pattern --linear-filter
```

Use the ordinary mip cooker with the committed checker source for that case;
the helper's square profile is a separate pattern. Add `--shader-dir
build/shaders-shared` for shared replay. Native Wayland checks accept
`--auto-mip-case <geometry-report> <texture-report> --linear-filter`.
Local ignored reports are `m4-linear-gpu-{pattern,thin-pattern,shared,fallback}`,
`windows-linear-filter`, and associated `m4-linear-*` logs under
`build/qualification/`.

This qualifies a bounded within-mip bilinear path, not production filtering quality
or completion of M4. Integer footprint selection remains deliberately coarse;
trilinear/anisotropic filtering, source mip generation, arbitrary clipping/numerics,
tiled/partial residency, streamed full-chain replacement and performance stay open.
No locked ADR is changed.
