# M4 streamed benchmark launcher — 2026-10-15

The public launcher now exposes `--streamed-world`, defaulting to 10000 GPU visual
proxies with a live HUD at a requested 1280x720. It can freshly cook and independently
check eight fine/coarse graphs using the existing qualified native-tool cache, or
reuse `--cooked-scene`. No dependencies download automatically. Preparation's
source-to-cook and Rust admission proof precedes GPU execution.

This connects the already qualified camera-world/crowd/timing/HUD profiles. It
changes no shader or residency policy. Width/height options now apply to streamed
timing mode, while direct runs retain their previous 960x540 defaults. Reports
include executable/shader/manifest hashes, camera ownership history, JSON/CSV and
an interactive HTML trace with memory/counter metrics. CSV preserves raw samples;
HTML plots measured samples and exposes world selection/epoch in the tooltip.

## Executed evidence

All following paths are ignored local artifacts under `build/qualification/`.
Native runs used RX 7900 XTX/RADV, private Weston Wayland, release and actual
Vulkan validation. No visible desktop benchmark was requested or launched.

- `streamed-launcher-final/report.json`: six successful runs across five cases:
  zero/10000-proxy streamed, repeated tight one-fine-slot without HUD, procedural,
  resident whole-world and resident mixed 10000-proxy. Each runs 180 frames with
  ten warmup frames. Independent streamed ownership/clock/summary checks pass;
  CSV field values, all 15 world columns and HTML measured arrays match JSON.
  Node syntax checks pass. An impossible bootstrap-cap rerun removes owned
  JSON/CSV/HTML/suite success artifacts; five invalid argument combinations fail
  before execution. Successful evidence remains separate from failure outputs.
- `streamed-launcher-auto/verification.json`: actual automatic preparation and
  90-frame 10000-proxy run pass source/admission, export and independent timing/
  ownership checks; 80 measured samples, 14 mutations, peak complete-current/
  pending/table charge 553984 bytes. Asset proof/logs are in `assets/proof/` and
  `assets/prepare.log`. Preparation reused the built importer.
- `streamed-launcher-build-path/`: normal launcher build path validates all 52
  shader modules, builds release and completes 120 HUD-enabled streamed frames.
- `streamed-launcher-prepared/`: additional 120-frame zero/10000-proxy native runs.
- `streamed-launcher-{clippy,build,tests}.log`: denied-warning Clippy, release build
  and all 30 renderer CPU tests pass. Formatting/diff checks pass.
- `windows-streamed-launcher/report.json`: all eleven Rust packages cross-link,
  pass Clippy and execute CPU tests through Wine. This is not native Windows GPU
  execution or native Windows qualification of the offline-tool cache layout.

Reproduce the route/export/failure checks after preparing both fixture manifests:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
python3 tools/qualify_streamed_launcher.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --resident-scene build/qualification/resident-world-cook/scene.json \
  --output build/qualification/streamed-launcher-final \
  --gpu 7900 --headless-dri-prime pci-0000_03_00_0
```

## Limits

This adds repeatable access and report export, not a new renderer qualification
claim. Existing pixel/GPU-counter/grounding evidence remains in the
[HUD gate](M4_STREAMED_WORLD_HUD.md). The native cached cooking layout is
Linux-qualified; prepared manifests provide the route for other tool layouts.
GPU timings exclude earlier uploads/rebuilds; CPU wall includes synchronous
renderer costs. Memory scopes are distinct and do not total VRAM. FIFO, one frame
in flight, opaque fixture materials and visual proxies remain explicit limits.
General catalogs, independent per-view LOD, virtualized geometry/textures,
characters and authoritative MMO simulation remain outstanding.
