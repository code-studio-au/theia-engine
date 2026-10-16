# Theia scene benchmark — native Bevy reference

This optional comparison executable ports Theia's current streamed-world **visual
workload** to pinned Bevy 0.19.1 / wgpu Vulkan. It is not a replacement engine or
a dependency of any Theia runtime package. It provides a native
comparison of the current MMO scene with matching unlit materials.

It uses the exact cooked triangle/UV/texture/material data, shared readonly Rust
camera/proxy motion functions, selected-source terrain heights, proxy near/far
geometry and face colours/fog. Its custom unlit material supports Bevy's bindless
path with ordinary bindings as a capability fallback. MSAA, tonemapping, dithering,
lights are disabled. Both overlays are disabled by default. Window extent and
presentation mode match (FIFO by default).

Bevy preloads all eight representations and replays a completed Theia run's
per-frame selection. Theia's asynchronous streamer is **not ported** by this tool.
Bevy updates proxy transforms/detail through parallel CPU ECS; Theia generates
presentation on the GPU. This compares two implementations of a matched visible
workload, not identical scheduling or isolated renderer costs.

## Run

Build prerequisites: Rust, Theia's already qualified compiler/SPIR-V tools, and
native window-system development libraries (on this Linux host, Wayland and
xkbcommon development packages were needed). Dependencies are locked in this
standalone package's Cargo.lock. No existing engine manifest is changed.

```sh
python3 tools/run_bevy_comparison.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --slangc /path/to/slangc \
  --players 1000 10000 --frames 3660 --warmup 60
```

It builds both release executables and Theia shaders, runs Theia without its HUD,
exports actual cooked resources and the observed route, then runs Bevy. Each
population gets independent source hashes, original reports and Bevy JSON/CSV.
An aggregate JSON and interactive HTML trace are written under ignored
`build/bevy-comparison/`. Each recorded pose follows the same logical source
frame even when wall-clock duration differs. Bevy additionally holds the initial
pose for `--pre-roll` frames (default 120) before recording to prepare resources.

`--skip-build` reuses matching binaries/SPIR-V. Add `--headless` for the existing
owned Weston setup; otherwise the windows are visible sequentially. For this GPU:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
python3 tools/run_bevy_comparison.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --players 10000 --frames 660 --warmup 60 --route-frames 600 \
  --skip-build --headless --headless-dri-prime pci-0000_03_00_0 --gpu 7900
```

`--validation` enables Theia validation and requests Bevy/wgpu validation plus
Vulkan's validation layer. Logs containing validation errors fail the runner.
`--snapshot-frames 0 60 120 179` adds separate frozen captures and independent
source ray/colour checks; Pillow is needed only for those image checks. Captures
are excluded from timing runs. Successful report files do not prove validation
when it was disabled. The runner withholds aggregate success after failures.

The qualification machine used extracted development packages, without installing
system packages. Reuse that local cache when compiling here if needed:

```sh
PKG_CONFIG_PATH="$PWD/build/qualification/bevy-tool-sdk/root/usr/lib/x86_64-linux-gnu/pkgconfig" \
LIBRARY_PATH="$PWD/build/qualification/bevy-tool-sdk/root/usr/lib/x86_64-linux-gnu" \
cargo build --release --locked --manifest-path tools/theia-bevy-benchmark/Cargo.toml
```

Those paths are local setup, not portable defaults or repository prerequisites.
The native GPU evidence is Linux/RADV only; Windows reference execution remains
unqualified. The optional tool is outside the product's Rust CI package matrix.

## Read the results carefully

FIFO host-frame intervals are presentation-limited. Non-vsynced modes are now
available below; compositor pacing may still apply on other systems.
Theia CPU draw wall includes fence/acquire/present; Bevy's recorded CPU main
schedule excludes later render extraction/submission. Bevy GPU timestamps require
`--gpu-timings` and retain their own render-graph scope. Memory/streaming policies differ. Do not divide these CPU scopes,
compare Theia GPU milliseconds to Bevy host intervals, or interpret the HTML as
a total-engine performance ranking.

A one-time initial surface clear addresses observed startup PRESENT-layout and
acquire-semaphore diagnostics in this Bevy reference. It runs before rendering
only while the window needs its initial present, outside recorded samples. It
changes no registry/upstream sources and leaves timed frame rendering unchanged.

[Executed qualification and limits](../../docs/validation/M4_BEVY_VISUAL_REFERENCE.md).

## Compare the stats displays

`--stats-overlay` on `run_bevy_comparison.py` enables Theia's existing GPU debug
HUD and a native Bevy UI panel. Both occupy 468x288 pixels at (12,12), with eleven
rows updated every frame. Bevy uses its default font, ordinary UI layout and
shaped text. It shows live rolling FPS and previous CPU main-schedule time,
requested proxies, active regions, preloaded chunks/world textures and source
time/frame. GPU milliseconds display the previous completed render-graph query
with `--gpu-timings`, otherwise `N/A`; values are never copied from Theia. These
are corresponding stats panels, not identical glyph workloads.

The Bevy panel labels its preload/replay scope. Its world-texture count excludes
the white proxy texture and UI font atlas. Theia's labels retain their existing
selected/current/pending asset scopes. CPU timing labels likewise retain their
different meanings. The feature does not implement Theia's planned M6 UI.

For paired overlay-off/on observations in both engines:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
python3 tools/compare_stats_display.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --players 10000 --frames 660 --warmup 60 --route-frames 600 \
  --skip-build --headless --headless-dri-prime pci-0000_03_00_0 --gpu 7900
```

Build the reference again after enabling its new UI/default-font features before
using `--skip-build`. Output includes both control traces and
`stats-comparison.html` / `.json`. Summaries retain only measured frames with the
same logical time and selected geometry. They report each engine against its own
control; GPU/CPU rows with different scopes are not compared as rankings.

Theia's HUD replaces scene shading inside its panel; Bevy draws UI after scene
rendering. Observed changes therefore include avoided scene resolve work in
Theia, different fonts/metrics and the optional HUD descriptor profile. Presentation and
run-to-run clock/scheduler variation remain. This is a stats-display profile
comparison, not an isolated font microbenchmark or a general UI speed ranking.
Bevy's UI plugins/default font remain available in the disabled control, but no
stats nodes or per-frame text formatting are created there.

[Stats-display qualification](../../docs/validation/M4_STATS_DISPLAY_COMPARISON.md).

## Non-vsynced presentation and GPU timings

Add `--present-mode immediate --gpu-timings` to either comparison launcher.
Supported exact modes are `fifo` (default), `immediate`, and `mailbox`; unsupported
requests fail. Reports record the mode and verification scope. Bevy's configured
surface is private: the tool checks native window/adapter capabilities and the
pinned selector's supported-mode path, and rejects fallback diagnostics.

`--gpu-timings` brackets Bevy render-graph rendering with GPU timestamp queries,
exports raw ticks and warmup-filtered summaries/CSV, and enables a GPU trace
selector in the HTML. It serially waits for each query readback outside the GPU
interval; host cadence includes that wait. Theia's existing GPU frame interval
covers compute/visibility/resolve. These are different renderer pipelines, with
uploads before their ranges excluded. Avoid interpreting the timings as a
whole-engine score. The Bevy HUD can show the previous completed GPU result.
Separate frozen captures keep collection disabled.

Theia's standalone launcher also accepts `--present-mode`; the direct lab flag
is `--benchmark-present-mode`. Shader modules and default FIFO behavior remain.
This does not implement equivalent Bevy streaming: all eight graphs still preload.

[Native qualification, observations and limits](../../docs/validation/M4_BEVY_PRESENTATION_TIMINGS.md).

## Controlled fine-streaming lifetime gate

The ordinary comparison above still preloads/replays. A separate correctness
profile, `--streaming-probe <region 0..3>`, now reopens an actual cooked fine graph
on one file worker over four protected coarse GPU graphs. It checks prepared
meshes/images/materials and queue completion before publication, then waits for
render-asset removal/completion before releasing encoded ownership.

Re-export an overlay-disabled source fixture to include cooked bundle paths, with
at least 140 route frames. The profile requests at frame 20, permits publication
from frame 40 and retires from frame 80. It automatically captures coarse, fine
and final coverage. Optional `--probe-cpu-budget`, `--probe-upload-budget` and
`--probe-cancel-frame 21..39` exercise admission and cancellation. Upload bytes are
adapter-specific logical payloads, not physical GPU memory. It rejects ordinary
snapshot/HUD/GPU-timing flags because these lifecycle captures/waits are diagnostic.

`tools/qualify_bevy_stream_probe.py --scene <scene.json> --source-report
<completed-Theia-report.json> --headless-dri-prime <selector> --output <directory>`
exports the inputs and executes the full owned-headless validation, budget,
cancellation, source, pixel and pose matrix. The cooked assets remain unchanged.
This qualifies one controlled fine load/retirement, not repeated camera demand,
equivalent streaming, physical memory or streaming performance.

[Executed contract and qualification](../../docs/validation/M4_BEVY_FINE_STREAM_PROBE.md).

## Repeated camera-driven streaming

`--camera-stream` runs actual cooked fine loads under the shared camera-demand
planner, protected coarse coverage and GPU readiness/removal acknowledgements.
`--stream-fine-slots 1..4` defaults to two; load/retain radii are 20/35 metres.
`--stream-cpu-budget` and `--stream-upload-budget` limit unique encoded ownership
and adapter-specific logical payloads respectively. Physical VRAM is not measured.
This profile ignores the source report's selections and records its own.

Use a newly exported fixture (at least 140 route frames) with actual cooked paths.
Optional `--stream-capture-frames 0,30,75,...` selects at most sixteen unique
capture frames; default is start/thirds/endpoint. The endpoint drains remaining
fine owners outside recorded samples before success. Frame traces omit duplicated
history; the final report retains a 64-event ring. Source-ground metadata remains
available even when fine GPU payloads are absent.

`tools/qualify_bevy_camera_stream.py --scene <scene.json> --headless-dri-prime
<selector> --output <directory>` prepares route fixtures and runs independent
per-frame demand/ownership checks, captures and proxy pose checks. It includes
rapid cancellation and repeated slot/capacity pressure. Camera captures explicitly
report excluded near-overlapping proxy rays with equal canonical depth codes;
RGB tolerance is unchanged, and GPU depth ordering for excluded rays is unverified.
Historical replay/probe sampling remains strict by default.

This is a correctness profile with captures and serial lifecycle waits. It rejects
HUD, GPU-timing and ordinary frozen-snapshot controls. A timed paired streaming
comparison must still control wall-clock demand rate and cache/accounting scope.
[Qualification and platform incident](../../docs/validation/M4_BEVY_CAMERA_STREAM.md).

## Optional shared frame schedule

Add `--target-hz 60 --present-mode immediate` to `run_bevy_comparison.py` or
`run_scene_benchmark.py` to request matching nominal frame-start deadlines. The
standalone Bevy route accepts `--target-hz 1..240`, including `--camera-stream`;
Render Lab's equivalent is `--benchmark-target-hz`. Default pacing is unchanged.
The fixed operation probe and frozen snapshot/reference controls reject this clock.

The clock starts at recorded frame zero after startup/pre-roll. Absolute deadlines
never skip frames or rebase after a miss. Sleep is outside the CPU work metric;
host intervals include it. JSON/CSV retain each deadline and lateness, and HTML
shows clock evidence. Check those observations before treating I/O deadlines as
comparable: FIFO or an overloaded renderer can fall behind the requested rate.
A cap deliberately limits host cadence; this is not an uncapped FPS score.

`tools/qualify_benchmark_clock.py --scene <scene.json> --headless-dri-prime
<selector>` exercises paired replay, independently streamed camera compatibility,
deliberate FIFO overload and invalid/forged inputs. Camera streaming still uses
diagnostic captures and rejects GPU-timing/HUD controls; measured independent
streaming comparison remains a separate gate.
[Executed evidence](../../docs/validation/M4_BENCHMARK_CLOCK.md).
