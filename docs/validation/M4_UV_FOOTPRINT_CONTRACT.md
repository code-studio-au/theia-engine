# M4 analytic UV differential and texture-footprint CPU contract (2026-10-11)

`theia_assets::texture_footprint` now defines a bounded CPU reference for perspective-
correct authored UV gradients and an explicit prototype mip policy. GPU implementation
is outstanding at this checkpoint. Existing fixed-mip rendering is preserved.

Inputs are three represented f32 clip positions and UVs, a framebuffer pixel-center
coordinate and extent. The profile bounds magnitudes to 64, extent to 1..16384,
positive W >=1e-4, wholly near/far-unclipped vertices, nondegenerate projected area
and positive interpolation denominator. Invalid/nonfinite/unbounded values return
errors. The caller identifies the visible triangle; this function is not a coverage
or clipped raster oracle and does not authenticate geometry/material provenance.

Linear screen-space barycentric weights and their per-pixel derivatives interpolate
`UV/W` and `1/W`. With numerator q and denominator r, the analytic derivative is
`(dq - UV * dr) / r`. Computation uses f64 over represented f32 input values. Gradients
remain unwrapped; differentiating wrapped vertex UVs would introduce false jumps at
repeat seams. This is a mathematical CPU model, not proof of shader f32 arithmetic.

The separate mip policy uses the maximum length of the Jacobian's X/Y columns scaled
by base texture dimensions. It chooses ceil(log2(max(rho,1))), clamped to the complete
resident chain. Exact power-of-two comparisons avoid a log2 rounding dependency.
Texture dimensions must fit 1..1024 and the supplied chain count must be complete.
This isotropic footprint estimate is not anisotropic filtering, reconstruction
quality, a producer mip-filter guarantee or tiled/partial fallback policy.

Four new tests qualify known affine gradients and resolution scaling, all six
vertex/winding permutations, 1000 generated perspective cases against independent
finite-difference interpolation, exact mip thresholds/clamping/invalid profiles,
repeat-seam behavior and constant UVs. The seam test selects mip 2 with unwrapped
gradients versus the incorrect mip 4 after wrapping vertex coordinates.
All 50 asset tests and 23 renderer tests pass; formatting/denied-warning Clippy pass.

Render Lab `--texture-footprint-reference <mesh uv texture x y width height view>`
connects the model to immutable cooked geometry/UV and the complete mip chain,
without Vulkan. It uses the visible triangle from the existing CPU raster oracle;
background returns no gradient. On the preceding 960x540/view-2 imported scene,
three sets of 85 framebuffer probes match prior coverage/identity and UV values
within 2e-4. Six covered probes per source choose mip 0 for square/odd textures and
mips 4/5 for the eleven-level thin texture. These are CPU choices, not GPU sampling.

All 11 packages cross-link/lint for Windows GNU; Wine executes 141 CPU tests,
including the new numeric cases. Native Windows/GPU/cross-vendor remain unqualified.
No shader changes, new GPU execution, filtering/performance, hosted CI or push claim.

Artifacts: `build/qualification/m4-uv-footprint/` per-source oracle probes/report;
`m4-uv-footprint-{assets-tests,assets-clippy,renderer-tests,renderer-clippy,build}.log`;
`windows-uv-footprint/`. The CLI may print asset diagnostics before its final JSON
record, consistent with the existing LOD reference helper.

Next add analytic shader gradients with the same per-primitive data and qualify
GPU automatic mip selection against these independent reference points. Maintain
fixed-view/legacy paths and record unsupported-profile fallback separately.
M4's broader hierarchy, derivative/filter quality and tiled residency gates stay open.
