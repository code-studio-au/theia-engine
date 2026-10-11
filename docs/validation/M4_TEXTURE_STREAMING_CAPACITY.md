# M4 full-capacity complete-set replacement (2026-10-11)

RX 7900 XTX / RADV Mesa 26.2.4 / X11 executes asynchronous replacement between
two of the preceding [16-image bundles](M4_TEXTURE_CAPACITY.md). Both sets share
immutable geometry, UV and texture sources but use different material/table/manifest
bytes. Runtime uploads complete independent GPU sets; encoded CPU dependencies
deduplicate by content hash. No Rust, shader, dependency or locked decision changes.

`tools/qualify_texture_streaming_capacity.py` measures actual physical charges, then
checks the exact limits and one byte below each independently. The CPU charge also
matches an independent union of immutable file hashes/lengths, including manifests.

| Budget | Exact coexistence limit | One-byte-short result |
|---|---:|---|
| CPU unique encoded bytes | 22088 | 5 CPU-budget deferrals, 0 publications |
| GPU root buffers + images | 141760 | 5 physical-budget deferrals, 0 publications |
| GPU images only | 131072 | 5 physical-budget deferrals, 0 publications |

Each complete GPU set charges 70880 bytes: 65536 images and 5344 root buffers.
The pending set has four root buffers and 16 images, with 5600 staging allocation
bytes tracked separately. Both complete sets coexist during upload: 32 physical
images, although their immutable source hashes are shared. Render targets, work
buffer, command pools, descriptor/sampler resources and staging are outside these
budget values; this is not a whole-process/device-memory cap.

The exact-limit run publishes five complete generations at frames 22/42/62/82/102.
Every short-budget run keeps the initial complete generation through 120 frames.
All four runs exit successfully with zero synchronization-validation warnings/errors,
including transfer retirement and teardown. Pending physical-budget failures release
submitted transfers before returning; subsequent attempts repeat cleanly. CPU-budget
failure defers before GPU submission.

Four separate RenderDoc 1.46 captures at frame 30 verify 85 framebuffer samples
each, exact complete dependencies and all 16 resident GPU image texels. Exact limits
sample replacement descriptors 12–15. The three short-budget cases sample retained
initial descriptors 0–3. These demonstrate visible atomic publication/retention,
not just host counters. Capture validation is explicitly disabled; standalone runs
provide validation evidence. Runtime has no readback.

The existing capture helper now accepts positive integer CPU/GPU/image budget
environment overrides for streaming captures. Existing unbudgeted behavior remains
covered by the preceding capacity captures. Three Python syntax checks and diff
whitespace checks pass; Rust/shaders are unchanged. Conservative device-idle graphics
retirement remains; no production overlap/allocation reuse, performance, arbitrary
texture-count transition or Windows/Wayland/cross-vendor claim. M4 remains open.

Reproduction after cooking the preceding capacity fixture:

```bash
DISPLAY=:0 VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
XDG_DATA_HOME="$PWD/build/qualification/renderdoc-data" \
python3 tools/qualify_texture_streaming_capacity.py \
  --cook build/qualification/m4-capacity/assets/cook.json \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --output build/qualification/m4-capacity/streaming
```

Observed reports/logs/captures/images: `build/qualification/m4-capacity/streaming/`.
The independent CPU-union check was added and the complete harness rerun successfully.
Offline fixture subprocesses suppress Python bytecode files in the tracked tools
folder. Changes remain local; no hosted CI or push claim.
