# M4 GPU automatic mip selection (2026-10-11)

Render Lab now accepts `--texture-auto-mips <sha256.ktx2>` with admitted fixed
geometry/material/UV pages. Complete-chain admission precedes Vulkan setup and is
rechecked before upload. The existing bounded upload owns one image and staging
allocation; its image view exposes every level. Nearest-repeat sampling uses an
explicit GPU-selected mip. Fixed-view and single-mip/table/bundle paths remain
separate. No per-frame host mip selection or runtime readback is introduced.

The Slang resolve reconstructs each visible primitive's perspective UV Jacobian
analytically, before repeat wrapping. It scales the two gradient columns by base
texture dimensions and chooses the first power-of-two footprint ceiling, clamped
to the complete chain. This implements the qualified CPU prototype policy;
hardware screen derivatives across primitive identities are not used. Wholly
near/far-unclipped vertices, positive W, bounded clip/UV magnitudes, area,
denominator, finite arithmetic and extent guards define the supported profile.
Valid UV reconstruction with unsupported gradients explicitly samples mip zero.
Invalid reconstruction/material indexing produces diagnostic magenta.

An optional lab-only fragment storage probe requires `fragmentStoresAndAtomics`,
four storage-buffer descriptors and a 3888-byte storage-buffer range. This feature
is queried/enabled only for automatic-mip mode; it does not change the approved
engine baseline. One RAII device-local buffer stores 81 records of 48 bytes at a
9x9 pixel grid. On this device requested/physical bytes are both 3888, separately
bounded to 1 MiB. Each resolve writes covered and background records; fragment
write reuse has an explicit Sync2 barrier. Dimensions below 10 or above 16384
skip diagnostic grid writes. Record fields are dx/dy, UV, footprint, mip, primitive
identity, reason and zero reserved word. RenderDoc reads this diagnostic buffer;
normal runtime never does.

RX 7900 XTX, RADV Mesa 26.2.4, Linux Mint/X11, RenderDoc 1.46 qualification:

| Final harness / view | Checked pixels | Supported grid probes | Fallback grid probes | Sampled mips |
|---|---:|---:|---:|---|
| Thin chain, default and shared / empty 0 | 81 each | 0 | 0 | none |
| Thin chain, default and shared / narrow 1 | 83 each | 23 | 0 | 2 |
| Thin chain, default and shared / full 2 | 85 each | 2 | 0 | 4, 5 |
| Wide quad, default / 0 | 83 | 0 | 36 | 0 |
| Wide quad, default / 1 | 83 | 0 | 81 | 0 |
| Wide quad, default / 2 | 83 | 0 | 36 | 0 |

Every capture verifies all 11 raw GPU mip levels and all 81 diagnostic records,
including explicit zeroed background fields. Supported gradients compare with the
independent f64 CPU oracle at `1e-5 + 0.001 * abs(reference)`; footprint tolerance
is `1e-3 + 0.001 * reference`, UV tolerance `2e-4`. Mip choice is exact. Fallback
requires reason 2, zero gradients/footprint and mip zero. Colour/depth compare at
the existing two-byte RGB / `2e-6` depth tolerances, including extra interior
primitive pixels. Capture validation is disabled; separate WSI runs provide
validation evidence. Earlier individual square/odd captures each verify three
uploaded levels, 85 colour/depth pixels and mip-zero selection.

The fallback fixture is a native meshoptimizer-cooked four-vertex/two-triangle
wide quad: visible geometry exceeds the gradient clip-magnitude profile while
remaining within near/far limits. Geometry is 760 bytes, SHA-256
`5ce9c2ed83ef850cd0b554141b8e8a4f375a9e40ee4aa6ca72a859b74110af05`.
The helper attaches admitted material/UV pages and source provenance. Native
cooking verifies triangle winding/content; no LOD error bound is claimed.

Each of the three final harnesses passes a 120-frame validation WSI run and another
120 frames at the exact 9472-byte physical image budget. A 9471-byte limit rejects
before presentation. Removing the automatic fragment module tests cleanup after
texture upload and probe allocation. All checked success/failure teardowns report
zero warnings/errors. Image budgets exclude staging, geometry/material/UV/probe
buffers, render targets and descriptor/sampler resources.

Preserved indirect, authority, 16-image complete-set streaming and fixed-mip modes
each present 120 frames with zero diagnostics. Renderer 23 tests, eight texture
producer tests, formatting, denied-warning renderer Clippy and Python checks pass;
the unchanged assets suite's 50 tests passed in the preceding footprint unit.
Both default/shared shader builds compile and validate 32 outputs for Vulkan 1.3.
Existing Slang profile-upgrade notices remain; warning-free compilation is not
claimed. Windows GNU cross-link/Clippy passes for 11 packages; Wine runs 141 CPU
tests. Native Windows/MSVC/GPU execution remains unqualified.

Owned nested Weston 13 executes six preserved modes plus automatic thin-chain
sampling, 120 frames each with zero Vulkan diagnostics. This qualifies native
Wayland surfaces inside Weston-on-X11, without Wayland capture/pixel replay or
native-seat claims. Programmatic restore is excluded by the existing Winit limit;
the compositor's known layer-finalization BUG on termination remains recorded.

Reproduction (build shader variants before running their corresponding harness):

```sh
python3 tools/compile_render_shaders.py
python3 tools/qualify_automatic_mips.py \
  --reference build/qualification/m4-multi-image-cook.json \
  --texture-cook build/qualification/m4-mip-contract/thin-cook.json \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --output build/qualification/m4-auto-mips-qualified
python3 tools/cook_texture_footprint_fixture.py \
  --library build/qualification/meshoptimizer/lib-build/libmeshoptimizer.so \
  --source-archive build/qualification/meshoptimizer/meshoptimizer-1.3.tar.gz \
  --output-dir build/qualification/m4-auto-mips-fallback-assets
```

Save the cook helper's JSON stdout as a reference report. Use that report with
the same harness for fallback; use `--shader-dir build/shaders-shared` for the
compiled shared interface. Local ignored reports/logs are in
`build/qualification/m4-auto-mips-qualified{,-shared,-fallback}`;
`m4-auto-mips-{square,odd}`; `wayland-auto-mips`; and `windows-auto-mips`.
The harness environment used `DISPLAY=:0`, the Radeon ICD and the owned RenderDoc
data directory described in the preceding GPU mip evidence.

This closes a bounded automatic-selection prototype gate, not M4. Filtering
quality, arbitrary clipped/degenerate/numerical cases, cross-vendor arithmetic,
anisotropy, tiled/partial residency, full-chain bundle replacement and performance
remain open. No locked ADR is changed. Mip-zero fallback protects the prototype
from unsupported gradients; it is not a texture streaming fallback policy.
