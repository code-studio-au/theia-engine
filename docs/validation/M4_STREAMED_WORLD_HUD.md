# Live HUD for the streamed world and crowd (2026-10-15)

`--world-camera-hud` requires camera-world crowd mode and a timing report. It
adds the existing tiny repo-native debug font to the combined material resolve.
The overlay is opt-in; timing/recording without it retains the previous profile.
No per-entity host updates, GPU visibility readback or extra HUD draw is added.

## Display scopes

- **FPS:** rolling 30 completed presentation intervals, including warmup samples.
- **CPU MS / GPU MS:** last completed draw-wall / frame-stage timing samples;
  first-frame values are zero. GPU time includes HUD resolve and excludes earlier
  upload/rebuild work, as in the timing contract.
- **MESHLETS visible/total:** current GPU compaction counts summed across four
  world work buffers, versus selected cooked geometry counts.
- **TEX:** selected image binding count, including repeated images across chunks;
  this differs from unique source textures and all retained image owners.
- **ASSET MIB:** current graphs + complete pending graph allocations + the fixed
  terrain table. Work, targets, staging, swapchain/driver costs are excluded.
- **DRAW MIB:** work arenas and owned offscreen targets; excludes asset graphs,
  terrain table, staging, swapchain and driver allocations.
- **PUB / EVICT / DEF:** fine publications, fine retirements and deferred requests.
- **PHASE / T:** current camera route phase/time, with warmup/repeat semantics
  unchanged. **PROXIES / NEAR / FAR** read current GPU work directly.

The fragment path binds four existing world work arenas and the proxy work
buffer, totaling **21 storage descriptors**. It requires a checked optional HUD
profile and a **128-byte** fragment push range. Other world profiles retain their
16-buffer requirement; the intended renderer baseline and production descriptor
policy are unchanged. The three HUD uint4 values occupy bytes 80..128 after
camera rows/texture bases. All 48 bytes must be submitted: the first replay caught
an inherited 32-byte submission truncating the allocation/streaming fields.

Work-buffer publication already covers fragment reads. Immutable generation
rebuilds and reader retirement preserve descriptor owners before resources move.
No additional persistent GPU buffer is allocated for this HUD.

## Observed qualification

`world-hud-final-v3/report.json` passes **19 native cases** (18 common timing/HUD
cases plus the varied-material scene). Runtime cases activate validation and
finish with zero warnings/errors, including controlled bootstrap/source/rebuild
failures. The optimized executable independently passes all 19 cases
(`world-hud-release/report.json`). Invalid timing/HUD combinations fail before
Vulkan; fatal live failures leave no completed timing report.

Eight fresh replays verify **45360 glyph pixels** across all 11 lines and
**1084 scene ray/colour probes**. Cases cover full/repeated/zero proxies, varied
material factors with **14 selected image bindings**, coarse-only budget
deferrals, shared modules, pending GPU allocations and frame zero. One explicitly
reported adjacent-D32 proxy ordering case uses the bounded rule below. Captures
disable validation; native cases provide the validation evidence.

Both shader profiles compile/validate **52 SPIR-V modules**. Renderer formatting,
denied-warning Clippy and 30 CPU tests pass; three additional pure CPU tests
cover the frozen depth-overlap regression and negative cases. Eleven Windows GNU
packages cross-link/pass Clippy and CPU suites pass under Wine
(`windows-world-hud/report.json`); native Windows GPU remains unqualified.

The disabled-HUD timing profile still passes all 17 native cases
(`world-hud-disabled-regression/report.json`). All six upload/error/cooked-stream/
10000-proxy mixed regression plans pass with zero diagnostics. A fresh legacy
mixed HUD replay verifies **5390 glyph pixels**, preserving the shared font and
older HUD path (`world-hud-legacy-mixed/report.json`).

A longer optimized, validation-disabled, HUD-enabled run presents **3660 frames**
and independently checks **3600 measured samples**, 14 world mutations and a
553984-byte peak graph/pending/table charge (`world-hud-long-release/report.json`).
On RX 7900 XTX/RADV, 960x540, private native Wayland/headless-GL compositor, FIFO
and one frame in flight, observed mean frame interval is **25.020 ms** and mean
GPU frame work is **2.397 ms** (p95 **2.468 ms**). This is one local observation
with instrumentation and HUD enabled, not a production performance target.
GPU intervals retain the earlier upload/rebuild exclusions.

## Replay findings and explicit bounds

Dense overlapping proxies exposed an ideal-double ray ordering mismatch at
pixel (493,215), period 5/frame 221. The ideal nearest proxy rounds to D32 code
1065338290; the actual raster identity's independently intersected surface rounds
to **1065338291**, exactly matching stored depth. Both hits are inside primitive
3. This is consistent with finite-precision depth ordering; the evidence does
not isolate a particular transform/raster operation.

Replay now reports proxy/proxy ambiguities only when the actual identity and
primitive match an independent source hit, its D32 code is the same or immediately
adjacent to the ideal nearest code, and stored depth **exactly equals** that
actual source hit's rounded code. Colour still matches that surface. Other ID
checks remain strict; ordinary depth/colour limits stay unchanged. Three frozen
CPU tests reproduce the measured two-proxy case and reject foreign IDs, wrong
primitives, missed geometry, unmatched stored depths and a two-code gap. Probe
selection is deterministic by proxy ID, and the observed pixel remains an
explicit regression probe.

The first-frame capture also records startup staging and retired target metadata.
Replay separates storage/staging roles and verifies their exact inventories,
including all 33 startup staging buffers. Shader payload checks use storage
buffers; visibility/depth images come from the final resolve's actual bindings.
The first-frame HUD verifies zero timing defaults, 10000 far proxies and 31
coarse meshlets. These capture metadata counts are not a live-VRAM measurement.

## Scope

This remains a benchmark debug HUD with fixed ASCII glyphs, no general text/UI
system and no extra gameplay authority. The 468x288 panel changes shaded pixel
coverage, so performance comparisons must record its enabled state. The source
fixture, conservative generation rebuilds, prevalidated assets, two-level terrain
query table and unskinned proxies retain their earlier limitations. Native
Windows GPU execution remains unqualified.
