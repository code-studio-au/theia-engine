# Frame timings for the camera-streamed crowd (2026-10-15)

`--world-camera-timings <json>` adds the existing finite timestamp recorder to
`--world-camera-test <scene.json> --world-camera-crowd <0..10000>`.
`--world-camera-warmup <frames>` excludes the initial samples from statistics
without delaying camera motion, demand or proxy simulation. Warmup defaults to
zero; at least three measured frames are required. Timing and camera reports must
use different paths. Injected WSI recovery is qualified in the untimed profile
and cannot combine with timing recording.

The new report workload is `theia-streamed-mmo-v1`. Its `scene_time` follows the
camera's 60-second route and repeats at `camera_route_frames`; it does not use the
older standalone benchmark's delayed warmup clock. `logical_duration_seconds`
denotes one route period here. Each sample includes a `world` snapshot with
committed selection/epoch, protected epochs, publications/retirements/deferrals,
CPU encoded ownership, current graph/image owners, pending uploads, the fixed
query-table charge, total current/pending/table bytes and route cycle. Snapshots
are taken after publication and before recording that frame's GPU work; pending
fence readiness describes that CPU observation.

## What the timings measure

Four timestamps partition **frame compute**, **visibility/depth** and **material
resolve**, using the existing single graphics/compute queue recorder. They cover
four world compactions and GPU proxy generation/draws. They exclude upload
submissions and graphics-generation retirement/rebuild before frame recording.
`cpu_draw_ms` is draw wall time, including fence/acquire/present and synchronous
poll/upload/rebuild costs, rather than background worker CPU usage. Frame interval
includes event-loop gaps. FIFO, one frame in flight, instrumentation, driver,
validation and build context remain in the report.

The tracked work/target metric sums all four world work arenas, proxy work and
owned offscreen targets. It excludes graph/query-table owners, upload staging,
swapchain images and driver allocations. The separate per-frame world ledger
includes current graphs, complete pending graph allocations and the persistent
terrain table, with image bytes as a subset rather than an additional charge.
It does not measure total VRAM or CPU process RSS.

Only a normally completed finite route writes a timing report. Controlled source,
bootstrap and graphics-generation failures leave no completed timing file. No GPU
visibility/count readback controls work sizing; completed query readback supplies
timing data and the CPU already owns residency metadata.

## Observed qualification

`world-timings-final/report.json` passes **17 native cases**. Runtime cases activate
validation and finish with zero warnings/errors, including expected failures. Successful runs
cover full/zero proxies, one-fine/coarse-only budgets, shared modules, repeated
routes, fast turns and short normal shutdown. Invalid/missing timing modes,
excessive warmup, identical output paths and incompatible WSI injection fail
before Vulkan. Bootstrap, changed source and failed graphics rebuild do not
produce completed timing files.

Three fresh replays pass **580 ray/colour probes**, all GPU proxy poses/ground
queries, world compactions and timing snapshots: full route, repeated route and
zero proxies. Capture validation is disabled; native cases provide validation
proof. No shader changes or tolerance expansion occur in this timing slice.

The independent oracle checks every frame's clock, warmup flag, ownership/caps,
selection/counter/epoch changes and eligible publications/retirements. It checks
prior pending ownership before fine acknowledgement, current/pending/
fixed accounting, final agreement with the separate camera report, finite stage
times, their sum, and exact mean/percentile/max summaries over measured samples.
The 360-frame/120-period replay checks 330 measured samples and 46 mutations;
the 240-frame/10-warmup route checks 230 measured samples and 14 mutations.

The optimized executable independently passes all 17 native cases
(`world-timings-release/report.json`). Renderer formatting, denied-warning Clippy
and 30 CPU tests pass. Eleven Windows GNU packages cross-link/pass Clippy and CPU
suites pass under Wine (`windows-world-timings/report.json`). All six preserved
upload/error/cooked-stream/10000-proxy mixed regression plans still pass with zero
native diagnostics (`world-timings-preserved-regressions/report.json`). Shader
modules are unchanged from the prior 51-module qualified gate.

## Longer observed run

`world-timings-long-release/report.json` passes a **3660-frame** optimized native
Wayland run with **3600 measured samples**, 10000 proxies, no HUD and validation
explicitly disabled. It checks every timing/residency sample and 14 world
mutations; peak current/pending/table bytes stay at 553984. This is one local
observation on RX 7900 XTX/RADV, 960x540, private headless/GL compositor, FIFO and
one frame in flight. Runtime validation evidence comes from the shorter native
cases above, not from this disabled-validation run.

| Metric (ms) | Mean | p95 | Max |
|---|---:|---:|---:|
| Presented-frame interval | 25.105 | 25.551 | 26.301 |
| CPU draw wall | 25.092 | 25.539 | 26.287 |
| GPU frame work | 2.336 | 2.405 | 2.454 |

These intervals describe this display/synchronization/instrumentation setup. GPU
frame work excludes earlier streaming transfers/rebuilds, and must not be
converted into a claim of unconstrained client FPS or networked-player capacity.
The logical 60-second route is frame-driven; its wall duration follows presented
frame intervals. Source data is prevalidated and mock proxies are unskinned.

## Scope and next gate

The live HUD is still separate work. These checks qualify report correctness;
they do not establish a production performance target or total streaming GPU
cost. The fixture prevalidates content, retains a small all-level terrain table,
uses mock unskinned proxies and conservatively rebuilds graphics generations.
Independent per-view LOD, larger catalogs, full geometry/texture virtualization
and server/network player scale remain open. Native Windows GPU is unqualified.
