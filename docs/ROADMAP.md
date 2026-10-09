# Theia — Milestone Roadmap and Acceptance Gates

The goal is an **early end-to-end playable vertical slice**, not years of isolated rendering research. Work can advance on GPU client and authoritative server tracks in parallel once contracts stabilize.

| Milestone | Deliverable | Exit criterion |
|---|---|---|
| M0-A | Device/RADV and shader compiler qualification | Report GPU capabilities, Slang compile and SPIR-V validation |
| M0-C | Headless GPU compute | Rust ash dispatch produces 256 expected values — **DONE** |
| M0-D | Render Lab | Native window + Vulkan swapchain + task/mesh-generated triangle; validation and resize work |
| M1 | GPU ECS minimal | Correct dense components, handles, small structural mutations, epoch safety vs CPU oracle |
| M2 | Flat 2.5D visibility | Spatial candidate ranges, GPU culling, indirect mesh/task draws without host per-object lists |
| M3 | Visibility buffer/materials | Correct primitive identity/depth, material attribute reconstruction, bindless sampling |
| M4 | Static world/streaming | Cooked meshlets+LOD, terrain, virtual textures, immutable cells, resource budget/fallback |
| M5 | GI/temporal | Static streamed SDF + dynamically updated shared radiance cache, motion vectors, reconstruction |
| M6 | Characters/UI | GPU animation/skin, shared pose baseline, Bézier text, UI, host shaping/IME |
| M7 (parallel) | Local authoritative game | Rust server tick, navigation/collision, movement, interactions, simple combat |
| M8 (parallel) | Network vertical slice | Quinn DATAGRAM+streams, state deltas, server validated play, persistence, two clients |
| M9 | Scale and operations | Cohort replication, NetLOD, worker authority transfer, profiling, reliability/load exercises |

## Immediate M0-D implementation plan

1. Audit existing repo and current Rust version, dependencies and Slang source.
2. Create minimal executable `apps/theia-render-lab` without nested Git (`cargo new --vcs none`).
3. Native `winit` event loop, Vulkan 1.3 instance + debug utils + surface; select compatible GPU/queue families and query mesh/task features/limits.
4. Create swapchain, image views, dynamic rendering render targets, basic sync and resize handling; start clear-screen-only.
5. Implement minimal Slang mesh-only + fragment triangle; compile, validate and **execute**.
6. Add task shader amplification, validate output and framebuffer.
7. Add compute-written GPU work descriptors + indirect mesh-task dispatch, correct barriers, no host readback for count.
8. Add explicit results/correctness logs, Vulkan validation, support debugging with RenderDoc. Document per-vendor evidence.

## Exit gates / nonfunctional requirements

- No reported validation errors for implemented M0-D paths; image acquisition/present and teardown safe.
- Repeated resize/minimize/restore safe; out-of-date swapchain handled.
- Slang compiler errors and shader pipeline errors diagnosed, no silent fallback.
- Known-good M0-C compute test unchanged and remains runnable.
- `cargo fmt`, `cargo clippy`, shader validation and at least basic CI pass.
- Any hardware-specific workaround explicitly documented; no claim of universal performance.

## Don't build in M0-D

General ECS, complex render graph, radiance cache, geometry LOD streaming, network replication, audio, full UI, advanced multi-queue GPU scheduling or 10k-player load harness. Each is an intentionally later step.
