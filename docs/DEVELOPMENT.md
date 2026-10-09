# Theia — Linux Developer Workflow

## Verified reference environment

Linux Mint, RX 7900 XTX (Navi 31), Mesa RADV 26.2.4, `VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json`, Rust + Cargo 1.99.0, ash 0.38, Slang 2026.19 at `~/dev/tools/slang/bin`. Local repository: `~/dev/theia-engine`. Windows is a first-class intended platform but no Windows runtime execution was observed yet.

## Known-good commands

```bash
cd ~/dev/theia-engine
slangc -version
slangc shaders/compute/buffer_test.slang -entry computeMain -stage compute -target spirv -profile spirv_1_6 -o build/shaders/buffer_test.spv
spirv-val --target-env vulkan1.3 build/shaders/buffer_test.spv
spirv-dis build/shaders/buffer_test.spv | head -45
cargo fmt --manifest-path apps/theia-gpu-test/Cargo.toml
cargo clippy --manifest-path apps/theia-gpu-test/Cargo.toml
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation cargo run --manifest-path apps/theia-gpu-test/Cargo.toml
```

Expected GPU output: `GPU: AMD Radeon RX 7900 XTX (RADV NAVI31)`, `All 256 GPU values verified!`, first 8 odds 1…15, last 8 odds 497…511. Existing program prints `Vulkan debug messenger initialized.`. No validation warnings were observed; messenger callback delivery not independently proven.

## Project hygiene

`cargo new --bin --vcs none apps/theia-render-lab` (avoid nested `.git`). Commit `.slang`, `Cargo.lock`, `Cargo.toml` and source. Ignore `target/`, `**/target/`, `build/`, compiled `.spv`, logs and private ChatGPT data exports. Use topic branches, PRs and CI. Run `git status` before pushing.

## Direct Rust/Vulkan caution

Early compute test uses host-visible memory and `queue_wait_idle` on purpose and has large `unsafe` scopes. Production modules should add RAII, scoped unsafe with rationale, validated lifetime dependencies, device-local buffers and staged transfers. Query Vulkan feature chains; only enable supported features; do not assume mesh/task support from API version alone.

## Shader pipelines

Slang `[[vk::binding(0,0)]] RWStructuredBuffer<uint>` shader uses `[numthreads(64,1,1)]` entry `computeMain`. SPIR-V renamed entry `main` (disassembly verified). Compiled command `-profile spirv_1_6`; validate with Vulkan 1.3 target. M0-D qualification should test Slang task/mesh stages and alternate frontend only if specific codegen fails.

## If build fails

- Missing `cargo`: install Rust via official rustup.
- Crates.io DNS temporarily failed earlier: check `getent ahostsv4 static.crates.io` and HTTPS; later builds succeeded.
- Missing Slang: `~/dev/tools/slang/bin/slangc -version` expected 2026.19.
- Extra Vulkan ICD warning: use `VK_DRIVER_FILES` set to exact RADV JSON manifest.
- Multiple GPUs: explicitly select physical GPU using device properties, not ordinal 0 alone.
- Shader file relative path currently assumes launch from repository root.
