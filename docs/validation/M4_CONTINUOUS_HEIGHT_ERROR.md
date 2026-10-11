# M4 continuous heightfield LOD error evidence (2026-10-11)

Executed with the pinned meshoptimizer 1.3 offline producer and RX 7900 XTX/RADV
26.2.4 viewer. No hosted CI, Windows GPU or performance qualification.

The new offline verifier treats canonical represented f32 positions as exact
rationals. Each bounded surface must continuously cover its rectangular XY
bounds with no overlapping interiors and equal shared-boundary heights. Exact
convex triangle overlays partition the fine/coarse surfaces; their height
difference is affine per polygon, so maximum absolute vertical error occurs at
an overlay vertex. The exact maximum fraction gates admission. Its floating
report rounds upward. This does not bound 3D Hausdorff distance, silhouette,
normals/materials or runtime floating-point projection.

| Result | Bytes | Triangles / meshlets | Continuous vertical upper bound | SHA-256 |
| --- | ---: | --- | ---: | --- |
| Terrain ratio .5 | 6704 | 256 / 4 | 0.004977348517231262 | `ce244cd3d1ef53d11c43c5a301d93f651f8d27d8e30f0b21fc76c4e24d0d1dbb` |
| Terrain ratio .25 | 5976 | 128 / 2 | 0.012254270974889582 | `1736ab5476264c00eca81ea78a56f142a87935a3b81e2bc85931bbcd6344430f` |
| Flat grid ratio .5 | 6708 | 256 / 4 | 0 | `36f7bacc05435a0cb8f7ec096db250bb3216b9881e24e1a8b809cc0b8bc1dd28` |

The ratio-.5 exact error is 280199815769/56294996180992. Its overlay checks
3771 triangle pairs and 6247 polygon vertices. Boundary locking retains 64 exact
boundary edges. Native absolute error is independently reported as
0.003891846165060997; sampled vertical maximum is 0.004977348433142348. The
continuous gate rejects a 0.00497734848 threshold which passes the sampled gate.
These differences are tiny and are not a performance or visible-quality claim.
Repeated ratio-.5 cooking produces identical reports and content identities.

All 18 Python tests pass. Five new tests compare a non-source diagonal crossing
extremum and 100 seeded random corner-height combinations against an independent
analytic center formula; identity/flat/center-peak cases, compensated holes and
overlap, discontinuous shared boundaries, wrong winding, bad indices/counts,
nonfinite data and invalid limits are covered. Native terrain ratios .5/.25 and
flat .5 all pass complete Rust mesh admission. The original unsimplified
grid/quad/triangle/full-terrain hashes are unchanged. Binary schema is unchanged;
LOD metadata and its content identities change deliberately.

All three outputs execute 120 frames/ten generations with synchronization
validation and zero diagnostics. Separate ratio-.5/.25 captures present 180
frames and pass 81 depth/diagnostic-material samples plus exact immutable GPU
bytes and work/meshlet checks. They do not claim one probe per triangle.
Preceding depth/RGB tolerances apply; capture validation remains disabled.

Artifacts under ignored `build/qualification/`:

- `m4-continuous-height-{cook,coarse,flat,repeat}.json`.
- `m4-continuous-height-{tests,reject}.log`.
- `m4-continuous-height-{cook,coarse,flat}-{reader,smoke}.log`.
- `m4-continuous-height-{cook,coarse}/report.json`, capture logs/images.

This is a qualified offline producer's bound, not a runtime verifier or trust
certificate for arbitrary metadata. General hierarchy/seams, nonrectangular or
3D geometry, projected-error selection and virtual texture work remain open.
