# Streamed four-region world with GPU-grounded proxies (2026-10-15)

`--world-camera-test <scene.json> --world-camera-crowd <0..10000>` combines the
protected camera scheduler with dense moving GPU visual proxies. The camera,
proxy motion and fine/coarse selection mask share the same route frame. The mask
comes from committed CPU world selection after graphics-generation publication;
pending uploads never enable fine-ground queries early.

A persistent readonly terrain table contains the eight CPU-qualified fine/coarse
ground surfaces. GPU proxy compute generates XY positions, chooses the committed
surface, queries its direct-grid triangle candidates, writes complete poses and
performs conservative frustum/detail compaction. A failed height query writes a
nonfinite pose that replay rejects, rather than silently substituting a different
terrain function. Existing near/far proxy geometry uses the same 35 m detail rule.
There are no per-proxy host updates, allocations or draw calls in the frame path.

Four independently compacted world draws and one proxy draw share visibility and
depth. Region tags occupy bits 29..30; bit 31 distinguishes frame-local proxy IDs.
The combined material resolve keeps the four-world texture/factor path and shades
proxy identities separately. World draws finish before binding the proxy pipeline,
so subsequent region work cannot accidentally use the proxy pipeline. The scene
keeps its 16 fragment storage descriptors; no extra fragment storage binding is
needed for deterministic proxy colours. Optional storage-array features remain
isolated to this profile, not new baseline requirements.

## Ownership and accounting

The **40704-byte** RADV terrain allocation is deducted from the physical budget
before coarse bootstrap, fine admission or subsequent pending transfers. Camera
reports retain the existing graph-only `peak_gpu_bytes` and add `fixed_gpu_bytes`
and `peak_total_gpu_bytes`; the latter includes the persistent table. Fine demand
sees the remaining budget. The exact all-coarse physical cap is therefore
**375072 bytes**, including 334368 graph bytes. One byte below cannot bootstrap
complete coverage and fails with clean teardown.

The table outlives the proxy descriptor/pipeline owner and all submitted graphics
users. It retains ground metadata for both levels even when fine render graphs
are not resident; this is an explicit small-fixture cost, not virtualized query
streaming. The **376160-byte** proxy work allocation, four world work arenas,
render targets, upload staging, descriptors and driver overhead remain outside
the graph-plus-table cap. CPU encoded ownership budgets do not measure startup
catalog reads, generated table metadata or process RSS.

## Qualification

`world-crowd-final/report.json` passes **23 native cases** with actual validation
activation and zero warnings/errors, including controlled failures. Cases cover
0/1/1000/10000 proxies, one/four fine slots, varied texture/material graphs,
shared shaders, independent coarse-only CPU/physical/image caps, WSI recovery,
1320-frame repeated traversal, rapid turns, early shutdown, one-byte-short table
and coarse bootstrap budgets, option conflicts and source/rebuild failure cleanup.
Failure reports never falsely acknowledge fine publication.

The same report passes **eight replays and 1581 ray/colour probes**: two route
positions, varied palette/factors, shared shaders, coarse-only terrain, obsolete
ready cancellation, a captured pending upload and zero proxies. Each replay checks
all generated proxy records, selected-ground heights against independent source
triangles, exact table geometry/CSR contents, culling, near/far counts and identity/
depth bounds. Combined probes include **368 world-front / 184 proxy-front**
occlusion cases. Maximum sampled ground error is **1.44e-7 m**, with maximum XY
motion error **0.0000554 m**, within the previously qualified motion tolerance.
No depth/colour/motion tolerance expansion was needed.

At route frame 180, all 10000 proxies survive culling: 4995 near and 5005 far.
The repeated route performs **88 fine publications / 86 retirements**, retaining
64 history rows and dropping 287. Peak graph bytes are 513280; adding the stable
40704-byte table gives **553984 physical graph-plus-table bytes**. Coarse epochs
remain pinned at 1 and all committed/pending caps match the independent oracle.

`world-crowd-release/report.json` independently passes all 23 native cases using
the optimized executable. Both shader profiles compile/validate **51 SPIR-V
modules**. Renderer 30 CPU tests, formatting and denied-warning Clippy pass.
Eleven Windows GNU packages cross-link/pass Clippy; CPU suites pass under Wine
(`windows-world-crowd/report.json`). GPU replay captures disable validation;
native logs provide the actual validation evidence. Generated logs/captures are
ignored under `build/qualification/`.

The preserved scripted reload passes full 180-frame and ready-pending 110-frame
shutdown checks plus a fresh legacy replay with 235 probes
(`world-crowd-legacy-reload/report.json`). The standalone terrain query still
passes all ten cases after its shader function was shared with proxy compute
(`world-crowd-ground-regression/report.json`). All six upload/error/cooked-stream/
10000-proxy mixed regression plans pass with zero diagnostics
(`world-crowd-preserved-regressions/report.json`). The original headless compute
probe again verifies all 256 values (`world-crowd-compute.log`).

## Scope

This profile is a headless-qualified correctness route, without timed benchmark
recording or HUD integration. The older whole-world timed mixed benchmark remains
separate. Proxy contact samples the ground beneath the center of each mock box;
there is no foot IK, slope-conforming animation or collision simulation. Selection
changes can move the contact height by the previously bounded fine/coarse terrain
difference; no morphing or temporal smoothing is claimed.

Startup still prevalidates all eight graphs and derives all terrain metadata.
Graphics publication conservatively retires complete generations; source and
rebuild failures remain fatal. Independent per-view LOD, larger source catalogs,
virtual geometry/textures and production streaming throughput remain open. GPU
proxies are visual only and provide no server movement/combat authority or proof
of networked player capacity. Native Windows GPU remains unqualified.
