# Owned headless Wayland and Xwayland qualification (2026-10-14)

`tools/qualify_headless_display.py` starts pinned extracted Weston 13 with its
headless/GL backend, private mode-700 runtime/socket and optional Xwayland listener.
It removes desktop display/authentication variables from compositor/client
environments and scopes library/module/data/GPU-selector overrides to children.
No desktop power settings, system services, seats/VTs, user configuration or system
package installation are changed. Only owned process groups and temporary resources
are cleaned up. This is a Linux qualification tool, not an engine backend abstraction.

The tool separates actual Wayland surface validation from Xlib/Xcb surfaces through
the owned Xwayland server. Both Theia paths still select RX 7900 XTX/RADV. GL
composition is a separate role; the qualified child-only Mesa selector chooses
the same GPU. Native-seat/physical scanout, real input and performance are not tested.
The desktop monitor was DPMS-off before/after these runs and remained off during
qualification; the headless compositor supplies its own repaint clock.

## Executed evidence

Linux Mint 22.3, Radeon RX 7900 XTX/RADV Mesa 26.2.4, Weston 13.0.0,
system Xwayland 24.1.6, RenderDoc 1.46:

- Eight native Wayland modes (clear, mesh-only, direct task, indirect, visibility,
  authority fixture, local demo and linear mip-tail-5) each present 120 WSI smoke
  frames through seven swapchain generations, with zero Vulkan warnings/errors
  through teardown. Resize, zero-size deferral and labeled recovery injection pass.
  Winit's previously documented programmatic minimize/restore exclusion remains.
- The same eight modes each present 120 ordinary Xwayland frames through two
  generations, with zero diagnostics. Actual Theia surface logs identify Xlib,
  not a guessed generic "X11" surface label. The first helper check mistook that
  label and failed; corrected assertions accept the explicit Xlib/Xcb handles.
- Three Xwayland visibility captures use explicit fixture counts 0/1/2 and each
  verify 84 identity/depth/colour samples plus indirect commands/counts. They use
  the existing M3 independent reference; generic default captures are not claimed
  to exercise three distinct work counts.
- Three Xwayland automatic linear mip-tail captures check empty/narrow/full views,
  all six resident mip byte arrays, original-dimension gradients, source/image mip
  mapping, reason fields and 81 diagnostic records each. Colour/depth samples are
  81/83/85. The selected device reports eight-bit filter weights; existing bounded
  precision-aware checks apply. These are render-target replay pixels, not a
  composited screenshot or Wayland RenderDoc replay.

Thus 16 standalone runs (1920 presented frames) and six captures pass; 501 pixels
and 243 tail diagnostic records are checked. Capture validation remains disabled
and separate from standalone validation. No Rust/shader changes occur. Python
syntax/whitespace checks pass. Native Windows/cross-vendor/seat/scanout and hosted CI
remain unqualified.

## Open recovery case

Initial Xwayland full WSI recovery smoke timed out after the frame-80
PresentSuboptimal injection and generation 10, both with integrated-GPU composition
and matched RX 7900 XTX composition. Neither timeout has completed teardown evidence
and neither is counted as passing. The precise cause is unproven. Ordinary frames
and native Wayland WSI pass independently; the helper explicitly reports
`xwayland_wsi_recovery: false`. It does not silently substitute a recovery test or
change the renderer to hide the failure. Existing desktop X11/nested Wayland evidence
remains distinct. Logs: `headless-xwayland-{probe,samegpu-probe}`.

## Dependencies and commands

Use the extracted Weston setup described in [nested Wayland](WAYLAND_NESTED.md).
The Xwayland module additionally needs privately extracted Ubuntu Noble
`libxcb-composite0` 1.15-1ubuntu2 amd64, which is absent from this host's system:

| Artifact | SHA-256 |
|---|---|
| libxcb-composite0 deb | a6cfd3799a2fab79ecc1539e7ab2092da1f2ae6abe560017686314276e71884e |
| loaded libxcb-composite.so.0 | 05ecaefd248cffe76d6c7c49425fda28708cecc9cec7b87c8f782803bbbae078 |
| system /usr/bin/Xwayland | 2ff53a6f24a30bae4bab731e48bce39f6dd71c43f5221a0944b7d80add747a14 |

Extract the deb into `build/qualification/headless-xwayland-tool/root`. No system
installation is needed; the helper's `--extra-root` selects this private dependency.
The initial missing-module startup failure is not a runtime pass. Existing system
dependencies/graphics drivers and Xwayland binary satisfy this host; this is not a
general dependency installer or a pinned Xwayland build.

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
XDG_DATA_HOME="$PWD/build/qualification/renderdoc-data" \
python3 tools/qualify_headless_display.py \
  --output build/qualification/headless-display-qualified \
  --compositor-dri-prime pci-0000_03_00_0 \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --auto-mip-case build/qualification/m4-multi-image-cook.json \
                  build/qualification/m4-linear-thin-pattern/cook.json \
  --linear-filter --tail-budget 507
```

The PCI selector is specific to this qualified host and affects only composition;
omit it or supply an appropriate Mesa selector elsewhere. Optional capture/texture
arguments bound the suite rather than implying those checks ran. The final report,
per-mode logs, replay reports and compositor logs are under
`build/qualification/headless-display-qualified/`. Weston retains its previously
recorded layer-finalization diagnostic on shutdown; no clean-compositor claim.
Owned temporary sockets/server groups are gone after completion.

Primary background: [Weston backend documentation](https://wayland.pages.freedesktop.org/weston/toc/running-weston.html)
describes headless operation for test suites. Its current version is newer than the
pinned local Weston 13; actual local help/source/execution establish the used flags.
No architecture decisions change. Next integrate owned display environments with
asset qualifiers, keeping Wayland validation and Xwayland replay separate.
