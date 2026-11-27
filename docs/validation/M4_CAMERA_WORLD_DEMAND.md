# Camera-driven world demand over protected coarse coverage (2026-10-15)

`--world-camera-test <scene.json>` connects the existing benchmark camera rig to
actual committed fine residency in the four-region world fixture. It starts with
four protected coarse graphs (CPU IDs 4..7), then asynchronously requests fine
IDs 0..3. Camera position drives rectangle-distance demand: 20 m load radius,
35 m retention radius, distance/ID priority and one to four fine slots. The
camera period defaults to the frame count; `--world-camera-period` can repeat the
route. `--world-camera-report` writes bounded evidence, including incomplete
failure reports.

One file worker and one pending complete GPU graph are permitted. Each frame
recomputes demand from committed fine IDs. Obsolete requests cancel before ready
results can publish; fine removals precede new requests. A capacity-deferred
region is blocked until the committed world epoch changes, avoiding repeated
reads under unchanged coarse-only capacity. Selected coarse coverage survives
reading, uploading and capacity deferral. Transfer fences, prior frame readers
and the old graphics/presentation generation retire before new descriptors and
matching CPU selection publish. Coarse resources remain owned at bootstrap epoch
1 throughout. Resource destruction releases submitted GPU owners before encoded
CPU reservations.

History retains at most 64 events with an explicit dropped count. Events record
requests, uploads, publications, retirements, cancellations and CPU/GPU deferrals.
Current published graph counts exclude pending graphs; CPU live encoded charges
include complete staged reservations. Peak counters cover current graphs plus
complete pending graphs, excluding partial failed candidates, staging, work
arenas, targets, descriptors and driver overhead. They do not measure total VRAM.

## Observed qualification

Debug native evidence is in `build/qualification/world-camera-final/report.json`:
all **19 native cases passed**. That combined report remains `passed:false`
because its second replay hit an oracle assumption, described below. The
corrected independent replay gate is
`build/qualification/world-camera-replays-v2/report.json`, which passes **nine
captures and 1534 ray/colour probes**. Reports and logs are ignored local artifacts.

Native cases cover full/one/four fine slots, varied material/image graphs, shared
shader modules, independent coarse-only CPU/physical/image caps, WSI recovery,
repeated traversal, rapid turns, early shutdown, five pre-Vulkan option conflicts,
changed immutable input and failed graphics-generation construction. Successful
runs activate validation; all native runtime cases, including controlled failures,
finish with zero validation warnings/errors. Failure reports stay incomplete and
do not falsely acknowledge fine publication. The 1320-frame repeated route
publishes 88 fine graphs and retires 86, retains 64 history rows and drops 287.
Coarse-only cases publish no fine graphs and issue at most four requests.

Replays cover two route positions, varied material factors/two-image fine region,
shared modules, three independent coarse-only caps, obsolete ready-upload
cancellation and an upload still pending at capture. The independent oracle
checks real-resource hash unions, closest-point distance/retention priority,
selection, epochs, CPU/GPU/image coexistence charges and bounded history. Replay
checks selected raw payloads, exact live buffer/image inventory, four independent
GPU compactions, indirect commands, frame identity/depth bounds and nearest-hit
material/colour probes. Pending capture inventory includes destination and staging
resources; unused retained payload contents are not claimed from RenderDoc.

The route can cull one entire region. RenderDoc records four indirect-count parent
calls but omits a mesh-dispatch child for zero work. The old oracle incorrectly
required four nonempty subdraws. It now checks all four parent calls, verifies each
GPU indirect command/count against compaction, and requires exactly one child per
nonempty compaction. Replaying the same failed capture passes with unchanged
image/depth tolerances. No renderer or shader workaround was needed.

Renderer 28 unit tests, formatting and denied-warning Clippy pass. Eleven Windows
GNU packages cross-link and pass Clippy; CPU tests pass under Wine. Native Windows
GPU execution remains unqualified. Default/shared SPIR-V modules are unchanged
from the previous qualified gate.

The optimized executable independently passes all **19 native cases**
(`world-camera-release/report.json`), including source/generation failure cleanup,
WSI, repeated and rapid-cancellation cases. Preserved scripted reload passes full
180-frame and ready-pending 110-frame shutdown checks; its legacy replay passes
235 probes (`world-camera-legacy/report.json`). All six upload/error/cooked-stream/
10,000-proxy mixed regression plans pass with zero diagnostics
(`world-camera-regressions/report.json`). The unchanged original headless compute
test verifies all 256 values (`world-camera-compute.log`).

## Scope and remaining gates

This is a four-region correctness profile on RX 7900 XTX/RADV, without benchmark
timing, proxy crowd or HUD integration. It synchronously reads/checks all eight
fine/coarse inputs at startup to derive the small source catalog and bounds;
subsequent workers reread fine inputs. It does not qualify cold-start I/O, a
production asset catalog or throughput/capacity for a large world.

The prototype renders fine whenever that region has committed fine residency.
Independent per-view LOD selection over shared residency remains an architecture
target; this test does not replace that decision. Geometry is the previously
qualified 8 m fine grid and 16 m coarse interiors with matching 8 m perimeters;
this is not general virtualized meshlet LOD or texture-page streaming. Optional
storage-buffer descriptor array features remain confined to this profile and are
not new baseline requirements or an approved production binding policy.

Graphics-generation retirement remains conservative and can stall. Worker join
has no hard I/O cancellation deadline; source/descriptor failures are fatal and
clean up rather than rolling back and retrying. Lost history prefixes cannot be
fully reconstructed; the oracle checks retained events and aggregate accounting.
Timed 10,000-proxy scene integration needs a separate ground-height contract for
mixed selected terrain, followed by replay and timing qualification. No server
movement, combat or networking authority is introduced.
