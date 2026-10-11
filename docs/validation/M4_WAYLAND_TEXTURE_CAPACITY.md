# M4 maximum texture-table Wayland execution (2026-10-11)

The preceding [resident capacity](M4_TEXTURE_CAPACITY.md) and
[complete-set replacement](M4_TEXTURE_STREAMING_CAPACITY.md) fixtures now execute
through actual Wayland Vulkan surfaces on RX 7900 XTX/RADV Mesa 26.2.4 within the
owned Weston 13 X11/GL compositor. This extends execution/validation coverage;
capacity pixel/texel replay proof remains the separate X11 evidence. No capacity
Wayland screenshot/replay, native-seat, Windows or cross-vendor claim.

`tools/qualify_wayland.py --capacity-cook` runs its six preserved standalone modes,
then four maximum-capacity bundles, an uncapped complete-set replacement probe,
and four independent measured-budget cases. All 15 runs complete 120 frames with
zero Vulkan synchronization-validation warnings/errors, including teardown.
Every capacity run confirms a Wayland surface and 16 hash-sorted resident images.

| Capacity case | Publications | Deferrals | Graphics/swapchain generations |
|---|---:|---:|---:|
| Four fixed bundles | 0 each | 0 each | 7 each |
| Uncapped replacement | 5 | 0 | 12 |
| Exact CPU/GPU/image limits | 5 | 0 | 12 |
| CPU limit minus one byte | 0 | 5 | 7 |
| GPU limit minus one byte | 0 | 5 | 7 |
| Image limit minus one byte | 0 | 5 | 7 |

Observed coexistence charges match X11: 22088 unique encoded CPU bytes,
141760 GPU root/image bytes and 131072 physical image bytes. Replacement
rebuilds a complete graphics generation, so five publications add five generations
to the seven WSI generations. The initial new harness incorrectly expected seven
for streaming as well; the renderer had passed with 12 and zero diagnostics.
The assertion now accounts for publications, and the full suite was rerun.
Initial harness failure is retained under ignored `build/qualification/`.

Wayland programmatic minimize/restore remains explicitly skipped due to pinned
Winit's unsupported restoration; resize, zero-size deferral and labeled WSI recovery
injections still execute. Weston again logs its own layer-finalization diagnostic
on owned-process termination; no clean-compositor-shutdown claim. The helper
terminates only its compositor and removes its private runtime socket directory.

Python syntax and whitespace checks pass. Rust/shaders remain unchanged; no new
compilation claim. M4/general streaming/LOD/virtual textures remain open, including
conservative device-idle retirement and other capacity/count transitions.

```bash
DISPLAY=:0 VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
PYTHONPYCACHEPREFIX=build/qualification/python-cache \
python3 tools/qualify_wayland.py \
  --capacity-cook build/qualification/m4-capacity/assets/cook.json \
  --output build/qualification/wayland-capacity
```

Observed reports/logs: `build/qualification/wayland-capacity/` and
`wayland-capacity.log`; original assertion failure:
`wayland-capacity-initial-harness-failure.log`. Local commits only; no push or
hosted CI claim.
