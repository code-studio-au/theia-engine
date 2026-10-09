# Theia — Actual Implementation Status (2026-10-09)

**This file separates observed evidence from planned systems.** All information below is based on terminal output supplied by the user in the conversation; Codex must independently check the local checkout.

## Completed and verified

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

## Immediate next goal — M0-D

Create **separate** `apps/theia-render-lab` (cargo new `--vcs none`), native window and Vulkan surface/swapchain, compile and run Slang task+mesh+fragment shaders, render one GPU-generated triangle. Then compute-generated indirect mesh task dispatch, Vulkan validation and correct resize/recreation. Keep headless compute test intact as regression. No ECS, GI, game server or editor yet.

## Known caveats

- Early one-shot compute test uses HOST_VISIBLE|HOST_COHERENT memory, `queue_wait_idle`, and large explicit unsafe block. Correct for qualification, **not** production data path. Production should use device-local buffers, staging, fences/timelines, RAII and scoped unsafe.
- `VK_EXT_debug_utils` messenger creation is observed but no proof of callback delivery; check enabled validation instance layer and submit diagnostic message or enumerate instance layers explicitly.
- Cargo had transient crates.io DNS resolution failure, then subsequent compilation succeeded.
- Shader path is relative to working directory `build/shaders/buffer_test.spv`: run executable from repo root until asset path resolution improves.
- User has not reported running render-lab or pushing a GitHub repo; do not mark complete without evidence.
