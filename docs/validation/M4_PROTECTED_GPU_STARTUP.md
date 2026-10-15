# Protected coarse GPU startup (2026-10-15)

`--world-covered-test <scene.json>` is a separate fixed-startup qualification
profile. It uploads/adopts four complete coarse v2 graphs first, publishes their
CPU world once, then wraps that world in `ProtectedWorld`. Fine candidates are
attempted in fixed region order. Every successful complete transfer retires before
fine CPU publication; selected graphics/work construction follows startup admission.
There are no prior graphics users at that point. Unselected coarse GPU buffers and
images remain owned alongside the active fine resources until renderer teardown.

A CPU reservation failure from the encoded cap defers that fine candidate. A GPU
physical/image cap failure retires and releases any partial upload, cancels its
CPU reservation and leaves coarse selection intact. Other errors remain fatal.
Coarse bootstrap itself must fit all three caps. An insufficient bootstrap fails
without presenting. All eight input graphs are read/checked synchronously; this
profile does not qualify fallback behavior for missing/corrupt fine files, worker
cancellation, live replacement or camera-driven requests.

The existing four-chunk visibility path draws exactly one selected complete graph
per region with four work arenas/compactions/indirect draws. Inactive coarse sets
own geometry/material/UV/table buffers and images, without additional work arenas.
Selected descriptor arrays contain four storage entries per role and only the
selected images. Frame-local slot tags remain 0..3 regardless of the persistent
fine/coarse chunk IDs. Extra storage indexing features remain opt-in.

| Startup outcome | Unique CPU bytes | Physical resource bytes | Image bytes | CPU epoch |
|---|---:|---:|---:|---:|
| Four coarse + four selected fine | 237988 | 691840 | 524288 | 5 |
| Four coarse + first three selected fine | 213864 | 602320 | 458752 | 4 |
| Four coarse only | 142040 | 334368 | 262144 | 1 |
| Four coarse + varied fine image counts/factors | 272280 | 658768 | 491520 | 5 |

Protected coarse epochs remain `[1,1,1,1]`. Fine selection is `[0,1,2,3]`, mixed
`[0,1,2,7]`, or coarse `[4,5,6,7]`. CPU charges cover unique encoded owners;
physical charges cover retained root buffers/images, excluding four work arenas,
staging, descriptors, visibility/swapchain targets and driver overhead. These are
not total VRAM measurements or scalability results.

## Verification and replay scope

Native plans use actual validation-layer activation and require zero diagnostics
through teardown. They cover exact caps, each one-byte-short full capacity,
coarse-only capacity, each one-byte-short bootstrap, invalid fallback count,
missing shader cleanup, mode conflicts, varied fine resources and shared shaders.
The full recovery route presents 120 frames over seven swapchain generations.
A rejected partial fine upload must not leak into the retained owner ledger.

RenderDoc's `refAllResources` captures live metadata for inactive owners, but
RenderDoc 1.46 does not preserve initial contents of the unused storage buffers.
The oracle checks the exact complete buffer-length multiset plus live image count
for all owners, and exact payloads for the selected graphs. It does not claim
readback of inactive coarse bytes. Separate coarse-selected captures verify those
assets' uploaded payloads. All captures also verify four GPU compactions,
whole-image identity/primitive/depth bounds and independent nearest-hit ray/
material-factor/sRGB colour probes. Reusing retained coarse content after live
fine retirement remains the next gate. No pixel tolerances were expanded.

The helper also replays the original four-chunk mode. There are no shader changes;
the preceding 48-module default/shared profiles remain in use. Captures disable
validation; native routes supply validation evidence. Generated data, binaries
and reports remain ignored under `build/qualification/`.

`world-covered-final-v2/report.json` passes **18 native cases** with zero diagnostics
and **seven replays with 1616 ray/colour probes**. Full protected inventory is 32
root buffers/32 images; the mixed selection retains 28/28, coarse-only 16/16, and
the varied fine fixture retains 32 buffers/30 images. Selected payload checks and
all four compactions pass in each applicable capture. Deferral reasons match the
specific CPU versus GPU/image cap under test. The first replay attempt exposed
the unused-initial-content limitation above; no renderer colour fix or relaxed
oracle tolerance was needed for this profile.

Renderer formatting, denied-warning Clippy and 28 CPU tests pass. Windows GNU
cross-link/Clippy passes eleven packages, with Wine CPU suites passing
(`windows-world-covered/report.json`); native Windows GPU remains unqualified.

The optimized executable passes full recovery, mixed selection, coarse-only
capacity and short-bootstrap rejection with zero diagnostics
(`world-covered-closeout/report.json`). The six preserved upload/cooked-stream/
10000-proxy mixed plans pass (`world-covered-preserved-regressions/report.json`),
and the unchanged headless compute app verifies 256 values again
(`world-covered-original-compute.log`). These remain separate workload claims.
