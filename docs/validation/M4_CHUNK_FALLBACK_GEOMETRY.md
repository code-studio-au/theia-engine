# Edge-compatible cooked quadrant fallbacks (2026-10-15)

`cook_benchmark_scene.py --chunk-fallbacks` optionally adds four complete coarse
quadrant bundles alongside the original fine scene. They are offline fallback
candidates. No runtime camera demand, automatic LOD switching, protected residency
or crowd benchmark integration is implied by these files.

Each 64 m quadrant uses a 16 m interior grid, with boundary triangle edges split
at their 8 m midpoints. All perimeter vertices retain the fine positions, heights
and UVs exactly. There are no skirts, overlapping cover tiles or artificial lifts.
Each ground page has 41 vertices and 48 triangles, compared with 128 fine triangles.
All authored building/tree triangles, winding, UVs and material slots are preserved.
Total scene geometry falls from 2048 triangles/43 meshlets to **1728 triangles/31
meshlets**. The reduction is modest because props remain unchanged.

Three independent geometry tests check all **16 fine/coarse quadrant selections**
for positive winding, exact projected area, matching shared vertex heights and
edge multiplicity. Every internal edge has two users and each outer edge one;
a changed seam height is rejected. Exact rational continuous heightfield overlays
prove complete nonoverlapping quadrant coverage and a vertical error upper bound
at most **0.11387628316879274 m**, below the recipe's 0.12 m limit. Removing a
coarse triangle fails that gate. This bounds vertical surface difference only,
not normals, silhouettes, runtime arithmetic, collision or gameplay navigation.

The ordinary Rust glTF importer and pinned native meshoptimizer/KTX cooker process
all eight graphs. `qualify_benchmark_scene.py` checks source/normalized hashes,
exact position/UV rows, winding-preserving triangle/material multisets, every
native texture pixel, dependency bindings and pure Rust complete-bundle admission.
It checks **131072 texture texels** across the eight graphs. Five original scene
tests and the original fine-plus-whole cooked-source qualification also pass.

`qualify_chunk_fallbacks.py` draws all 16 startup selections plus a 120-frame,
seven-generation coarse-world recovery route with actual validation enabled.
Full/angled replays of coarse and mixed selections check four compactions, all
raw buffer/image payloads, whole-image identity/depth bounds and independent
nearest-hit ray/colour probes. Shared-profile replay is also included. Capture
validation is disabled; the native cases supply validation evidence.

The all-coarse resident world charges **142040 unique CPU encoded bytes** and
**334368 physical resource bytes**, including **262144 image bytes**. Fine figures
remain 165148/357472/262144. These exclude work arenas, staging, descriptors,
visibility/swapchain targets and driver overhead, and are not total VRAM.

Local ignored evidence: `chunk-fallback-cook/`, `chunk-fallback-source-proof/`,
`chunk-fallback-final/`, `chunk-fallback-tests.log`,
`chunk-fallback-preserved-tests.log` and `chunk-fallback-preserved-source/`.
No runtime Rust or shaders change in this slice; prior compute, Windows and crowd
qualification is not recast as a new fallback-streaming result. M4 remains open.

Next connect protected coarse owners to the actual committed fine-chunk set,
then gate camera-driven requests and complete GPU publication without drawing
both representations of a region. Budget pressure must retain coverage. Terrain
boundary compatibility established here does not establish a general LOD system.

`chunk-fallback-final/report.json` passes **17 native cases** with zero validation
diagnostics and **four replays with 931 ray/colour probes**. Mixed masks 5 and 10
exercise alternating fine/coarse regions; all-coarse replay passes both shader
profiles. Runtime selection remains fixed for each process.

A second full native cook reproduces all eight complete bundle content hashes
exactly (`chunk-fallback-repeat.json`). Updating the source summary's limits text
does not change any rendered resource payload.
