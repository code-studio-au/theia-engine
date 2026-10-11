# M4 complete GPU mip upload and fixed-view sampling (2026-10-11)

The separate authored mip-chain profile now executes in Render Lab with
`--texture-mip-test <sha256.ktx2> <level>`, fixed geometry/material/UV inputs.
Source hashes, complete layout/provenance and selected level are checked before
Vulkan initialization, then rechecked on load. Legacy single-mip/table/bundle
admission is preserved; full-chain bundle replacement is not introduced.

The exact RGBA8_SRGB device format/extent/max-mip limits are queried. One device-local
image owns every admitted mip. One bounded staging allocation packs base-first texels;
one transfer command submits one copy region per level. Sync2 barriers cover the
complete image range before/after copy, and the existing dedicated transfer fence
retires before staging release and descriptor use. Actual physical allocation size
is checked before memory allocation; staging has a separate bounded charge.

The sampler retains nearest/repeat and view-relative level zero. An image view
exposes exactly the selected physical mip; the existing Slang sample-at-zero shader
therefore samples that level. Geometry/factors/UV and shaders are unchanged. The
fixed choice is made once at initialization; this is not automatic GPU mip selection,
derivative reconstruction, mip filtering or tiled/partial residency. Image RAII owns
view/image/memory, and shader/descriptor lifetimes retain the same frame-safe rules.

RX 7900 XTX/RADV Mesa 26.2.4/X11 replay:

| Source chain / interface | Sampled fixed views | GPU levels verified per capture | Pixel samples per capture |
|---|---|---:|---:|
| 4x4 square / default | 0, 1, 2 | 3 | 85 |
| 5x3 odd rectangle / default | 0, 1, 2 | 3 | 85 |
| 1024x1 thin / default | 0, 5, 10 | 11 | 85 |
| 4x4 square / shared primitive | 0, 1, 2 | 3 | 85 |

All twelve RenderDoc 1.46 captures verify every native level's raw GPU bytes,
identity/depth/UV/material output and CPU-reference sampled colour. Thus the eleven-
level chain's unsampled views still have upload-byte proof; their raster sampling is
not claimed. Capture validation is explicitly disabled. The CPU mip oracle checks
level range even at background pixels and applies nearest-repeat/sRGB before factors.
Runtime performs no readback.

Each captured view also passes a separate 120-frame WSI validation run. Four exact-
budget 120-frame runs and four one-byte-short initialization failures pass zero
warnings/errors through teardown. Measured full-image charges are 4096 bytes for
square/odd chains, 9472 for the eleven-level thin chain, independent of fixed view.
The square's 84 source bytes require 96 physical staging bytes; thin has 8188 source
bytes/8192 staging. These image-only limits exclude geometry/material/UV buffers,
render targets, sampler/descriptor/pool resources and staging. Short budgets reject
before presentation; invalid selected level and newly hash-named nonopaque mip data
reject before Vulkan context creation for each chain/interface case.

Preserved indirect/authority/16-image complete-set streaming WSI runs each pass
120 frames with zero diagnostics. Renderer 23 tests, assets 46 tests, formatting,
denied-warning renderer Clippy, three Python syntax/whitespace checks pass. Ten
unchanged active SPIR-V modules revalidate (five per interface); no shader compilation
or new Slang language-feature qualification is claimed in this unit.

Owned nested Weston executes the six preserved modes plus thin-chain views 0/5/10:
nine native Wayland WSI runs, 120 frames each, all zero Vulkan diagnostics. This is
execution, not Wayland mip pixel/replay evidence. Existing programmatic restore
exclusion and Weston's own shutdown diagnostic remain. Windows GNU all 11 packages
cross-link/lint; 137 Wine CPU tests pass. Native Windows/GPU/Quinn/cross-vendor and
performance gates remain open; M4 is not complete.

```bash
DISPLAY=:0 VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
XDG_DATA_HOME="$PWD/build/qualification/renderdoc-data" \
python3 tools/qualify_texture_mips.py \
  --reference build/qualification/m4-multi-image-cook.json \
  --texture-cook build/qualification/m4-mip-contract/cook.json \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --output build/qualification/m4-gpu-mips-square
# Use odd-cook.json / thin-cook.json, or --shader-dir build/shaders-shared.
```

Observed artifacts: `build/qualification/m4-gpu-mips-{square,odd,thin,shared}/`
reports/logs/captures/images; first diagnostic capture and tests/lint/build/SPIR-V/
regression logs; `wayland-texture-mips/` and `windows-texture-mips/`. Next qualify
perspective-correct authored-UV gradients/footprint and automatic mip selection before
mip filtering or tiled fallback. All changes remain local; hosted CI unexecuted.
