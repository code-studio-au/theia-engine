# M4 projected heightfield LOD CPU evidence (2026-10-11)

No Vulkan/GPU LOD selection is performed by this unit. The preceding continuous
heightfield producer supplies the declared bound; the CPU CLI does not recompute
or authenticate that proof. See [continuous error](M4_CONTINUOUS_HEIGHT_ERROR.md).

The oracle computes an outward-rounded projected correspondence bound over a
common world AABB. It accounts for W changes and both pixel axes, guards Vulkan
near/far/nonpositive-W boundaries, rounds producer errors upward to f32, and
keeps fine LOD on finite-input numeric overflow or exhausted pixel budget.
The correspondence assumption is vertical-only, common XY, with both endpoints
inside the AABB. No shader rounding, depth/disocclusion, silhouette/material,
3D Hausdorff, hierarchy, hysteresis or streaming-demand guarantee is implied.

All 39 asset tests pass. Six new tests cover analytic orthographic displacement,
zero error, perspective denominator changes with zero axis-Z coefficient, clip
and malformed profile guards, finite extreme overflow, upward f32 conversion
including subnormal cases, and 3000 seeded independent point-projection samples.
Doubling extent doubles the bound in those sampled cases. Renderer tests (14),
formatting and denied-warning Clippy for both packages pass.

The source-linked full terrain and preceding ratio .5/.25 pages produce:

| Coarse triangles | Extent | View | Pixel budget | X/Y bound | Choice |
| ---: | --- | ---: | ---: | --- | --- |
| 256 | 960x540 | 2 | 1 | 0.63234 / 2.04430 | Fine |
| 256 | 960x540 | 2 | 10 | 0.63234 / 2.04430 | Coarse |
| 256 | 1920x1080 | 2 | 10 | 1.26469 / 4.08859 | Coarse |
| 256 | 960x540 | 1 | 10 | 3.75277 / 6.69367 | Coarse |
| 128 | 960x540 | 2 | 1 | 1.55683 / 5.03307 | Fine |
| 128 | 960x540 | 2 | 10 | 1.55683 / 5.03307 | Coarse |
| 128 | 1920x1080 | 2 | 10 | 3.11367 / 10.06614 | Fine |
| 128 | 960x540 | 1 | 10 | 9.23935 / 16.47987 | Fine |

Flat terrain's zero bound chooses coarse at a zero pixel budget. A freshly
hashed source-link mutation, foreign fine geometry and negative pixel budget
fail before Vulkan initialization (tested with empty DISPLAY). Original files
remain untouched. The CLI verifies exact common position bits, triangle counts,
source hash link and boundary/profile metadata; it consumes the declared error
rather than treating arbitrary metadata as a verified proof.

Artifacts under ignored `build/qualification/`:

- `m4-projected-lod-tests.log` (39 asset tests).
- `m4-projected-lod-renderer-tests.log` (14 renderer tests).
- `m4-projected-lod-real.json` (eight camera/extent/budget results).
- `m4-projected-lod-flat.json` (zero error/budget).
- `m4-projected-lod-negative/{foreign-source,wrong-fine,negative-budget}.log`.

CI now includes a CPU CLI qualification step; hosted CI was not executed.
Windows/native GPU and performance are unqualified. Next add GPU page choice
and compare work/identity/framebuffer against this CPU model.
