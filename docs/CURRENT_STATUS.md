# Theia — Actual Implementation Status (2026-10-10)

**This file separates observed evidence from planned systems.** The original handoff
records user-supplied terminal evidence. The following update was independently
executed in the local checkout on 2026-10-10.

## M0-D task amplification — independently executed

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
Next outstanding task: **compute-written GPU work descriptors and indirect
mesh/task dispatch**, with explicit barriers and no host count readback. **M0-D
remains open.** Windows/Wayland GPU execution, other vendors and GPU-assisted
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
