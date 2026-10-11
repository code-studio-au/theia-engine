# M4 resident texture-table capacity qualification (2026-10-11)

The existing runtime and Slang indexed sampling path now has actual RX 7900 XTX /
RADV Mesa 26.2.4 / X11 evidence for its maximum 16 resident textures. No Rust,
shader, dependency or locked architectural decision changed. M4 remains open for
general geometry LOD, streaming policy and virtual textures; this is a bounded
resident descriptor qualification, not a performance result.

`tools/cook_texture_capacity.py` reuses the verified eight-vertex/four-triangle
geometry and authored UV page from `m4-multi-image-cook.json`. Pinned source-built
KTX-Software 4.4.2 cooks 16 distinct opaque 2x2 RGBA8_SRGB spatial patterns. Each
916-byte texture passes official KTX validation and exact source texel comparison.
Hash-sorted descriptors map to 16 white material factors; four bundles assign
consecutive groups of four indices to the triangles. Every dependency is referenced
by the material table. Repeating the complete cook into the same location produces
an identical report, including all immutable content IDs.

| Sampled descriptor indices | Bundle SHA-256 |
|---|---|
| 0–3 | e3f5b7c101094345ef83c5a886fecd25c70be33a2625b4afffed0167adc9af80 |
| 4–7 | 7b28af46e56b701ff1722e1a441dd4903fce93171709014a973050478e39143d |
| 8–11 | afaf93ef3f1fc13870bda9271ace3020e45f286ea5b9a6c786e208d8d33d5f43 |
| 12–15 | 8530efcc4a26a67554b925dca4e0255c18bed75f8b607204424d97cef6e631e3 |

Each material page is 668 bytes, table 1084 bytes and v2 bundle 176 bytes. Geometry
and UV identities remain `62ccf421…` and `f6231d0e…`; full paths and provenance are
in the generated cook report. This synthetic capacity fixture is separate from
source glTF import qualification.

`tools/qualify_texture_capacity.py` runs four independent RenderDoc 1.46 captures.
Each replay verifies complete bundle dependencies, the exact raw GPU texels of all
16 resident images, identity/depth/UV/material output and 85 framebuffer samples.
Triangle-centroid probes ensure every assigned descriptor is actually sampled;
all indices 0–15 appear across the four captures. Capture validation is explicitly
disabled, as in the preceding qualified workflow. Runtime performs no readback.

Separate synchronization-validation runs verify:

- 120-frame WSI smoke with ten swapchain generations, resize/minimize/restore and
  labeled acquire/present recovery injections: zero warnings/errors through teardown.
- Sixteen images consume 4096 physical Vulkan allocation bytes each, total 65536.
  A 120-frame run with the exact image-only budget succeeds with zero diagnostics.
- At 65535 bytes, 15 image transfers have been submitted before allocation of the
  final image is rejected. Dedicated transfers/resources retire during error cleanup;
  zero swapchain generations, no presentation, zero validation warnings/errors.

The image-only budget excludes geometry/material/UV/table buffers, render targets,
staging and descriptor/sampler resources. This unit does not qualify concurrent
16-image replacement or an aggregate complete-bundle budget. Five existing texture
producer tests and Python syntax checks pass. Rust/shaders are unchanged, so their
preceding lint/test/SPIR-V evidence is retained without a new compilation claim.
Windows/Wayland/cross-vendor sampling at this capacity remains unqualified.

Reproduction, from the repository root with the qualified source-built KTX tool and
portable RenderDoc already staged under ignored `build/`:

```bash
python3 tools/cook_texture_capacity.py \
  --reference build/qualification/m4-multi-image-cook.json \
  --ktx build/qualification/ktx-tool/source-build/Release/ktx \
  --build-dir build/qualification/ktx-tool/source-build \
  --source-archive build/qualification/ktx-tool/ktx-source-v4.4.2.tar.gz \
  --output-dir build/qualification/m4-capacity/assets
DISPLAY=:0 VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
XDG_DATA_HOME="$PWD/build/qualification/renderdoc-data" \
python3 tools/qualify_texture_capacity.py \
  --cook build/qualification/m4-capacity/assets/cook.json \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --output build/qualification/m4-capacity/qualified
```

Observed logs/replay images/reports: `build/qualification/m4-capacity/qualified/`.
Repeated producer reports: `first-cook.json`, `assets/cook.json`; producer test log:
`producer-tests.log`. All changes remain local; no hosted CI or push claim.
