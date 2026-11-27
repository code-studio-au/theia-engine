# M4 conventional GPU coarse mip tails (2026-10-14)

Render Lab `--texture-tail-budget <dense texel bytes>` requires automatic-mip
mode and selects the qualified CPU plan before Vulkan initialization. The complete
content-addressed source is admitted/rechecked; full-file CPU I/O is still required.
The initialization bytes are released after upload rather than kept in a CPU cache.
Only the selected contiguous suffix is copied into a conventional image whose base
extent is the first resident source level. The protected 1x1 level is always present.
There is no sparse binding, arbitrary tile mapping or partial file I/O.

One bounded upload copies resident levels to image-relative indices with full-range
barriers and dedicated-fence retirement. Actual image allocation requirements still
use `--texture-budget`; dense texel and physical limits are independent. Full-chain,
fixed-view, single/table/bundle paths remain separate. The sampler supports nearest
or opt-in within-mip linear filtering with the preceding queried capabilities.
No per-frame host mip choice or normal-runtime GPU readback is introduced.

The separate tail fragment module extends the camera push block from 64 to 80 bytes:
four u32 words at byte 64 carry original width/height, first resident source level
and original level count. Existing modules keep the 64-byte layout. The shader
checks the resident dimensions/count against these values, evaluates gradients and
footprint using original dimensions, clamps requested source levels to the resident
tail, then subtracts the first level for sampling. Odd original dimensions must not
be inferred by shifting the smaller resident dimensions back upward.

The diagnostic buffer remains 81 records of 48 bytes. In tail mode byte 28 records
selected source mip and byte 44 records image-relative mip. Reason 4 means supported
gradients requested unavailable finer data; reason 2 still denotes unsupported
gradients, which request source mip zero and then use the finest resident level.
Background fields remain zero. Existing nontail records retain reserved zero at 44.
Pipeline/descriptor generation lifetimes and fragment write-reuse barriers remain
as previously qualified; source/push metadata stays immutable across recreation.

RX 7900 XTX/RADV Mesa 26.2.4/X11, RenderDoc 1.46 final replay:

| Source / mode | Dense cap | First source level | Resident levels | Physical image bytes | Sampled source mips (narrow/full) |
|---|---:|---:|---:|---:|---|
| 1024x1 alternating / nearest | 508 | 4 | 7 | 1792 | 4 / 4,5 |
| 1024x1 alternating / linear | 507 | 5 | 6 | 1536 | 5 / 5 |
| 1024x1 alternating / protected only | 4 | 10 | 1 | 256 | 10 / 10 |
| 1024x1 alternating / full tail | 8188 | 0 | 11 | 9472 | 2 / 4,5 |
| 1024x1 alternating / shared interface | 508 | 4 | 7 | 1792 | 4 / 4,5 |
| 1024x1 alternating / wide numeric fallback | 508 | 4 | 7 | 1792 | 4 / 4 |
| 5x3 odd authored / nearest | 12 | 1 | 2 | 512 | 1 / 1 |
| 1x1024 alternating / nearest | 508 | 4 | 7 | 12288 | 4 / 4 |

Each row has three view captures: ordinary scenes check 81/83/85 pixels for empty/
narrow/full views; wide fallback checks 83 each. All 24 tail captures verify every
resident mip's raw GPU bytes against its corresponding original KTX index and all
81 gradient/identity/source-mip/image-mip/reason records. Narrow ordinary views
have 23 residency-coarsened grid probes for nonzero tails. Full horizontal tail-4
has two uncoarsened probes; tail-5 has one uncoarsened and one coarsened. Protected
and odd tails coarsen both full-view probes. Wide numeric fallback retains reason 2
at 36/81/36 covered grid probes. Empty views clear diagnostics explicitly.

Eight standalone WSI runs and eight exact-physical-cap runs each present 120 frames;
each one-byte-short physical cap rejects without presenting. Each missing tail-module
case tests cleanup after image/probe allocation. All checked teardowns report zero
Vulkan warnings/errors. Dense budgets 0..3 reject before Vulkan, including readonly
background references; invalid tail-mode combinations reject. The vertical case's
12288-byte charge despite 508 dense bytes illustrates driver tiling/alignment costs;
the measured horizontal savings are not a universal allocation guarantee.

## Filter precision qualification

The initial linear tail-5 replay failed the ideal two-byte RGB oracle at pixel
(864,216): red was 10 rather than approximately 6, with matching depth/identity/mip.
The selected device reports `subTexelPrecisionBits=8`. Vulkan's
[linear filter weight rules](https://raw.githubusercontent.com/KhronosGroup/Vulkan-Docs/main/chapters/textures.adoc)
use this precision for fractional weights. A new CPU diagnostic
`linear_repeat_bounds` computes bilinear extrema over a one-quantization-step
interval per weight, clipped to [0,1], with RGB decoding before interpolation.
Its bounded precision profile is 4..16 bits; it is not a complete model of all GPU
coordinate/colour arithmetic or a cross-vendor proof.

The harness reads the selected device's reported precision from a separate validated
one-frame run. Linear reference commands accept trailing `--filter-precision <bits>`
and return linear colour bounds after material factors. Replay allows two encoded
bytes beyond those per-channel bounds; nearest paths keep the previous two-byte
ideal check, and depth/gradient/identity/mip tolerances do not change. A fixture must
still distinguish nearest from the bounded filtered interval. The final tail-5
captures pass; exactly one pixel needs the precision envelope. The initial failure
is not counted as a successful run. A new CPU test checks floor/ceil/nearest weight
quantization at dark edges, ideal inclusion and invalid precision rejection.

## Preserved paths and platforms

Three additional complete-chain bilinear checker captures pass the same precision-
aware oracle, all-level proof, WSI and budget/cleanup checks. Owned nested Weston 13
executes six preserved modes and linear tail-5, 120 frames each with zero Vulkan
diagnostics. This is actual Wayland surface execution inside Weston-on-X11, without
Wayland replay/pixel/native-seat claims. Programmatic restore is still excluded;
the compositor's existing layer-finalization diagnostic remains.

Preserved fixed-mip viewing and 16-image complete-set streaming each present 120
frames with zero diagnostics. The unchanged headless Slang compute test again
verifies all 256 values on the target GPU. Invalid filter precision rejects even
for readonly background references.

Assets 59 tests, renderer 23 tests, eight producer tests, formatting, denied-warning
Clippy and Python checks pass. Both shader variants compile/validate 33 outputs for
Vulkan 1.3; existing profile-upgrade notices remain. Windows GNU all 11 packages
cross-link/lint and Wine executes 150 CPU tests. Native Windows/MSVC/GPU and the
previous Wine Quinn startup limitation remain unqualified.

Reproduction with the existing cooked sources:

```sh
python3 tools/qualify_automatic_mips.py \
  --reference build/qualification/m4-multi-image-cook.json \
  --texture-cook build/qualification/m4-linear-thin-pattern/cook.json \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --output build/qualification/m4-gpu-tail-508 --tail-budget 508
```

Use 507 plus `--linear-filter` for the precision case, 4 for protected-only or
8188 for full-tail control. Add `--shader-dir build/shaders-shared` for shared
interface. The fixture helper's `--profile vertical` produces the vertical source.
Wayland accepts `--auto-mip-case <scene-report> <texture-report> --tail-budget 507
--linear-filter`. Local ignored logs/reports are `m4-gpu-tail-*`,
`wayland-mip-tail`, `windows-mip-tail-final` and `m4-gpu-tail-admission.json` under
`build/qualification/`.

This closes a bounded conventional mip-tail sampling gate, not M4 or virtual
texturing. GPU page demand, tiled residency, asynchronous tail replacement,
multi-frame retirement/reuse, filtered mip generation/quality and performance remain
open. No locked ADR changes.
