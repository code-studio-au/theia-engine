# M4 GPU resident heightfield LOD evidence (2026-10-11)

Executed on RX 7900 XTX/RADV 26.2.4, Vulkan loader/validation 1.3.275, Slang
2026.19 and RenderDoc 1.46. No hosted CI, Windows GPU or performance claim.

The source-linked full terrain and continuous-qualified coarse pages remain
resident at fixed storage bindings. Compute selects a page, culls its meshlets
and emits indirect command/counts/IDs. Mesh loads use the same GPU selector.
No live host selection/count readback occurs. Compute/mesh declarations have
separate 112/64-byte push blocks. Both default/shared shader variants compile
and SPIR-V validate 29 outputs; the preceding specific GLSL primitive-interface
fallback remains unchanged.

The f32 policy uses absolute/relative margins and a bounded input profile (matrix
and box magnitudes <=64, extent <=16384, error zero or 1e-6..1). Unsupported
numeric profiles and clip boundaries retain fine. Cases compare GPU projected
bounds to the conservative CPU model and forbid a coarse choice when CPU requires
fine. This does not prove all possible shader/raster arithmetic or vendors.

| Replay case | Selected page | Reason | GPU X/Y bound | Pixel samples |
| --- | --- | --- | --- | ---: |
| Terrain budget 6 | Coarse 128 triangles | Coarse | 1.55801 / 5.03462 | 81 |
| Terrain budget 1 | Fine 512 triangles | Pixel budget | 1.55801 / 5.03462 | 81 |
| Narrow budget 6 | Fine | Pixel budget | 9.24140 / 16.48266 | 81 |
| Translated empty budget 6 | Fine | Pixel budget | 40.48334 / 5.03462 | 81 |
| Translated empty budget 100 | Coarse | Coarse | 40.48334 / 5.03462 | 81 |
| Flat budget 0 | Coarse 256 triangles | Coarse | 0 / 0 | 81 |
| Terrain intermediate | Coarse 256 triangles | Coarse | 0.63342 / 2.04552 | 81 |
| Budget 5.033072 | Fine | GPU numerical margin | 1.55801 / 5.03462 | 81 |
| Shared shader directory | Coarse 128 | Coarse | 1.55801 / 5.03462 | 81 |
| Far guard fixture | Fine | Clip boundary | Not applicable | 81 |
| Near-crossing fixture | Fine | Clip boundary | Not applicable | **0; work only** |
| Tiny flat bound fixture | Fine | Unsupported profile | Not applicable | 81 |

Each of twelve separate captures presents 180 frames, verifies both exact GPU
page bytes, selected page/header, compact culling IDs/mask and indirect command.
Eleven verify depth/gradient colour at the preceding tolerances. Near-crossing
replay checks work/page choice only, explicitly `lod_framebuffer_verified=false`;
the CPU oracle cannot rasterize clipped triangles. A conservative threshold
keeps fine even though the raw CPU model permits coarse. Empty culling works for
both selected levels independently of LOD eligibility.

Guard fixtures are test-only offline position rewrites with original native
provenance nested and exact continuous error/boundary checks rerun. Far/near
translate Y by +32/-1.5. The tiny flat fixture raises the true zero-error bound
to a valid conservative 1e-8 to exercise numeric-profile fallback. No new native
simplification error is claimed for rewritten positions. Original assets stay
unchanged. This fixture builder is not a production authoring/streaming pipeline.

Nine LOD configurations execute standalone 120-frame/ten-generation WSI and
synchronization validation with zero diagnostics: default, budget 1, flat,
intermediate, far, near, numeric guard, shared directory and exact physical cap.
The terrain pair charges 7248+5984=13232 physical geometry bytes. Exact cap
13232 succeeds; 13231 rejects the second allocation after the completed fine
upload and tears down without diagnostics. Work/staging/targets/descriptors/
pools are excluded; staging keeps existing individual bounds. The default
combined geometry cap is 4 MiB. This path requires both resident pages.

Sixteen existing modes each execute 120-frame WSI validation without diagnostics:
clear, mesh, task, indirect, spatial, visibility, asset, camera, cull, diagnostic
material, factors, UV, v1, v2, complete-bundle streaming and geometry cell
streaming. Shared LOD is counted above. Bundle-stream replay also passes 85
samples at the explicitly selected alias epoch. Headless compute verifies all
256 values. Renderer/assets tests pass 14/39, formatting and denied-warning
Clippy pass. Four malformed option/source cases fail before Vulkan. Missing
mesh shader after both uploads/compute pipeline cleans up without diagnostics.

Artifacts under ignored `build/qualification/`:

- `m4-gpu-lod-{shaders,shared-shaders,renderer-tests,assets-tests,headless}.log`.
- LOD smoke/budget logs, `m4-gpu-lod-regression-*.log` and negative logs.
- `m4-gpu-lod-{coarse,fine,narrow,empty,empty-coarse,flat,mid,conservative-threshold,shared}/report.json` with captures/images.
- Guard JSON/source files, smoke logs and `m4-gpu-lod-guard-{far,near,profile}-capture/report.json`.
- `m4-gpu-lod-regression-stream/report.json` (85 samples).

General meshlet hierarchy, streamed LOD/page demand, multi-frame retirement,
hysteresis, 3D/material/silhouette errors, clipped framebuffer reference and
virtual texture work remain open. This qualifies a bounded two-level resident
heightfield slice and does not close M4.
