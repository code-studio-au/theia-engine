# M4 bounded CPU outdoor chunk demand (2026-10-15)

The stateless `world_demand` module and complete-bundle example qualify the next
CPU prerequisite to world streaming. No renderer, shader or binary format changes.

Asset tests pass **67**. Four new demand tests cover inclusive edge/corner radii,
retention using actual residency, failed loads not becoming residents, zero radius,
nearest priority/stable-ID ties, pressure eviction/deferral, invalid catalogs and
queries, opposite extreme finite coordinates, and 2000 deterministic randomized
queries against exhaustive integer points inside rectangular chunk bounds.
Formatting and all-target warning-denied Clippy pass.

`tools/qualify_world_demand.py` passes four **686-sample** CPU routes using the
checked four-quadrant cooked scene. Each contains 240 eyes emitted by the actual
Render Lab benchmark CPU reference plus 446 spatial/boundary excursions. Geometry
bounds come from admitted cooked positions. Per sample an independent closest-
point model checks desired/load/removal/deferred IDs, actual resident IDs, world
epoch and the unique complete-bundle encoded-byte union after CPU publication.
Source resource hashes are checked and the scene manifest hash is recorded.

| Load radius | Retain radius | Slots | Transactions | Deferred candidates | Retention-band candidates |
|---|---|---|---|---|---|
| 20 | 35 | 4 | 20 | 0 | 205 |
| 20 | 35 | 2 | 19 | 253 | 82 |
| 20 | 20 | 2 | 21 | 250 | 0 |
| 0 | 0 | 1 | 17 | 320 | 0 |

The byte cap is 165148, enough for the union of all fixture resources including
coexisting staged and active CPU sets. Final far-away samples publish no chunks
and charge zero bytes. The example parses/validates the bounded entire XY route
before publication. It performs synchronous local reads, not background streaming.

Evidence under ignored `build/qualification/`: `world-demand-final/report.json`,
its four JSONL traces/route, `world-demand-tests.log`, `world-demand-clippy.log`
and `world-demand-example-build.log`. These are CPU correctness observations,
not GPU executions, performance scores or network capacity claims.

Windows GNU cross-link/Clippy passes eleven packages, and Wine executes the
CPU test suite including all 67 asset tests. See
`windows-world-demand/report.json`; native Windows/GPU remains unqualified.

Distance-only XY demand deliberately ignores altitude, camera direction and
occlusion. It can request offscreen chunks or omit far visible content. It supplies
no mandatory coarse fallback, pending queue, byte-aware admission or frame/upload
completion. Those must be integrated and qualified before executing this policy
in a renderer. Slot pressure can evict retention-eligible chunks; no pinning or
zero-churn guarantee is made. The mixed benchmark remains one resident whole-world
bundle with visual proxies. M4 stays open.
