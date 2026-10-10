# Theia — Actual Implementation Status (2026-10-10)

**This file separates observed evidence from planned systems.** The original handoff
records user-supplied terminal evidence. The following update was independently
executed in the local checkout on 2026-10-10.

## M2 bounded spatial/visibility reference slice — independently executed

Render Lab `--spatial-test` builds direct-addressed XY CSR memberships and gathers
candidates, applies height-aware bounds culling, compacts visible IDs and renders
GPU-generated indirect task/mesh work. RX 7900 XTX passes the 120-frame WSI smoke
with ten swapchain generations and zero validation warnings/errors. Separate
RenderDoc captures match independent CPU bounds/CSR/masks/IDs/counts for empty,
filtered and full views; replay images contain exactly 0, 8 and 32 triangles.

Nine render stages validate; seven CPU tests pass, including 1,000 randomized
view comparisons against brute force. All four earlier rendering modes and the
M0 capture workflow pass regression checks. This qualifies the bounded M2 reference
slice, not dynamic M1 scene integration, perspective/HZB culling, packed vertical
indexes, content streaming or scale. Evidence:
[M2 spatial qualification](validation/M2_SPATIAL_VISIBILITY.md).

## M1 bounded GPU reference slice — independently executed

Separate `apps/theia-ecs-lab` executes Slang initialization, ordered structural
mutation, parallel component integration and retained-reader compute. RX 7900 XTX
passes 526 batches / 8,717 commands against the CPU oracle with zero synchronization
validation warnings/errors including teardown. Both archetype migration directions,
compaction, 64-bit world identities, stale/conflicting handles, chunk/slot exhaustion,
retained old GPU data and permanent generation exhaustion are checked.

Four ECS shader outputs pass SPIR-V validation. Local Rust checks pass; missing
shader and unmatched-device construction failures clean up without diagnostics.
M1's bounded reference slice is qualified; production parallel mutation, real
concurrent rendering-reader overlap, general schemas, scale and other platforms
remain open. CPU readbacks are diagnostic only. Evidence:
[M1 GPU ECS qualification](validation/M1_GPU_ECS.md).

## M1 CPU oracle — previously qualified

`crates/theia-ecs` now provides the bounded dense-column/generational-handle CPU
reference. Six local unit tests pass, including 10,000 deterministic randomized
operations checked against an independent map. Formatting and Clippy with denied
warnings pass. This is reference scaffolding; the subsequent bounded GPU execution is recorded above. Scope and next GPU checks: [M1 qualification contract](M1_GPU_ECS.md).

## M0-D reference qualification closeout — independently executed

- RX 7900 XTX / RADV / X11: `--wsi-smoke-test` completes all four explicitly
  injected acquire/present recovery probes, 120 successful queued presentations
  and ten swapchain generations, with zero validation warnings/errors including
  teardown. Genuine driver suboptimal results were observed during resize.
  Genuine driver out-of-date, surface/device-loss and allocation-failure returns
  remain unobserved; fatal present classification has CPU tests only.
- RenderDoc 1.46 captures and replays compute + indirect task/mesh work. Diagnostic
  replay verifies command/count `(1,1,1,1)`, two downstream mesh groups and GPU
  transforms, and exports the correct two-triangle image. Runtime has no readback.
- Capture runs explicitly disable validation. Combined capture/validation produced
  layout diagnostics; standalone synchronization-validation runs pass. Muted
  RenderDoc output also suppresses the deliberate callback check. These tooling
  limitations are recorded, not treated as a validated combined workflow.
- Normal indirect smoke and preserved headless compute (256/256) pass. Formatting,
  Clippy with denied warnings and five unit tests pass locally. Changes remain
  local at user request; no new hosted CI result is claimed.

The **local RADV/X11 M0-D reference slice is qualified**; M1 can begin. Windows,
Wayland, other vendors, GPU-assisted validation, real fatal WSI returns and
performance remain separate qualification work. No locked ADR changed.
Evidence and commands: [M0-D closeout](validation/M0D_CLOSEOUT.md).

## M0-D GPU indirect work — independently executed

- Default Render Lab runs Slang compute to write a device-local 64-byte work buffer:
  indirect task group dimensions, draw count, transforms and downstream mesh count.
  It executes `vkCmdDrawMeshTasksIndirectCountEXT` with no host mapping/count
  readback. `drawIndirectCount` is queried/enabled explicitly.
- Resource-scoped Sync2 barriers order buffer reuse and publish compute writes to
  indirect command/count reads and task storage reads. The task copies transforms
  into its shared payload and launches the GPU-selected number of mesh groups.
- Radeon smoke cycles qualification inputs 0, 1, 2 while resizing/minimizing/restoring:
  120 queued presentations, six swapchain generations, zero warnings/errors including
  teardown. Three fixed-input 120-frame runs also pass; window pixels verify a clear
  frame, one left triangle and two triangles respectively.
- `--direct-task`, `--mesh-only` and `--clear-only` preserve prior paths. All three
  smoke regressions and unchanged headless compute (256/256) pass. Missing indirect
  task after work-resource creation exits cleanly; invalid/mismatched CLI inputs fail.
- Six render shader outputs pass Vulkan 1.3 SPIR-V validation; Rust formatting,
  Clippy with denied warnings and both extent tests pass. CI includes the new stages.

Evidence: [GPU indirect work qualification](validation/M0D_INDIRECT_WORK.md).
This qualifies a bounded work-generation test, not scene culling or GPU ECS.
The subsequent local reference closeout is recorded above.
Windows/Wayland GPU runtime, other vendors, GPU-assisted validation and performance
remain unqualified. No locked architectural decision changed.

## M0-D task amplification — previously qualified

- Slang task + payload-consuming mesh shaders compile to SPIR-V 1.6 and pass
  Vulkan 1.3 validation. Task output uses `TaskEXT`, `TaskPayloadWorkgroupEXT`,
  and `OpEmitMeshTasksEXT(2,1,1)`; both stages use 32 lanes.
- Default Render Lab submits one task group, which launches two mesh groups and
  supplies two offsets plus a scale. Each group generates an RGB triangle with
  no vertex/index buffers. `--mesh-only` preserves the qualified single-triangle
  path; `--clear-only` preserves the bootstrap.
- RX 7900 XTX / RADV / X11: task smoke passes 120 frames and six swapchain
  generations, with zero validation warnings/errors including teardown. A
  300-frame run also passes; own-window checks verify both triangle centers,
  six color regions and the clear gap between them.
- Slang 2026.19 crashed for the documented `out payload` task-parameter form.
  Explicit `groupshared` payload storage compiles, validates and executes; no
  HLSL/GLSL fallback or locked architecture change occurred.
- Formatting, Clippy with denied warnings and both extent tests pass. Mesh-only,
  clear-only and unchanged headless compute regressions pass; missing task module
  errors clean up the already-created mesh/fragment modules without diagnostics.
  Shader CI now includes task and both mesh variants.

Evidence and limitations: [task amplification qualification](validation/M0D_TASK_AMPLIFICATION.md).
The subsequent compute-written indirect qualification is recorded above. **M0-D
remains open for qualification closeout.** Windows/Wayland GPU execution, other vendors and GPU-assisted
validation remain unqualified; no performance claim is made.

## M0-D mesh-only triangle — previously qualified

- Slang 2026.19 compiles mesh + fragment stages to SPIR-V 1.6; both pass
  `spirv-val --target-env vulkan1.3`. Mesh uses `MeshShadingEXT` / `MeshEXT`,
  local size 32x1x1, three output vertices and one primitive. No language fallback.
- Render Lab now creates a mesh-only graphics pipeline and executes
  `vkCmdDrawMeshTasksEXT(1,1,1)` on RX 7900 XTX / RADV / X11. There is no vertex
  shader, vertex buffer or index buffer. **At this qualification, task shaders had not executed yet.**
- Mesh smoke: 120 queued presentations, six swapchain generations, zero validation
  warnings/errors including teardown. A 300-frame run also passes; own-window image
  checks verify the clear background, interpolated center and red/green/blue regions.
- Pipeline/layout/module RAII covers creation errors and recreation for the actual
  swapchain format. `--clear-only` preserves the bootstrap path; `--shader-dir`
  supports explicit shader locations. Missing shaders fail with clean teardown.
- Rust formatting, Clippy with denied warnings and the two extent tests pass.
  The original compute app/source remain unchanged. CI now also compiles/validates
  shaders with the official Slang release, pinned by SHA-256.

Evidence and limitations: [mesh triangle qualification](validation/M0D_MESH_TRIANGLE.md).
The subsequent task amplification qualification is recorded above;
compute-written indirect mesh/task work remains outstanding. **M0-D remains open.** Windows/Wayland GPU
execution, other vendors and GPU-assisted validation remain unqualified.

## M0-D bootstrap — previously qualified

- Added separate `apps/theia-render-lab`: ash 0.38, ash-window 0.13, winit 0.30.13,
  pinned lockfile. Native X11 surface and FIFO swapchain execute on RX 7900 XTX,
  RADV Mesa 26.2.4; dynamic rendering displays a dark blue clear.
- Queries/enables Vulkan 1.3 mesh/task capability, BDA, descriptor-indexing subset,
  timeline semaphores, dynamic rendering and Sync2. **At bootstrap qualification, mesh/task pipelines had not
  executed.** The lab additionally requires EXT swapchain/surface maintenance for
  presentation-fence retirement; this does not redefine the approved engine profile.
- Explicitly enables `VK_LAYER_KHRONOS_validation` (installed version 1.3.275),
  requests synchronization validation via instance pNext, and verifies callback
  delivery with an intentional diagnostic excluded from totals.
- Corrected hazards found in the first synchronization-validation run. Final smoke:
  **120 queued presentations, six swapchain generations, zero warnings/errors
  including teardown**. Requests resize/minimize/restore and explicitly verifies
  zero-size deferral. A separate 300-frame run and own-window capture verify the clear.
- Unsupported `--gpu` selection exits with an actionable error and zero teardown
  diagnostics. Zero frame count is rejected. Extent tests: **2/2 passed**.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
  pass for both apps. Existing compute source is unchanged; Slang compute compile,
  `spirv-val --target-env vulkan1.3`, and **256/256 GPU values** pass again.
- Added Linux/Windows Rust CI (build/lint/unit tests, without GPU execution).
  Wayland, Windows runtime, other vendors, split queue families, device/surface loss,
  and GPU-assisted validation remain **untested**. The smoke log does not prove an
  actual `ERROR_OUT_OF_DATE_KHR` return was exercised.

Commands, observed logs and limitations: [M0-D bootstrap qualification](validation/M0D_BOOTSTRAP.md).
The subsequent mesh-only triangle qualification is recorded above. Task amplification
and GPU-written indirect work counts remain outstanding. **M0-D remains open.**

## Original handoff — completed and verified

1. Linux Mint, AMD RX 7900 XTX (RADV NAVI31) detected via `vulkaninfo`; RADV Mesa 26.2.4, device Vulkan API 1.4.354.
2. Vulkan driver filtering with `VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json` selects RADV and removes unrelated Dozen/llvmpipe in reported enumeration (integrated Radeon still appears).
3. `VK_EXT_mesh_shader` advertised; `taskShader=true`, `meshShader=true`; maxTaskPayloadSize 16384; maxMeshOutputVertices=256; maxMeshOutputPrimitives=256. **No mesh/task GPU pipeline has executed yet.**
4. Slang `slangc -version` prints **2026.19** and PATH configured from `~/dev/tools/slang/bin`.
5. `shaders/compute/buffer_test.slang`: set 0 binding 0 RWStructuredBuffer<uint>, 64 threads/workgroup, writes `index * 2 + 1` for 0…255.
6. Slang compiled shader to `build/shaders/buffer_test.spv`; `spirv-val --target-env vulkan1.3` succeeded with no errors; `spirv-dis` shows `GLCompute`, `LocalSize 64 1 1`, set 0/binding 0, SPIR-V 1.6.
7. Rust 1.99.0 / Cargo 1.99.0; `ash` 0.38.0+1.3.281 added to `apps/theia-gpu-test`.
8. Headless Rust test loaded Vulkan using ash, selected RX 7900 XTX, created buffer, compute pipeline, dispatched four workgroups and read back 256 integers; first values 1,3,5,7,9,11,13,15; last values 497…511. **256/256 passed**.
9. Vulkan debug-utils messenger added, created, destroyed; application prints `Vulkan debug messenger initialized.`; executed under `VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation` with no diagnostics displayed. **Layer activation/message delivery not independently proven**, and synchronization validation/GPU-assisted validation not yet independently run.
10. `cargo fmt` and `cargo clippy` completed with no reported warnings after unused import cleanup; debug run completed normally.
11. Local project directory renamed to `~/dev/theia-engine`; Git repository initialized, nested Cargo-created `.git` issue fixed and user reported initial commit completed. **GitHub remote creation/push not confirmed in this conversation.**

## Files expected in the user's local checkout

```
~/dev/theia-engine/
  .gitignore
  apps/theia-gpu-test/Cargo.toml
  apps/theia-gpu-test/Cargo.lock
  apps/theia-gpu-test/src/main.rs
  shaders/compute/buffer_test.slang
  build/shaders/buffer_test.spv   # generated/ignored
```

This handoff ZIP does **not** include copies of the user's latest local application sources; Codex should inspect the Git checkout. An earlier uploaded `main.rs` in the conversation had the debug callback; local version may have changed after that.

## Original handoff plan — M0-D

Create **separate** `apps/theia-render-lab` (cargo new `--vcs none`), native window and Vulkan surface/swapchain, compile and run Slang task+mesh+fragment shaders, render one GPU-generated triangle. Then compute-generated indirect mesh task dispatch, Vulkan validation and correct resize/recreation. Keep headless compute test intact as regression. No ECS, GI, game server or editor yet.

## Original handoff caveats (see the update above for new evidence)

- Early one-shot compute test uses HOST_VISIBLE|HOST_COHERENT memory, `queue_wait_idle`, and large explicit unsafe block. Correct for qualification, **not** production data path. Production should use device-local buffers, staging, fences/timelines, RAII and scoped unsafe.
- `VK_EXT_debug_utils` messenger creation is observed but no proof of callback delivery; check enabled validation instance layer and submit diagnostic message or enumerate instance layers explicitly.
- Cargo had transient crates.io DNS resolution failure, then subsequent compilation succeeded.
- Shader path is relative to working directory `build/shaders/buffer_test.spv`: run executable from repo root until asset path resolution improves.
- User has not reported running render-lab or pushing a GitHub repo; do not mark complete without evidence.
