# Owned headless environments in texture qualifiers (2026-10-14)

`tools/qualify_automatic_mips.py` and `tools/qualify_texture_mips.py` now accept
`--headless` and optional `--headless-dri-prime`. The shared `headless_display.py`
context owns a private Weston/GL/Xwayland runtime. Standalone validated commands
use its actual Wayland socket; RenderDoc commands use its Xwayland DISPLAY. Desktop
display/authentication variables are removed rather than used as fallback. Default
desktop workflows remain unchanged when the flag is absent.

The context scopes paths/GPU selection to its compositor child and retains display
ownership through all qualifier operations. Startup or caller errors close the
owned compositor/group before removing its temporary runtime. ExitStack guards
both success and failure. Reports explicitly record Wayland validation, Xwayland
replay, native-seat false and Xwayland WSI-recovery false. Successful headless
validation runs must log an actual Wayland surface in addition to existing frame/
diagnostic predicates. Invalid source/view/alpha commands also use the owned
environment, although admission errors must still precede Vulkan initialization.

RX 7900 XTX/RADV Mesa 26.2.4, extracted Weston 13 / system Xwayland 24.1.6:

- Linear horizontal mip-tail at dense cap 507: three Xwayland captures verify six
  resident levels, 81 diagnostic records per view and 81/83/85 pixels. Native
  Wayland WSI and exact 1536-byte image-cap runs each present 120 frames with zero
  warnings/errors; 1535 rejects cleanly, and missing shader cleanup passes. The
  selected-device precision query uses a separate validated Wayland frame.
- Native box-generated 8x8 chain: fixed views 0/2/3 each check all four raw GPU
  levels and 85 pixels, plus 120 Wayland WSI frames. Exact 4096-byte image-cap
  execution and 4095-byte rejection pass zero diagnostics; invalid level/alpha
  reject before Vulkan.
- Injected caller exception after owned display startup confirms separate backend
  environments and removal of the private runtime after cleanup. No owned server/
  capture processes remain after the completed checks.

Six captures and 504 framebuffer probes pass. Capture validation remains disabled
and separate from the standalone runs. Python syntax/whitespace checks pass; no
Rust/shader changes or new Windows/GPU-vendor qualification. The known headless
Xwayland recovery timeout stays open and is not worked around inside the renderer.
Desktop power settings are not modified by these helpers; the independent preceding
[headless suite](HEADLESS_DISPLAY.md) establishes actual monitor-off execution.

Reproduction after the private dependency setup in that report:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
XDG_DATA_HOME="$PWD/build/qualification/renderdoc-data" \
python3 tools/qualify_automatic_mips.py \
  --reference build/qualification/m4-multi-image-cook.json \
  --texture-cook build/qualification/m4-linear-thin-pattern/cook.json \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --output build/qualification/m4-headless-tail-assets \
  --tail-budget 507 --linear-filter --headless \
  --headless-dri-prime pci-0000_03_00_0
```

The PCI selector is host-specific and optional. Local ignored reports/logs:
`m4-headless-{tail,fixed}-assets/` including compositor logs, and
`headless-context-failure/report.json` under `build/qualification/`.
This is qualification-tool integration, not native-seat/physical scanout,
composited display pixels, a Wayland debugger workaround or performance evidence.
No architecture decisions change. Asset refinement/publication work can now use
these explicit independent environments without requiring an active desktop monitor.
