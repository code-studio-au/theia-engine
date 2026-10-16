# Shared benchmark frame schedule — qualified 2026-10-16

An optional clock now gives the native Theia benchmark and isolated Bevy reference
the same nominal frame-start deadlines. This closes a prerequisite for comparing
camera streaming: matching logical camera frames at different uncapped speeds
gives file workers different wall-clock deadlines.

## Contract

- Public launchers `run_scene_benchmark.py` and `run_bevy_comparison.py` accept
  `--target-hz 1..240`. Omission retains the previous presentation-paced behavior.
- Render Lab accepts `--benchmark-target-hz` only with a timed benchmark. Bevy
  accepts `--target-hz` for replay and camera-stream routes, excluding frozen
  snapshots, CPU reference requests and the fixed fine-operation probe.
- Both executables compile the same Rust clock module. Frame zero establishes an
  independent monotonic epoch after startup; Bevy's pre-roll is outside that epoch.
  Each deadline is `floor(frame * 1_000_000_000 / hz)` nanoseconds after the epoch.
  Late frames are retained, with neither skipped indices nor schedule rebasing.
- Waiting sleeps rather than spins. It occurs before streaming/proxy work and
  outside the existing CPU work interval; host cadence includes the wait. GPU
  timestamp scopes remain unchanged. Theia carries the clock observation with
  its pending frame so later query collection cannot shift it to the next frame.
- `metadata.frame_pacing` describes the requested schedule, or is null when
  disabled. Enabled samples contain scheduled/elapsed/lateness/wait milliseconds.
  Warmup filters timing summaries without resetting the clock or camera.
- JSON and CSV preserve the observations. Both HTML reports describe the clock;
  Theia exposes a lateness trace, and the comparison lists per-engine lateness.
  `benchmark_clock.py` independently checks every integer deadline, frame index,
  monotonic time and lateness calculation, and counts whole-period misses.

## Executed evidence

Ignored local artifacts: `build/qualification/bevy-clock/`.
Native matrix: `native/report.json`; replay charts: `native/replay/comparison.html`.
All native cases used the RX 7900 XTX / RADV, Vulkan, a private native Wayland
display and actual validation. These are correctness runs, not performance samples.

| Case | Recorded frames | Result |
|---|---:|---|
| Immediate, 60 Hz, zero proxies, both engines | 360 | Every deadline checked; no full-period misses |
| Immediate, 60 Hz, 10000 proxies, both engines | 360 | Same; GPU queries and both stats displays enabled |
| Bevy camera stream, Immediate, 60 Hz, 10000 proxies | 180 | 11 publications and 11 retirements; final coarse-only drain; no full-period misses |
| Bevy replay, FIFO, requested 240 Hz | 180 | 179 frames at least one period late; every frame retained |
| Theia camera stream, FIFO, requested 240 Hz | 140 | 136 frames at least one period late; every frame retained |

The 60 Hz replay p95 lateness ranges from 0.059 to 0.061 ms; its largest recorded
lateness is 0.397 ms. The camera-stream p95 is 0.062 ms, maximum 0.069 ms.
These observations qualify the clock on this host and compositor, not a timing
guarantee. FIFO overload intentionally exposes accumulated drift; those overloaded
runs cannot support a claim of matched wall-clock demand.

Seven replay/camera captures pass 823 scene-colour probes with maximum RGB error
one byte, plus four eleven-row HUD checks. The camera verifier's existing bounded
ambiguous-proxy-depth exclusion is enabled but excludes zero rays in these three
captures. Independent CPU/source-ground checks pass 130000 proxy poses across
fine/coarse and near/far representations. Camera demand, unique encoded ownership,
logical payload caps, publication epochs and final cleanup pass the existing
independent streaming checker. No validation diagnostics were reported.

Seven invalid executable configurations reject, and two forged clock traces
(altered deadline/lateness) reject. A separate default FIFO pair records 60 frames
per engine, GPU queries and two strict captures; it confirms null clock metadata
and absent per-frame clock observations when no schedule was requested.
A supplementary 30 Hz, twelve-frame pair checks JSON/CSV clock round trips and
both HTML clock summaries with GPU timing and stats enabled.

Linux release builds, denied-warning Clippy and formatting pass for both tools;
32 Render Lab and 12 Bevy tests pass, including drift and missed-deadline tests.
All 57 existing SPIR-V modules validate for Vulkan 1.3. The preserved compute
executable again verifies all 256 GPU values. Windows renderer cross-link/lint and
32 Wine CPU tests pass under `windows/`; native Windows GPU and the optional
Bevy Windows backend remain unqualified.

## Reproduce

With matching release binaries, compiled shaders, the checked cooked manifest
and native Wayland dependencies prepared:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
PYTHONPYCACHEPREFIX="$PWD/build/qualification/python-cache" \
python3 tools/qualify_benchmark_clock.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --headless-dri-prime pci-0000_03_00_0 \
  --output build/qualification/bevy-clock/native
```

For a scheduled visual replay use the existing comparison launcher with
`--target-hz 60 --present-mode immediate`. Standalone Bevy camera runs can add
`--target-hz 60` to `--camera-stream`; diagnostic captures and serial lifecycle
acknowledgements remain enabled there.

## Limits and next gate

The clock measures starts, not file-request issue, upload/publication or display
completion. Epochs are independent between sequential runs. It adds no real-time
guarantee, cold-cache protocol, artificial I/O latency or automatic overload
correction. Requested caps deliberately constrain host cadence and are not an
uncapped throughput score. Background worker CPU, physical VRAM and existing
differences in CPU/GPU metric scopes remain outside this gate.

The ordinary comparison still replays Theia selections with Bevy graphs preloaded.
A measured independent streaming pair still needs a capture-free profile, explicit
cache/accounting scope, streaming GPU/HUD instrumentation and reports of each
engine's differing readiness/selected detail. Those capabilities remain outstanding.
The prior [device-loss/platform incident](M4_BEVY_CAMERA_STREAM.md) is unchanged;
these passing runs do not establish its root cause or fix device-loss cleanup.
