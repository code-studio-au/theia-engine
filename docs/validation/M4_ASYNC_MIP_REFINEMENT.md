# M4 asynchronous conventional mip-tail refinement (2026-10-14)

Render Lab `--texture-refine-budget <dense texel bytes>` requires an initial
`--texture-tail-budget` and automatic mips. Both suffix plans are admitted before
Vulkan; the requested suffix must add finer source levels. This qualification
requests one refinement after 20 presented frames. It does not infer demand from
GPU feedback or implement a production texture streamer.

One background Rust worker reads and validates the complete content-addressed
source through a bounded channel. The renderer keeps using the coarse image while
I/O runs. The main thread rechecks the initial source hash, fits the finer suffix,
and submits an owned upload. The physical image cap applies to **active plus
pending image allocation requirements**, separately from dense texel bytes and
staging. Insufficient coexistence capacity defers this single request and preserves
the coarse image; a changed source is rejected with the same retention behavior.
Neither path retries in this unit.

The main thread polls the dedicated transfer fence. Only a ready upload can become
a resident image/sampler. Publication retires the previous frame and conservatively
retires/destroys the complete old graphics/presentation generation before replacing
its image and metadata. Original extent, source first level and level count are
updated together, and the new generation uses the existing qualified tail fragment
module. No Slang changes or normal-runtime diagnostic readback are introduced.
This uses existing device-idle generation retirement rather than demonstrating
overlapped multi-frame allocation reuse.

Upload RAII owns transfer/fence, staging and image resources through errors and
teardown. The worker is joined on shutdown; bounded file size does not impose an
I/O latency limit or make blocking OS I/O cancellable. Full source bytes are released
after planning/submission, not retained as a CPU cache. Whole-suffix replacement
duplicates shared coarse levels temporarily; there is no sparse binding, tile
mapping, partial-file I/O, eviction policy or production scheduling claim.

## Independently executed evidence

RX 7900 XTX, RADV Mesa 26.2.4, Linux Mint 22.3; private Weston 13 headless/GL
with compositor selector `pci-0000_03_00_0`. Actual validation is enabled in
standalone native Wayland runs. RenderDoc 1.46 uses owned Xwayland for ordinary
presentation/replay with capture validation disabled. Full Xwayland WSI recovery
remains unqualified as recorded in [headless evidence](HEADLESS_DISPLAY.md).
No desktop power settings or global display configuration change.

The 1024x1 alternating authored eleven-level source has SHA-256
`f1cfec0a4871f82b9ff9486c61b550eda7810f8f3615bfec8b7243f3f62753a0`.
Its distinct per-level green/blue values expose incorrect source/image mip mapping.
Default linear refinement from dense cap 507 to 508 changes source first 5 to 4.
Measured image charges are 1536 active + 1792 pending = **3328 bytes**; staging
charges 512 additional bytes separately. An exact 3328-byte image cap publishes
once; 3327 defers once and keeps source first 5. Standalone WSI recovery, exact and
short budgets, and rejection of a mutated owned source clone each present 120
frames with zero validation warnings/errors including teardown.

Four default-interface captures prove initial coarse rendering (frame 10), coarse
rendering while the finer copy is submitted (frame 21), finer rendering after
publication (frame 50), and retained coarse rendering under the short budget
(frame 50). The pending capture includes `vkCmdCopyBufferToImage` and still samples
the old image. Each replay checks every resident level's raw GPU texels, 81
gradient/identity/source-mip/relative-mip/reason records, and 85 colour/depth pixels.
Queried eight-bit filter precision supplies the existing bounded CPU colour oracle.
No claim is made about measured throughput or frame latency.

The same four captures/budget boundaries/source-rejection checks pass with the
shared shader interface and with protected-1x1 to full-chain refinement:

| Interface / dense caps | Source first | Active + pending image bytes | Exact / one-byte-short cap |
|---|---|---:|---:|
| Default linear / 507→508 | 5→4 | 1536 + 1792 | 3328 / 3327 |
| Shared linear / 507→508 | 5→4 | 1536 + 1792 | 3328 / 3327 |
| Default linear / 4→8188 | 10→0 | 256 + 9472 | 9728 / 9727 |

The full-chain upload charges 8192 staging bytes separately. All **12 captures**
check 1020 pixels and 972 diagnostic records in total, in addition to every active
resident level's raw texels. Every pending capture includes a finer image copy
while sampling the original source first level. All twelve standalone Wayland
qualification runs complete 120 frames/zero diagnostics, including WSI recovery,
exact publication, short-budget retention and source rejection for each row.

Additional native Wayland lifetime/regression checks pass zero diagnostics through
teardown: a 22-frame run exits with the finer transfer submitted but unpublished;
an owned shader-directory clone loses its tail fragment module after frame-20 I/O
starts, so the ready finer image fails graphics-generation construction and the
process exits cleanly with the expected missing-shader error. The verified shader
directory remains untouched. Ordinary indirect rendering, static nearest coarse
tails, and sixteen-image whole-bundle streaming each pass 120-frame WSI recovery;
the latter retains its five qualified publications. Logs are under
`build/qualification/m4-refinement-cleanup/`. These checks cover conservative
single-frame owners, not general multi-frame retirement.

The preserved headless compute executable also re-runs on RX 7900 XTX and verifies
all 256 values (`build/qualification/m4-refinement-preserved-compute.log`). Its
source and shader are unchanged.

Local ignored logs/reports: `build/qualification/m4-refinement/`,
`m4-refinement-shared/`, `m4-refinement-full/`, `m4-refinement-negative.json`,
and `windows-refinement/`. Four invalid plans
(missing initial tail, same suffix, coarser suffix and a budget below protected 1x1)
fail before Vulkan. Linux formatting and denied-warning Clippy pass; renderer/assets
tests pass 23/59. All 71 existing SPIR-V artifacts revalidate for Vulkan 1.3.
Windows GNU cross-link/denied-warning Clippy passes all eleven Rust packages;
Wine executes 150 CPU tests. Native Windows/MSVC/GPU execution remains open.

## Reproduce

After building Render Lab and the preceding native fixtures, run:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
XDG_DATA_HOME="$PWD/build/qualification/renderdoc-data" \
PYTHONPYCACHEPREFIX="$PWD/build/qualification/python-cache" \
python3 tools/qualify_texture_refinement.py \
  --reference build/qualification/m4-multi-image-cook.json \
  --texture-cook build/qualification/m4-linear-thin-pattern/cook.json \
  --qrenderdoc build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --output build/qualification/m4-refinement \
  --linear-filter --headless-dri-prime pci-0000_03_00_0
```

The selector is this machine's compositor device, not a portable hardware default.
Use `--shader-dir build/shaders-shared` for the shared interface, or
`--initial-texels 4 --target-texels 8188` for protected-1x1 to full-chain refinement.
The helper measures exact physical coexistence rather than assuming the default
source's allocation size. Its fixed pending capture requires the worker/transfer
to reach the observed frame-21 boundary; other hosts may need a separately
qualified capture frame and must not claim a pending-copy proof from an idle frame.
The harness rejects a pending capture without `vkCmdCopyBufferToImage` evidence.

Remaining gates include tiled GPU demand/feedback, bounded multi-request scheduling,
multi-frame generation retirement/reuse, source I/O latency/cancellation, eviction,
general texture quality and native Windows GPU qualification. M4 stays open.
