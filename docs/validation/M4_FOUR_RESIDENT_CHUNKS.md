# Four complete resident world chunks (2026-10-15)

The isolated `--world-scene-test <scene.json>` profile composes four fine village
quadrants: 43 meshlets, 2048 triangles, 64 buildings, 128 trees and continuous
8 m triangular ground tiles. This is a fixed resident qualification scene, not
a timed benchmark or camera-driven world streamer. The existing coarse-world
10000-proxy benchmark and independent two-chunk replacement profile stay separate.

Each chunk owns its geometry, material, UV and texture-table buffers and uploaded
images. All four encoded graphs stage against one deduplicating CPU pool; transfers
retire before one CPU publication at epoch 1 and graphics descriptor construction.
Each chunk has its own 256-byte GPU work arena, GPU meshlet compaction and indirect
task/mesh draw into shared identity/depth targets. Frame-local identity uses bits
29..30 for one of four slots, low 29 bits for local meshlet+1, and a separate
primitive word. This profile does not change the older pair's bit-30 encoding.

The fragment shader selects four-element storage descriptor arrays for geometry,
materials, UVs and texture tables, plus one combined sampled-image array. It
reconstructs perspective-correct UVs, checks reconstructed depth, samples opaque
nearest-repeat mip-zero sRGB images and multiplies linear unlit material factors.
A pushed four-word texture-base table allows differing image counts per chunk.
The optional storage-array features are requested only for this profile and the
preceding compute probe. Sixteen fragment storage descriptors and the actual
image/sampler/push limits are checked; this does not establish a production
binding policy or new global GPU baseline.

## Qualification limitation found and retained

Direct divergent indexing of the push-constant `uint4` texture-base vector passed
SPIR-V and native Vulkan validation but failed the varied-palette replay: a chunk-0
roof at pixel 446,269 sampled chunk-2 imagery. Captured offsets were correct
`[0,4,8,10]`, and identity, depth, buffer contents and image payloads passed. Explicit
selection of scalar `.x/.y/.z/.w` components passes the same colour oracle. Keep
that qualified expression. This differential result localizes the failing form;
it does not independently assign the defect to Slang, RADV or replay. No shader
language fallback or looser pixel tolerance was used.

The variant producer's `--images 2 --factors` changes chunk 2's palette, aliases
four material slots onto two textures and assigns distinct nonwhite factors. It
checks unchanged geometry, UVs and material slots, expected factors and exact
native KTX pixels before publishing the fixture. Canonical table hashes map the
CPU oracle's source image order to material slots.

## Evidence

Native plans cover recovery, variant, empty work, both shader profiles, exact CPU/
physical/image caps, each one-byte-short cap, incomplete fixture, missing-module
cleanup, legacy pair and compute probe without scene modules, and conflicting
modes before Vulkan. All successful scene startups publish four complete graphs;
short caps fail before publication/presentation. Recovery presents 120 frames over
seven generations. Actual validation-layer activation and teardown totals are
required by the harness.

Base scene charges **165148 unique CPU encoded bytes**, **357472 physical resource
bytes**, including **262144 image bytes**, for 16 root buffers and 16 images.
Physical figures exclude the four work arenas, staging, descriptors, commands,
visibility/swapchain targets and driver overhead; they are not total VRAM.

Replay checks all 16 raw buffer payloads, all resident image payloads, four GPU
compactions, whole-image chunk/primitive/depth bounds and independent nearest-hit
rays with identity, depth and sRGB/material-factor colours. Empty, full and angled
views, varied image counts/factors and the shared-interface shader profile are
included. Capture runs disable validation; native plans supply validation evidence.

Reproduce after cooking the canonical scene and varied fixture:

```sh
python3 tools/cook_world_pair_variant.py --help
python3 tools/qualify_world_scene.py \
  --scene build/qualification/resident-world-cook/scene.json \
  --variant build/qualification/world-scene-variant/scene.json \
  --shared-shaders build/shaders-shared \
  --output build/qualification/world-scene-final
```

Local logs/artifacts remain ignored under `build/qualification/`. M4 remains open:
protected fallback coverage, camera-driven GPU demand, four-chunk live replacement,
benchmark integration, general terrain/geometry LOD, texture streaming/virtualization,
allocation scale and other vendors/native Windows GPU execution remain unqualified.

`world-scene-final/report.json` passes **15 native plans** and **six replays**,
with **1247 ray/colour probes** and all 16 chunk/material combinations. Chunk slots
0/1/3 win 23/25/8 multi-hit depth probes respectively; slot 2 is still covered by
ordinary nearest-hit probes. The varied fixture retains 16 root buffers and 14
images, charging **199440 CPU**, **324400 physical** and **229376 image bytes**.

Both default/shared shader profiles compile and validate **48 SPIR-V modules**
(`world-scene-{shaders,shared}-final.log`). Renderer CPU tests pass 28, formatting
and denied-warning Clippy pass; Windows GNU cross-link/Clippy covers eleven
packages and Wine CPU tests pass (`windows-world-scene/report.json`). Native
Windows GPU remains unqualified. The unchanged original compute app again
verifies 256 values (`world-scene-original-compute.log`).

The optimized executable also passes recovery, exact-cap full startup, varied
factors/images and one-byte-short GPU rejection (`world-scene-closeout/report.json`).
The preserved two-chunk live replacement replay passes 129 ray/colour probes after
its four-to-two-image publication; the extracted texture-table oracle is exercised
by both paths. The six ownership/cooked-stream/10000-proxy regression plans pass
with zero native diagnostics (`world-scene-preserved-regressions/report.json`).
