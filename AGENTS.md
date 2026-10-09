# Codex Instructions — Theia Engine

You are working on **Theia Engine**, an experimental high-performance MMO engine. Before editing, read `docs/CURRENT_STATUS.md`, `docs/DECISIONS.md`, `docs/ARCHITECTURE.md`, `docs/RISKS_AND_VALIDATION.md`, and the relevant subsystem document.

## Rules

1. **Preserve verified work.** Existing headless `theia-gpu-test` executes a Slang SPIR-V compute shader on Radeon RX 7900 XTX; do not repurpose or break it. Add separate `apps/theia-render-lab` for M0-D.
2. **No UE, Unity or custom C++ runtime/game code.** Runtime/source languages: Rust, Go, Slang. Offline third-party executables/libraries may be written in C/C++.
3. **Linux primary, Windows first-class.** Linux Mint, RADV first qualification target; no silent platform-specific assumptions in core APIs.
4. **Vulkan-first, direct explicit control.** Vulkan 1.3 baseline; mandatory mesh + task device capabilities for the intended renderer. Task stages need not appear in every individual pipeline. Ray-tracing hardware is not a requirement.
5. **GPU-first, not GPU-only.** Move bulk coherent, GPU-resident work to compute/mesh/task shaders; keep CPU tasks where I/O, sequential logic, authority, text shaping, latency or synchronization justify them.
6. **Separate authority and presentation.** Server decides gameplay truth; the GPU owns presentation and suitable local visual simulation. Never make the client an authority for combat, movement legality or inventory.
7. **Data-oriented:** dense SoA/AoSoA chunks, stable handles, change streams, explicit allocation ownership and synchronization. Avoid per-entity host calls, draws, allocations, or unnecessary FFI.
8. **Conserve complexity.** Do not implement advanced GI, complete ECS, massive networking, streaming or complex UI in M0. Build and test one verifiable slice at a time.
9. **Be precise about the evidence.** Mark APIs/features untested until tested; do not claim benchmarks or implemented capabilities without logs.
10. **Security and privacy.** Never commit ChatGPT account exports, secrets, `.env`, private keys, credentials or private conversation dumps to a public repo. Use `docs/CHAT_EXPORT.md`.
11. **Record decisions.** Do not overturn locked ADRs without explicitly flagging the tradeoff for user approval; record benchmark-gated fallbacks separately.
12. **Code standards.** Rust 2024 explicit `unsafe` blocks with safety comments for genuinely unsafe operations; RAII ownership for Vulkan objects in reusable library code; error paths should release resources. `cargo fmt`, `cargo clippy`, SPIR-V validation, Vulkan validation and tests as applicable.

## Immediate task (M0-D)

- Inspect current repository and maintain the working headless compute test.
- Create new Render Lab with `ash`, `winit`, `ash-window` as evaluated dependencies.
- Generate native Linux surface/swapchain and feature-check GPU requirements.
- Compile/validate Slang task, mesh and fragment shaders to SPIR-V; fall back to HLSL/GLSL **only if specific Slang features fail qualification**.
- Display a GPU-generated triangle without a conventional vertex buffer; then add GPU-written indirect mesh/task work counts.
- Vulkan debug-utils messages and actual validation-layer activation; correct swapchain recreation and resource lifetime.
- Do not claim runtime success without execution logs from target GPU.

## Preferred workflow

Create a topic branch, implement smallest useful working unit, format and lint, run shader validation and unit checks, open a PR with changes/tests/limitations, and request user to run actual GPU tests on their local RX 7900 XTX where remote hardware cannot validate. Avoid giant speculative refactors.
