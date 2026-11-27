# Stats display comparison — 2026-10-16

The optional native Bevy reference now accepts `--stats-overlay`, enabling a
corresponding Bevy UI panel while the source Theia run uses its existing GPU HUD.
The standalone `compare_stats_display.py` runner records both engines with their
displays disabled/enabled. This changes comparison tooling only: Theia's renderer,
shaders and general UI architecture are unchanged.

Both panels occupy 468x288 pixels at (12,12), with eleven rows updated per frame.
Theia retains its fixed 5x7 ASCII font in resolve, GPU work counters and completed
latency/scoped allocation data. Bevy uses ordinary UI layout, its default font,
18 px text and 24 px line height. Its live rows show rolling 30-interval FPS,
previous CPU main-schedule milliseconds, requested proxies, selected regions,
preloaded chunk/world-texture counts and source time/frame. GPU time is `N/A`;
source GPU timings are not substituted. World textures exclude the white proxy
texture and font atlas. Font shapes and available metrics intentionally differ.

Theia's GPU debug font is not a general UI system. The M6 UI/text targets remain
outstanding. Bevy's text/layout system performs more general work than this fixed
Theia font. No locked architecture decision changes and no Bevy dependency enters
Theia's production packages.

## Executed evidence

Ignored paths below are relative to `build/qualification/`.

- `bevy-stats-final/comparison.json`: validated native Wayland/Vulkan pairs at
  zero and 10000 proxies, 1280x720, 120 source/replay frames with ten warmup frames.
  Theia validation totals are zero warnings/errors. Bevy/wgpu/Vulkan validation
  logs contain no Vulkan diagnostics; its benign decoration warning is retained.
- Six frozen captures at frames 0/80/119 verify all eleven UI rows through text
  metadata, panel background and nonempty glyph coverage: **42969 bright ink
  pixels** across the row checks. This is not an exact native glyph-shape oracle.
  Snapshot text metadata freezes at request time before asynchronous readback.
- Independent source rays still pass **583 scene pixels outside the HUD**, with
  maximum **one RGB byte** error. HUD-covered rays are counted separately and
  excluded from world-colour checks. Existing geometry/colour limits are retained.
- The 10000-proxy case passes **70000** CPU pose/ground/detail checks across seven
  frames, covering fine/coarse terrain and near/far detail; ground error is below
  1.71e-7 m. Clocks, selected regions, backend/device and source hashes match.
- `bevy-stats-timing-final/stats-comparison.json`: four separate release runs at
  10000 proxies, validation disabled, 660 frames / 60 warmup / repeating 600-frame
  camera period. Each engine has **600 measured frames with matching selections**
  in its disabled/enabled pair. Bevy has 120 extra initial pre-roll frames.
  `statistics-proof.json` checks sample counts, finite intervals and difference
  arithmetic. These runs occurred after compilation finished, without other
  qualification GPU jobs running concurrently.
- `bevy-stats-{build,clippy,tests}.log`: release build, denied-warning Clippy and
  **ten** CPU tests pass. The new FPS test independently checks the rolling-window
  arithmetic and disabled collection. Formatting and Python syntax pass.
  Generated interactive report JavaScript passes Node syntax checks.

Observed means in the final timing context:

| Engine / metric | Display off | Display on | Observed change |
| --- | ---: | ---: | ---: |
| Theia GPU resolve | 0.037850 ms | 0.036667 ms | -0.001183 ms |
| Bevy CPU main schedule | 0.811277 ms | 0.900979 ms | +0.089702 ms |

These rows have **different scopes** and do not rank the engines. The small
negative Theia observation is not evidence of free UI or a meaningful speedup.
Theia avoids scene resolve shading under its panel; Bevy draws UI after rendering
the scene. Other frame work, optional descriptor profile, fonts, counters, clock
variation and run order also affect the observations. FIFO host intervals remain
about 24–25 ms and do not reveal uncapped throughput. Bevy GPU time is unmeasured.
The report therefore labels these as profile observations, not isolated text cost.

The earlier `bevy-stats-timing/` trial overlapped Clippy compilation and is
superseded by `bevy-stats-timing-final/`. The earlier `bevy-stats-native/` images
predate the request-time metadata freeze; final captures use the corrected code.
Native Windows UI/GPU execution and cross-vendor performance remain unqualified.

## Reproduce

```sh
python3 tools/compare_stats_display.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --output build/stats-comparison --players 10000 \
  --frames 660 --warmup 60 --route-frames 600 --skip-build \
  --headless --headless-dri-prime pci-0000_03_00_0 --gpu 7900
```

Build the standalone reference with its UI/default-font features first. Theia's
existing modules remain unchanged. Use `run_bevy_comparison.py --stats-overlay
--validation --snapshot-frames 0 80 119 --frames 120 --warmup 10` for corresponding
visual checks; that runner needs the same scene/output/device arguments.
[Full build and scope instructions](../../tools/theia-bevy-benchmark/README.md).
