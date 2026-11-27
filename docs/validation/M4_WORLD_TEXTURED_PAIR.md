# M4 two complete resident textured chunks (2026-10-15)

`--world-bundle-pair <bundle0> <bundle1>` connects complete v2 CPU admission,
independent GPU resource owners and two-chunk shared visibility/depth to textured
resolve. Eight root buffers and eight images in the qualified four-image-per-
chunk fixture remain independently bound. CPU sources share a bounded unique-
encoded pool. All transfers retire before publishing the initial two-chunk CPU
transaction and constructing the graphics generation. There is no live replacement.

An owned native Wayland first run completes 120 frames and seven swapchain
generations under recovery injection with zero Vulkan warnings/errors through
teardown (`world-texture-first/run.log`). The original-palette pair requests
178560 bundle physical bytes, including 131072 image bytes; its shared CPU pool
charges 117008 unique encoded bytes. This excludes work arenas, staging, targets,
descriptors, queries, samplers, container and driver overhead, not total VRAM.

The extra `cook_world_pair_variant.py` correctness fixture changes chunk 2's source
PNG palette through `rgb=(255-b,255-r,255-g)`, preserving opaque alpha. The ordinary
Rust glTF importer and pinned native meshoptimizer/KTX pipeline run on that new
source. Normalized positions, indices, UVs, material assignments/factors and source
texture bindings match the original exactly. Independent checks compare every
native KTX payload byte with the expected palette transformation; original cooker
provenance is preserved rather than rewriting its verification flags. This makes
accidental first-chunk image bindings fail visible colour checks. Generated
variant artifacts remain ignored under `world-texture-variant/`.

The distinct-palette pair requests **178576 bundle physical bytes**, including
**131072 image bytes**, and **186224 unique CPU encoded bytes** on the qualified
RADV device. Different metadata sizes affect buffer alignment; these observations
are specific to these generated fixtures. The renderer's sum of retained resource
owners must equal the upload helper's physical ledger.

`qualify_world_textured_pair.py` independently computes CPU byte unions and checks
native exact/short construction caps, missing shader cleanup and mode conflicts.
Replay compares both complete GPU root sets, all eight raw texture images, both
GPU frustum compactions, whole-image identity/primitive/depth bounds and nearest
ray intersections against each page. Pixel probes select the actual winning
chunk's material and texture, preserving the existing 0.002-texel nearest-boundary
envelope, two-byte encoded RGB tolerance and 0.00002 depth tolerance.

This bounded resident profile uses fixed cameras, opaque unlit factors and
nearest mip-zero sampling. It is separate from the single-bundle crowd benchmark,
which must retain its visual-proxy/HUD regression. Camera demand, fallback coverage,
independent live replacement, representative art/geometry LOD/mips and native
Windows GPU remain separate gates. M4 is still open.


`world-texture-final/report.json` passes **12 native plans**: baseline, recovery,
exact caps, one-byte-short CPU/physical/image caps, zero physical cap, missing
textured shader cleanup, geometry mode without that optional module, two
pre-Vulkan mode conflicts and a shared-profile resident pair. Runtime cases have
zero Vulkan diagnostics through teardown; construction failures present no frames.
Five textured replays pass **579 ray/colour probes**, including swapped inputs,
angled/empty views and the shared primary interface. All four materials in both
chunk namespaces are sampled. Chunk tags 0/1 win 7/3 two-hit occlusion probes.
All eight GPU root buffers and eight raw image payloads match admitted content.

A preserved geometry-only replay adds 129 probes. The original mixed world with
1000 visual proxies passes another 52 ray/colour probes and **5250 HUD glyph
pixels**. Both independent compactions and whole-image identity/depth bounds pass.
Replay validation is disabled; independent native routes use actual validation.

Default/shared interfaces compile and validate **45 SPIR-V modules**; the original
specific shared-primary GLSL fallback remains unchanged. Renderer CPU tests pass
28, formatting and warning-denied Clippy pass. Windows GNU cross-link/Clippy passes
eleven packages and Wine CPU tests pass (`windows-world-texture/report.json`);
native Windows GPU is still unqualified.

Logs under ignored `build/qualification/`: `world-texture-{build,clippy,tests,
shaders,shared-shaders}.log`, `world-texture-final/` and
`world-texture-preserved-regressions/`. The last reruns isolated four-chunk upload
ownership/failure cleanup, 120-frame cooked replacement and the 10000-proxy mixed
route after extracting the shared upload helper and owned-geometry constructor.
No generated fixtures/captures are committed.


The optimized executable separately completes a 120-frame exact-cap variant pair
and a 120-frame 10000-proxy mixed route (10 warmup frames excluded), each with
actual validation and zero diagnostics. Evidence is `world-texture-release/` and
`world-texture-release-build.log`. The original headless compute app also verifies
all 256 GPU values (`world-texture-compute.log`); its sources/shader are unchanged.
