# Retained coarse GPU reactivation (2026-10-15)

`--world-covered-test <scene.json> --world-covered-retire-test` adds a bounded
fine-retirement schedule to the protected startup profile. Selected fine regions
0..3 retire after 20/40/60/80 presented frames. Already-coarse regions are skipped.
The optional `--world-covered-report <json>` records the initial state and at most
four committed transitions; `completed` denotes normal run completion, including
short prefix runs, not a guarantee that all four events have occurred.

After the sole frame fence, the renderer retires/destroys its old graphics and
presentation generation. It moves the retained coarse geometry into that region's
existing work owner and installs the retained coarse material/UV/table/images.
No geometry/image reupload occurs. The old fine complete GPU set and a retained
CPU coverage snapshot stay in a local eviction guard. A complete new graphics
and descriptor generation must build before fine CPU removal publishes. Only
then do old fine GPU owners release, followed by their retained CPU reader.
Fatal generation construction errors stop rendering and tear down safely; this
qualification does not implement rollback to a previous generation.

Protected coarse owners are indexed by region, and work arenas/layouts remain
stable. Each publication still draws four selected regions. A varied fine chunk
with two images returns to its four-image coarse graph, exercising descriptor
array count/texture-base changes. Existing qualified scalar texture-base selection
is retained; no shader changes or language fallback are introduced.

For the base eight-graph fixture the post-publication ledger is:

| Presented-frame trigger | Selected IDs | CPU epoch | Unique CPU bytes | Physical root/image bytes |
|---|---|---:|---:|---:|
| Startup | 0,1,2,3 | 5 | 237988 | 691840 |
| 20 | 4,1,2,3 | 6 | 213992 | 602464 |
| 40 | 4,5,2,3 | 7 | 189976 | 513072 |
| 60 | 4,5,6,3 | 8 | 166164 | 423888 |
| 80 | 4,5,6,7 | 9 | 142040 | 334368 |

All coarse chunk epochs remain 1. The figures exclude work arenas, staging,
targets, descriptors and driver overhead. Temporary eviction owners remain alive
until generation construction/publication finishes; the ledger records after
those owners retire, not every transient allocation or total VRAM.

## Qualification

Native cases cover full, varied and shared-profile retirement, combined WSI
recovery, mixed/coarse startup under pressure and exact run prefixes at 20/21
and 80/81 frames. The prefix pairs prove that retirement occurs on the following
draw after each presented-frame threshold. A controlled missing-shader failure
removes only an owned private shader link after the first successful retirement.
The next generation fails with zero diagnostics and an incomplete report, without
publishing a second CPU retirement.

The oracle checks every history row against unions of real coarse/remaining-fine
encoded resources, expected selection, epochs, protected epochs, owned graph/image
counts and the final coarse allocation ledger. History stays at most five rows.
A normal prefix may finish with some fine regions still selected, as recorded.

Replay captures after each retirement plus varied/shared cases compare actual
published history with the selected graphs. They verify selected raw buffer/image
payloads, all live allocation inventory, four GPU compactions and nearest-hit
identity/depth/material/sRGB colours. A coarse buffer that was previously retained
and undrawn is now used and its contents become available to replay. Across the
post-retirement captures all four coarse graphs are checked after reactivation;
this closes the preceding startup-only inactive-byte gap for these transitions.
Capture validation remains disabled; native tests supply actual validation evidence.

This is a fixed finite retirement sequence with synchronous whole-generation
retirement, not camera demand, asynchronous fine reload or a performance claim.
No proxies or server authority are added. Next qualify fine reload while coarse
coverage remains visible, then connect the CPU demand policy to those GPU
operations and integrate the timed crowd scene. M4 remains open.

`world-retirement-final/report.json` passes **13 native cases** with zero validation
diagnostics, including the deliberate generation failure, and **seven replays
with 1632 ray/colour probes**. Active/retained root buffer and image inventories
fall through 28/24/20/16 as the base fine graphs release; the varied fine fixture
checks its different image counts separately. Protected epochs stay at 1 and
all complete 120-frame schedules end with four coarse graphs at epoch 9 (or the
corresponding smaller epoch when fine startup was capacity-limited).

Renderer formatting, denied-warning Clippy and 28 CPU tests pass. Windows GNU
cross-link/Clippy passes eleven packages, and Wine CPU tests pass
(`windows-world-retirement/report.json`). Native Windows GPU remains unqualified.
The 48-module shader profiles are unchanged by this unit.

The optimized executable passes recovery, a 21-frame prefix, mixed startup and
coarse-only startup, with history/owner checks and zero diagnostics
(`world-retirement-closeout/report.json`). A startup-only varied-resource replay
still passes 230 ray/colour probes with retirement disabled. The six preserved
upload/cooked-stream/10000-proxy mixed plans also pass
(`world-retirement-preserved-regressions/report.json`).
