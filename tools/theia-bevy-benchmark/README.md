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
