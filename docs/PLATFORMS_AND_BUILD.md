# Theia Platform, Toolchain and Hardware Policy

## Desktop

Linux Mint/Linux x86-64 is primary; Windows 11 x86-64 first-class. Vulkan 1.3 API baseline with `VK_EXT_mesh_shader` **both feature bits** meshShader and taskShader required by intended hardware profile. GPU vendor/model eligibility determined through feature chain + tested performance, not GPU brand strings. Mobile/browser outside scope, Mac/consoles deferred, Steam Deck evaluate later.

## Graphics API and shaders

Rust `ash` low-level Vulkan host. `winit` for native windows/input and `ash-window` surface integration now compile and execute on the qualified RADV/X11 and isolated nested Wayland paths. Native Windows/Wayland-seat execution remains open. Slang→SPIR-V primary, qualified with `spirv-val` and per-vendor runtime tests. If Slang mesh/task codegen fails specific feature, replace affected shader with HLSL/GLSL SPIR-V while preserving GPU interfaces. Vulkan 1.3 provides dynamic rendering, synchronization2, timeline semaphores and other core facilities; extension features must be queried/enabled explicitly. GPU buffer device address needed; descriptor indexing/bindless feature bits and 32-/64-bit atomic use require explicit feature profiling. No mandatory hardware RT. Vulkan device selection should prefer configured discrete RX 7900 XTX but support arbitrary compatible devices.

## Development reference GPU (OBSERVED)

AMD Radeon RX 7900 XTX, Navi 31, vendor `0x1002` / device `0x744c`. Mesa RADV 26.2.4 (kisak-mesa PPA), Vulkan device 1.4.354, Linux Mint. AMD Ryzen 7 7800X3D iGPU also enumerates. Driver JSON path `/usr/share/vulkan/icd.d/radeon_icd.json`; `VK_DRIVER_FILES` avoids Dozen llvmpipe enumeration. Reported meshShader=true, taskShader=true, maximum task payload=16,384, max output vertices/primitives=256. These limits are not default meshlet sizes. Slang compiler 2026.19 at `~/dev/tools/slang/bin/slangc`; Rust/Cargo 1.99.0; ash 0.38.0+1.3.281. Khronos validation layer 1.3.275 installed.

## GPU feature tiers

G1 baseline: Vulkan 1.3 plus mesh/task, compute, storage, BDA, required descriptor indexing, Sync2, timeline semaphores. G2/G3 increase residency, GI quality, shadows, animation and resolution budget, not authority/visibility rules. Minimum GPU model and VRAM **not yet known**. HDR scene-linear renderer, SDR mandatory, HDR/VRR optional; GPU UI native-res and consistent HDR white point. Host text shaping/IME/accessibility semantic bridge permitted even though rasterization is GPU.

## Audio

CPU-first; Kira/CPAL candidates. GPU state should not require synchronous readback for every sound emitter. Audio/UI/input must work independently of GPU frame cadence.

## M0 build constraints

Use pinned Cargo.lock. Prefer `cargo fmt`, `cargo clippy`, `spirv-val --target-env vulkan1.3`; validation for debugging, disabled for benchmarks. Build and test Linux first, Windows CI from the beginning, actual cross-vendor GPU tests as hardware becomes available. `target/` and `build/` generated files excluded from Git; Slang source tracked. Robust RAII and explicit unsafe only in reusable Vulkan infrastructure.
