# Composited Wayland pixel reference (2026-10-11)

The isolated Weston compositor captures its own displayed output. Offline Python
reads the native screenshot and compares the fixed indirect mesh/task scene to an
analytical raster/color reference. This is a separate observable display oracle,
not debugger replay or a runtime GPU readback path. No Rust/shader changes.

## Executed evidence

Same Linux Mint/RX 7900 XTX/RADV/Weston 13 environment as
[nested qualification](WAYLAND_NESTED.md); Pillow 10.2.0 only in offline analysis.
Ignored artifacts `build/qualification/wayland-pixels/`.

| GPU requested work | Sampled pixels | Background / left / right coverage |
|---:|---:|---|
| 0 | 7973 | 7973 / 0 / 0 |
| 1 | 7932 | 7590 / 342 / 0 |
| 2 | 7888 | 7204 / 342 / 342 |

- Each real Wayland client passes 300 frames/two swapchain generations, zero Vulkan
  warnings/errors including teardown. Actual native surface confirmed by logging.
  Fixed count is a qualification input; GPU still writes indirect command/count
  and downstream task/mesh work. No host work-count readback or vertex buffers.
- All three compositor output screenshots pass independent analytical position/
  barycentric RGB interpolation/sRGB background samples. Count 0 has no geometry;
  count 1 has only the left triangle; count 2 contains both. The two-triangle PNG
  was also inspected visually. No content was edited to match the oracle.
- The 960x540 canvases are detected at [170,243], [403,54], [20,338] in 1400x900
  output, proving the helper does not assume a fixed window placement. Full clear
  rows identify width and clear edge columns identify height. Unexpected scaling,
  occlusion, size or missing canvas fails rather than silently adapting the model.
- Probes use pixel centers every eight pixels, exclude one-pixel triangle edge
  bands, and permit two byte values/channel for sRGB/raster rounding. Required
  background/visible triangle coverage is checked. This is sampled output, not an
  exhaustive pixel proof or HDR/color-management qualification.
- Six preceding 120-frame WSI modes pass again with zero diagnostics. Automated
  Wayland minimize/restore remains explicitly skipped because Winit cannot restore
  programmatically. Available RenderDoc still rejects the Wayland surface extension;
  no replay success is inferred from these screenshots.
- Weston logs `BUG: finalizing a layer with views still on it.` during owned
  compositor termination. This third-party diagnostic is not a Theia Vulkan
  validation message; no clean-compositor-shutdown assertion is made. The compositor
  exits and the private runtime/owned client processes are cleaned up.
- No hosted CI, native-seat, user-input/restore, Windows/cross-vendor or performance
  claim. CPU analysis is an offline qualification tool, not engine gameplay/runtime.

## Capture scope

`--pixels` adds Weston's debug option only to its owned nested compositor. Pinned
source shows this enables its screenshot protocol. The mode-700 temporary socket
contains only test clients; main X11 desktop/user compositor is never captured.
The vendor `weston-screenshooter` writes its native PNG to an explicitly owned
XDG_PICTURES_DIR below ignored output; Python only reads that image. Only owned
processes/runtime and qualification files are managed. No system installation,
production debug/capture endpoint or public artifact publication.

Reference triangles come from the fixed qualified offsets (-.5,.5), scale .45 and
RGB vertex positions; mathematical barycentric interpolation is independent of
SPIR-V/driver/compositor implementation. Clear color is the fixed linear test
profile (.025,.07,.12). Unscaled SDR/opaque output is the only qualified profile.
Next expand platform/build coverage and remaining milestone slices while keeping
native-seat/Windows/debugger gates explicit.
