# Capture-free independent streaming comparison — qualified 2026-10-16

The optional native reference now measures independent cooked streaming alongside
Theia, using the qualified shared frame schedule. It retains each engine's actual
fine/coarse selection rather than forcing Bevy to replay Theia's readiness.
No engine architecture decision or Theia runtime code changes in this gate.

## Executed contract

`run_bevy_comparison.py --independent-streaming` requires `--target-hz`, at least
140 frames and no frozen snapshots. Both engines use the same cooked sources,
logical camera/proxy route, 20 m load / 35 m retain policy, `--fine-slots 1..4`
(default two) and unique encoded CPU cap (`--stream-cpu-budget`, default 64 MiB).
GPU budgets remain backend-specific: Theia tracks actual owned graph/image
allocations; Bevy's adapted logical payload count is not physical VRAM.

The Bevy executable's `--camera-stream --stream-measure` disables all diagnostic
captures and permits optional `--gpu-timings` / `--stats-overlay`. It retains the
qualified real-file worker, prepared-resource/queue acknowledgement, cancellation
and GPU removal before encoded release. Diagnostic camera/probe/replay defaults
remain available separately. Standalone measurement can omit the clock; the
paired launcher requires it so nominal camera-demand rates are shared.

Bevy's streaming panel has eleven rows: title, host FPS, previous CPU/GPU time,
proxy/region counts, selected fine slots, publications/evictions, encoded KiB,
logical KiB and cancellation/deferral counts. It uses the existing Bevy text/UI
renderer. Exported text is checked against every measured frame's state; new
streaming glyph/pixel conformance is not claimed. Replay HUD captures retain the
existing eleven-row ink checks.

All recorded GPU query pairs retain raw ticks and period; warmup affects summaries
without delaying the camera. Bevy's final fine drain occurs outside recorded
samples and queries. Theia ends the route with its observed ownership, then tears
down; final lifetime counters are therefore not compared as throughput totals.

## Readiness and accounting

The report checks protected coverage, publication epochs, per-frame ownership and
caps independently. Theia's total graph allocation includes current, pending and
fixed ground-table bytes. The existing independent Bevy checker also checks demand
priority, retirement and final coarse-only drain. Its allocation oracle models the
qualified eight-graph fixture, not arbitrary production asset layouts.

A common **near-demand** target selects the nearest up-to-slot-count regions
within 20 m, independent of either engine's retained fine residency. Per-frame JSON
and `readiness.csv` retain target regions, selected fine regions, missing regions
and elapsed clock observations. HTML adds missing-detail traces and matching-view
counts. This observation target is distinct from each engine's hysteresis planner.

For each contiguous near-demand window, JSON records first observed fine selection,
delay in logical frames/observed host milliseconds, and an exclusive end frame.
Never-observed fine stays null. Detail already present at window entry has zero
observed delay; this is not evidence of a zero-time load. Windows include warmup;
region-frame summaries exclude it. First-fine observations do not measure file,
upload or queue latency directly, nor guarantee fine detail remains throughout
the window; the per-frame trace retains subsequent misses.

Both constructors prevalidate their eight inputs. Theia runs first, fixture export
reads the sources again, then Bevy runs. OS caches are uncontrolled and likely warm;
there is no cold-cache equivalence claim or production disk-throughput claim.

## Native correctness evidence

Ignored artifacts: `build/qualification/bevy-stream-measure/native-final/`.
`report.json` passes five streaming pairs covering 2040 recorded frames, plus a
separate 240-frame diagnostic camera run and 60-frame replay pair.
All use native Wayland, RX 7900 XTX / RADV, Vulkan and actual validation.

| Pair | Frames per engine | Bevy publications / retirements, including final drain |
|---|---:|---:|
| Zero proxies, two slots | 240 | 16 / 16 |
| 10000 proxies, two slots | 240 | 16 / 16 |
| 10000 proxies, live HUD | 180 | 11 / 11 |
| 10000 proxies, one slot | 180 | 10 / 10 |
| 10000 proxies, encoded cap 142040 bytes | 180 | 0 / 0; four CPU deferrals |

All five pairs keep their requested 60 Hz schedule without a full-period miss.
GPU timestamp arithmetic/order and every frame's ownership/clock/CSV round trip
pass. The HUD case checks 180 exported eleven-row states and previous completed
GPU values. The coarse-only pair records 323 missing near-detail region-frames per
engine and unresolved windows rather than hiding unavailable fine detail.

Three diagnostic and two strict replay captures pass 664 scene-colour probes,
maximum RGB error one byte. The diagnostic verifier enables the existing bounded
canonical proxy-depth exclusion but excludes zero rays here. Replay HUD ink checks
pass. Independent source-ground/motion checks total 380000 proxy poses. Six invalid
measurement/capture/probe combinations reject; no runtime validation diagnostics.

Linux release build, denied-warning Clippy, formatting and twelve Bevy tests pass.
Two Python tests check unresolved-window, reentry and coverage semantics; Python
3.8 syntax checks pass. Generated replay/streaming chart controls execute under a
Node DOM stub, including region units for missing-detail traces.

Supplementary artifacts under `wrapper/` and `supplement/` qualify forwarding of
clock/slot/encoded-budget controls through the stats on/off launcher, default
uncapped capture-free mode without GPU queries/HUD, and FIFO replay metadata.
No new native Windows GPU or optional Bevy Windows qualification is claimed.

## Separate release observations

`build/qualification/bevy-stream-measure/timing/stats-comparison.html` records
four sequential release runs, validation/captures disabled, 10000 proxies, two
slots, Immediate presentation, 60 Hz, 660 frames and 60 warmup frames. GPU queries
remain enabled. No compiler or other qualification workload ran concurrently.
Each engine's off/on analysis retains 600 frames with matching logical time and
selected geometry. All four runs have zero full-period misses.

| Within-engine metric | HUD off mean ms | HUD on mean ms |
|---|---:|---:|
| Theia GPU frame | 2.361953 | 2.361715 |
| Theia GPU resolve | 0.045428 | 0.043501 |
| Bevy GPU render graph | 0.263892 | 0.273408 |
| Bevy CPU main schedule | 0.901442 | 0.982122 |

Each streaming pair has 465/600 measured frames with matching selections. Of 1125
common near-demand region-frames, Theia misses fine detail in 140 and Bevy in 275,
identically in these off/on observations. This exposes the current serial backend
readiness/retirement behavior; it is not an isolated I/O comparison or an engine
ranking. Host intervals remain approximately 16.667 ms because of the requested cap.

GPU scopes differ: Theia covers frame compute/visibility/resolve; Bevy covers its
render graph including enabled UI. Earlier uploads/lifecycle waits are outside
those intervals. Bevy's serial query readback affects host cadence. CPU scopes
also differ: Theia includes synchronous draw/fence/acquire/present; Bevy's main
schedule excludes later extraction/render preparation/submission. Theia replaces
scene shading under its HUD while Bevy draws UI afterwards. Negative/small changes
are not evidence of free UI or a general UI advantage. Worker CPU, cache/order,
GPU clocks, physical memory and differing selected detail constrain interpretation.

## Reproduce

With matching release executables, compiled shaders and native dependencies:

```sh
python3 tools/run_bevy_comparison.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --players 10000 --frames 660 --warmup 60 --route-frames 120 \
  --independent-streaming --target-hz 60 --present-mode immediate \
  --gpu-timings --headless --headless-dri-prime pci-0000_03_00_0 \
  --gpu 7900 --skip-build --output build/streaming-comparison
```

Add `--stats-overlay` for the live panels. `compare_stats_display.py` accepts the
same streaming/clock/slot/CPU controls and runs separate disabled/enabled profiles.
Use `qualify_streaming_comparison.py --scene <scene.json> --headless-dri-prime
<selector> --output <directory>` for the diagnostic matrix above. Validation and
capture checks belong in separate runs from release observations.

This does not add production assets, networking, animation, general UI, per-view
GPU LOD adoption or physical-memory measurements. The earlier private compositor
[device-loss incident](M4_BEVY_CAMERA_STREAM.md) remains unresolved; passing these
sessions does not establish platform stability or fix device-loss cleanup.
