# Optional Bevy comparison: presentation modes and GPU intervals

Locally qualified on 2026-10-16: Radeon RX 7900 XTX / Linux RADV / private
headless Weston native Wayland, Bevy 0.19.1 and locked wgpu 29.0.4. This extends
the optional reference tool; it changes no locked engine architecture or product
runtime dependency. Equivalent streaming remains outstanding.

## Contract

Both launchers accept `--present-mode fifo|immediate|mailbox`, default `fifo`.
Render Lab's corresponding direct option is `--benchmark-present-mode`, restricted
to timed workloads. Theia queries support on every swapchain construction and
passes the exact requested Vulkan mode, including graphics-generation rebuilds.
Unsupported requests fail rather than substituting FIFO.

The reference queries the same native window/adapter through a temporary wgpu
surface in its render world, retaining the window throughout the query. Pinned
Bevy's surface selector takes the explicit mode when supported; unsupported
requests fail our check, and the runner also rejects fallback diagnostics. Its
actual configured surface is private to Bevy: report evidence describes this
capability/selection proof, rather than claiming direct configuration readback.
Driver overrides and other environments need separate qualification.

Reports and HTML identify the requested/verified mode instead of a fixed FIFO
label. Immediate removes vsync pacing in this tested setup. Mailbox is another
non-vsynced submission policy, with different frame-discard semantics. Neither
mode promises freedom from compositor pacing on other systems. Both references
retain their single-frame baseline; this is not a swapchain-free throughput test.

`--gpu-timings` additionally enables Bevy timestamp queries inside encoders.
Two timestamps bracket its render-graph render systems, after Begin and before
Submit. This covers renderer compute, raster, postprocess and enabled UI, but not
preceding asset uploads, host extraction/encoding, presentation or readback.
Theia retains its existing compute/visibility/resolve ALL_COMMANDS interval.
These are corresponding renderer intervals with different internal pipelines,
not identical per-pass work or a total-engine score.

Extraction tags the just-updated logical frame after Last increments the host
tick. Every recorded frame must receive exactly one query pair; duplicate,
missing, reversed, nonfinite and nonpositive results fail. Raw ticks, timestamp
periods and derived milliseconds are exported alongside warmup-filtered summaries
and CSV. Finalization holds the last pose for two unmeasured host updates so the
final recorded pose renders and its results retire before report publication.

The optional Bevy collector serially maps and waits for completion after render
submission, using one reusable query set and two 16-byte buffers. The completion
wait/readback is outside its GPU interval and inside host cadence. Theia's
existing synchronous draw/fence/acquire/present CPU metric differs from Bevy's
main-schedule CPU metric. No ratios between those CPU scopes are reported.
Disabled collection creates no GPU query buffers or timestamp/readback systems.
The Bevy stats panel shows previous completed GPU milliseconds when available,
otherwise `N/A`. Frozen visual captures remain separate, without GPU collection.

## Executed correctness evidence

Ignored local artifacts under `build/qualification/`:

- `bevy-uncapped-final/comparison.json`: Immediate + GPU queries, actual validation,
  0/1000/10000 proxies, 180 frames each, warmup 10, pre-roll 60, 1280x720. All
  540 logical GPU frames have valid raw ticks. Twelve frozen captures at
  0/60/120/179 pass 1462 source ray/colour probes, maximum RGB error one byte.
  Independent pose checks cover 7000/70000 proxies across fine/coarse terrain
  and near/far detail, maximum ground error below 1.71e-7 metres.
- `bevy-uncapped-fifo/comparison.json`: preserved FIFO, GPU queries disabled,
  10000 proxies, 90 frames, validation, HUD enabled. Two captures pass all eleven
  rows and 242 scene probes outside the panel. Query fields are absent.
- `bevy-uncapped-mailbox/comparison.json`: Mailbox + GPU queries, zero proxies,
  60 frames, validation. All 60 query pairs pass. All three modes are supported
  by this target; unsupported-surface failure is not exercised on real hardware.
- `bevy-uncapped-negative/report.json`: three invalid/unscoped argument plans
  fail before Vulkan; interactive HTML host/GPU metric changes and hover execute
  under Node with finite plotted values.
- `bevy-uncapped-timing/statistics-proof.json`: exact frame coverage, raw tick
  arithmetic, CSV, warmup quantiles and matched-control arithmetic all pass.
- `bevy-present-{build,clippy,tests}.log`,
  `theia-present-{build,clippy,tests}.log`: release builds, denied-warning Clippy,
  10 reference and 30 renderer tests pass. Rust formatting and changed Python's
  Python 3.8 syntax pass. All 57 SPIR-V files present in the existing shader
  directory validate; no shaders changed or new compiler qualification claimed.
- `bevy-uncapped-windows/report.json`: all eleven product packages cross-link,
  pass denied-warning Clippy and Wine CPU tests. The optional Bevy executable
  is outside this matrix; native Windows and Windows GPU remain unqualified.

Corrected native logs contain no VUID, validation-error, ERROR or panic diagnostics.
The benign window-decoration warning remains. Earlier intermediate runs are not
promoted as final evidence. The original headless compute app is unchanged; a fresh native run passes its
256 expected values (`bevy-uncapped-compute.log`).

## Separate performance observations

`bevy-uncapped-timing/stats-comparison.json` / `.html` records four sequential
release runs: each engine, overlay disabled/enabled, Immediate, validation off,
10000 proxies, 660 recorded frames, warmup 60, camera period 600, Bevy pre-roll
120. Builds, other qualification jobs and GPU workloads had completed before
these runs. Each row uses all 600 matching logical camera/selection frames.

| Metric, mean milliseconds | Overlay off | Overlay on |
|---|---:|---:|
| Theia GPU compute/visibility/resolve | 1.228540 | 1.229444 |
| Bevy GPU render graph | 0.198346 | 0.207626 |
| Theia host interval | 1.606925 | 1.621885 |
| Bevy host interval | 2.838109 | 2.928020 |
| Theia CPU draw wall | 1.595700 | 1.610768 |
| Bevy CPU main schedule | 0.704520 | 0.776136 |

These are observations of this small synthetic scene, renderer configuration,
run order and clock state. They do not establish an engine-wide winner. Bevy's
shorter GPU range coexists with a longer host interval; the differently scoped
CPU rows cannot explain that difference by subtraction. GPU readback completion
also affects Bevy cadence. No repeated-trial confidence or offscreen saturation
claim is made.

Theia's HUD replaces scene resolve under the panel; Bevy draws UI after the
scene. Fonts, available metrics and shader profiles differ. In this optional
profile, enabled GPU instrumentation supplies an additional live Bevy statistic,
so its panel is not the exact historical `N/A` glyph workload. This is neither an
isolated font cost nor a general UI comparison.

## Reproduce

Build both matching release binaries first (see the reference README for this
host's extracted development-library setup), then:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
PYTHONPYCACHEPREFIX="$PWD/build/qualification/python-cache" \
python3 tools/run_bevy_comparison.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --players 0 1000 10000 --frames 180 --warmup 10 --pre-roll 60 \
  --present-mode immediate --gpu-timings --snapshot-frames 0 60 120 179 \
  --skip-build --validation --headless \
  --headless-dri-prime pci-0000_03_00_0 --gpu 7900 \
  --output build/qualification/bevy-uncapped-final
```

For the separate performance controls, use `tools/compare_stats_display.py` with
`--present-mode immediate --gpu-timings --frames 660 --warmup 60 --route-frames
600 --pre-roll 120`, the same headless/device selection, and no `--validation`.

## Next comparison gate

Bevy still preloads all eight graphs and replays Theia selections. Real streaming
must use common camera demand, protected coarse coverage, bounded fine admission,
file I/O, GPU-readiness publication, cancellation and retirement. Match logical
asset budgets first; backend physical allocations require separate accounting.
A visibility trace or Bevy CPU asset-load completion alone is not equivalent
streaming. Broader M4 LOD, virtual geometry/textures and representative art remain
separate engine work.
