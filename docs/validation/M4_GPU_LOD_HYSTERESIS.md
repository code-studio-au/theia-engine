# M4 GPU resident heightfield LOD hysteresis (2026-10-11)

The qualified two-page heightfield renderer now retains selection on GPU between
frames. `--lod-hysteresis <fraction>` accepts finite [0,1], default zero. Fine-to-
coarse requires padded projected error below a stricter entry budget; coarse-to-
fine still uses the original hard budget. Clip and numeric-profile guards override
history. This implements the preceding CPU contract conservatively; it does not
add hierarchy, streamed page demand, morphing, general 3D error or temporal shading.

One fixed view/page pair owns the work buffer. The first dispatch explicitly resets
to fine and avoids reading uninitialized storage; resizing retains history while
recomputing the bound for the actual extent. A new work buffer starts a new history
context. Reuse now publishes prior compute writes to compute storage reads/writes,
as well as preserving indirect/task/mesh synchronization. Runtime never reads back
selection or counts. Slang disassembly confirms the prior-choice load is inside
the nonreset/enabled branch. Single-frame-in-flight behavior remains unchanged.

Compute keeps its 112-byte push block; viewport Z carries entry fraction and W reset.
Mesh remains 64 bytes. Work byte 44 remains selected page; unused bytes 16/20/24/28
now record previous level/entry threshold/hard budget/reset for offline replay.
Reason 4 means fine retained by the entry threshold. Existing reasons 0–3 and fixed
storage bindings are preserved. The f32 entry budget has an inward safety margin;
replay requires it to be no greater than the configured mathematical entry limit.
Existing projected-bound margins and bounded numeric profile remain unchanged.

`--lod-hysteresis-test` explicitly drives eight 20-frame budget phases, fraction .2,
and a reset probe throughout phase six. It rejects manual budget/fraction overrides.
This is a qualification stimulus, not automatic application tuning or camera demand.

| Phase / capture frame | Hard budget | Selected page | Reason | Reset |
|---|---:|---|---|---|
| 0 / 5 | 6 | Fine | Hysteresis | No |
| 1 / 25 | 7 | Coarse | Coarse | No |
| 2 / 45 | 6 | Coarse | Coarse | No |
| 3 / 65 | 4 | Fine | Pixel budget | No |
| 4 / 85 | 7 | Coarse | Coarse | No |
| 5 / 105 | 6 | Coarse | Coarse | No |
| 6 / 125 | 6 | Fine | Hysteresis | Yes |
| 7 / 145 | 4 | Fine | Pixel budget | No |

Actual RX 7900 XTX/RADV 26.2.4/X11 replay matches this trajectory with both default
and shared-primitive-interface shader directories. All 16 captures verify exact
resident page bytes, selection/culling/indirect work, projected bounds against the
CPU oracle and 81 depth/gradient-colour pixels each. The same six-pixel budget yields
different valid levels according to history; the reset probe restores fine. Four
additional captures (81 pixels each) verify disabled stateless selection, zero-error/
zero-budget eligibility with fraction one, far clipping and numeric-profile fallback.
Capture validation stays explicitly disabled; runtime has no readback.

Both shader directories compile 31 outputs and pass Vulkan 1.3 SPIR-V validation.
Existing Slang profile-upgrade notices remain; no warning-free shader claim.
Renderer 22 tests, assets 42 tests, formatting and denied-warning Clippy pass.
Seven invalid CLI cases reject before GPU initialization.

Standalone validation:

- Default and shared directories each pass a 180-frame full trajectory, 120-frame
  hysteresis WSI and 120-frame disabled WSI; zero diagnostics through teardown.
- Indirect, 16-image complete-set streaming and authority WSI regressions each pass
  120 frames with zero diagnostics. Preserved headless compute verifies 256 values.
- Owned nested Weston runs eight native Wayland WSI modes (six preceding modes and
  two LOD modes), 120 frames each, plus the full 180-frame trajectory, all zero Vulkan
  diagnostics. This establishes execution, not Wayland LOD replay/pixel proof. The
  existing programmatic restore exclusion and compositor shutdown diagnostic remain.
- All 11 packages cross-link/lint for Windows GNU; Wine CPU tests pass 132 total,
  including the three new asset tests. Native Windows/GPU/Quinn remain unqualified.

Reproduction after building Rust and compiling the shader directories:

```bash
DISPLAY=:0 VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
XDG_DATA_HOME="$PWD/build/qualification/renderdoc-data" \
python3 tools/qualify_lod_hysteresis.py \
  --fine build/assets/ec117bdb76cfa189ad44957e80117497fb317fa56891edf363341e0679c3a7dd.theia-mesh \
  --coarse build/assets/1736ab5476264c00eca81ea78a56f142a87935a3b81e2bc85931bbcd6344430f.theia-mesh \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --output build/qualification/m4-gpu-hysteresis
# Add --shader-dir build/shaders-shared for the alternate-interface run.
```

Artifacts: `build/qualification/m4-gpu-hysteresis*/` captures/reports/images,
`m4-gpu-hysteresis-{shaders,shared-shaders,build,tests,clippy,invalid,headless}.log`,
regression logs and compute disassembly; `wayland-lod-hysteresis/` and
`windows-lod-hysteresis/`. M4 remains open; no performance, hosted CI or push claim.
