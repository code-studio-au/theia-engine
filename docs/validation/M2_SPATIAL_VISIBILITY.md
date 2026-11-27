# M2 bounded flat 2.5D visibility — 2026-10-10

Executed on RX 7900 XTX / RADV Mesa 26.2.4 / Linux Mint X11, Rust 1.99.0,
Slang 2026.19, validation 1.3.275, RenderDoc 1.46.

## Slice

`--spatial-test` adds a separate Render Lab path. One 64-lane compute group
initializes 32 resident AABBs, builds a direct-addressed 8×8 XY grid, prefix offsets
and contiguous memberships, gathers view-overlapping CSR ranges, deduplicates
candidates, applies exact XYZ bounds overlap, and compacts visible IDs. It writes
the indirect command and draw count without host mapping/readback. One task group
launches the visible number of mesh groups, each emitting one triangle. Mesh reads
are included in explicit work-reuse/publication Sync2 barriers. Device selection
checks the larger dispatch/payload/storage limits rather than assuming M0's limits.

This is a fixed GPU-generated resident scene in a specialized stream. View input
is one bounded host parameter; there are no CPU object/draw lists. Cell placement
does not define entity identity. IDs in this test are frame-local resident indices,
not a durable world-ID or handle schema. M1 renderer integration is not claimed.

## Observed checks

```bash
python3 tools/compile_render_shaders.py
cargo fmt --manifest-path apps/theia-render-lab/Cargo.toml -- --check
cargo clippy --locked --manifest-path apps/theia-render-lab/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path apps/theia-render-lab/Cargo.toml
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
  cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- \
  --gpu 7900 --spatial-test --wsi-smoke-test
```

Nine render shader outputs compile and validate for Vulkan 1.3, including the three
new spatial stages. Seven Render Lab CPU tests pass; the independent CPU grid
reference matches brute-force visibility for 1,000 deterministic randomized views.
GPU smoke cycles all three views, runs four labeled WSI recovery probes, presents
120 frames, recreates ten swapchain generations and reports **zero warnings/errors
including teardown**. Existing indirect, direct-task, mesh-only and clear-only
smoke regressions also pass with zero diagnostics.

RenderDoc uses the separately documented capture workflow with validation disabled;
capture totals are not validation evidence. Each fixed view completes 180 frames,
captures/replays, and matches the independent CPU reference:

| View | Visible count | Candidate mask | Indirect command/count |
|---|---:|---:|---|
| 0 — empty/outside | 0 | `0x00000000` | `(0,1,1,0)` |
| 1 — left/low | 8 | `0x1f1f1f1f` | `(1,1,1,1)` |
| 2 — full | 32 | `0xffffffff` | `(1,1,1,1)` |

Each scene has 64 membership references, exactly matching CPU counts, 65 offsets
and contiguous ID lists. Candidate masks and sorted visible IDs match exactly.
GPU bounds differ from independently evaluated CPU constants by at most
`5.960464477539063e-08`; comparison allows absolute `1e-6`, not bit identity.
Filtered IDs are `[0,1,8,9,16,17,24,25]`. The empty capture has no indirect task
sub-command; the helper distinguishes RenderDoc's zero-count parent marker from
a real mesh-dispatch child. Final replay image center checks identify exactly 0,
8 and 32 triangles. Preserved M0 indirect capture/replay also still passes.

Raw logs, captures, reports and PNGs are ignored local artifacts under
`build/qualification/`. [Capture commands](../DEVELOPMENT.md#renderdoc-capture-and-replay).

## Boundary

This qualifies a bounded local M2 reference slice. The view is an axis-aligned
box, not a complete perspective frustum or HZB. Packed per-cell vertical interval
indexes, dynamic scene membership from M1, general scene ingestion, streaming,
multi-group CSR construction, maximum-occupancy pressure and performance remain
unimplemented/unqualified. Entire resident XY bounds must fit the fixed world.
One-group prefix scanning is correctness scaffolding, not a scalable world builder.
Windows, Wayland and other vendors have not executed this path. Capture and
validation remain separate due to the recorded RenderDoc interaction.
No locked ADR changed; no GitHub push or hosted CI claim is made.
