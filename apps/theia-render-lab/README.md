# Theia Render Lab — M0-D mesh triangle

A separate qualification application; the headless `theia-gpu-test` remains unchanged.
The default path draws a Slang mesh-generated RGB triangle over a dark blue clear
using Vulkan dynamic rendering and Sync2. No vertex/index buffers or vertex shader
are used. Task amplification and GPU-written indirect work remain outstanding. See
[the roadmap](../../docs/ROADMAP.md) for the remaining M0-D gates.

## Run

From the repository root with a desktop session, Vulkan loader/driver, and
`VK_LAYER_KHRONOS_validation` installed:

```bash
python3 tools/compile_render_shaders.py
cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- --smoke-test
```

The driver override is optional and Linux specific. The application obtains native
Wayland/X11/Windows handles from winit/ash-window without hard-coded platform paths.
Wayland and Windows execution still require qualification.

- `--shader-dir <directory>` selects compiled `triangle.mesh.spv` / `triangle.frag.spv`.
  Default: the build checkout's `build/shaders` directory, independent of launch
  working directory. When relocating the executable, pass an explicit directory.
- `--clear-only` runs the qualified bootstrap path without loading shader files.
- `--gpu <name substring>` restricts selection; otherwise prefer the reference RX
  7900 XTX, then any qualified discrete GPU, then other qualified devices.
- `--frames <positive count>` exits after that many successfully queued presents.
- `--smoke-test` requests two resizes, explicitly checks the zero-size draw path,
  requests minimize for 300 ms, restores/resizes, and exits after 120 frames.
  Window-manager actions are requests; X11 need not report zero size on minimize.
- `--no-validation` explicitly disables the layer. It cannot combine with the smoke
  test. The debug messenger and callback-delivery diagnostic remain active.
- Close the window or press Escape to exit.

Validation defaults to explicit layer activation and a pNext request for
synchronization validation. The diagnostic warning confirms callback delivery and
is excluded from warning/error totals. Actual validation errors produce a failing
exit status after teardown; warnings are reported. GPU-assisted validation is not
enabled. Frames are queued presentations, not a measured display/scanout count.

## Profile and ownership

The selected device must expose Vulkan 1.3, mesh + task shaders, dynamic rendering,
Sync2, timeline semaphores, BDA, runtime descriptor arrays, partially bound
resources and sampled-image nonuniform indexing. Requirements and mesh/task limits
are logged for each candidate; the triangle additionally checks 32 mesh invocations
and at least three output vertices / one primitive. Capability support is not
shader-execution evidence.
A graphics+compute queue and a presentation queue are required. Separate families
use concurrent swapchain image sharing.

**This lab additionally requires** `VK_EXT_swapchain_maintenance1` (feature enabled),
`VK_EXT_surface_maintenance1`, and `VK_KHR_get_surface_capabilities2`. This is a
qualification-app restriction, not a change to Theia's approved minimum GPU profile.
It avoids treating device-idle as proof of presentation completion. A future
baseline WSI path or KHR maintenance spelling must be qualified separately; no
silent unsafe cleanup fallback is provided.

Each swapchain image owns its presentation semaphore and presentation fence. A
single frame fence protects the command buffer and acquire semaphore. Presentation
fences and device-idle retire a generation before its views, sync objects and
swapchain and graphics pipeline/layout are released. Out-of-date acquisition returns before resetting the frame
fence; suboptimal/out-of-date presentation requests recreation. Zero extents defer
rendering. RAII also cleans up partially initialized Vulkan objects and keeps the
window/loader/callback state alive through their dependent objects' destruction.

Each swapchain generation creates a pipeline for its actual color format. Viewport
and scissor are dynamic; extent changes require no shader changes. Shader modules
are freed after pipeline creation; pipeline/layout RAII also handles partial failure.

`tools/compile_render_shaders.py` uses offline Slang 2026.19 and `spirv-val`, selecting
`spvMeshShadingEXT` explicitly for the mesh stage (not the NV mesh variant). It
publishes the two outputs only after both validate for Vulkan 1.3. Missing shaders,
compiler errors and pipeline errors fail explicitly; no language fallback occurred.
On Windows use Python 3 with Slang/SPIR-V tools on PATH or pass their executable
paths to the script's `--slangc` / `--spirv-val` options.

The deliberately simple one-frame path and waits during recreation are qualification
scaffolding, not throughput benchmarks or the future production scheduler.

References: [Khronos semaphore reuse and retirement guide](https://docs.vulkan.org/guide/latest/swapchain_semaphore_reuse.html),
[presentation-fence lifetime contract](https://docs.vulkan.org/refpages/latest/refpages/source/VkSwapchainPresentFenceInfoKHR.html),
[present error/enqueue rules](https://docs.vulkan.org/refpages/latest/refpages/source/vkQueuePresentKHR.html).

## Checks

```bash
cargo fmt --manifest-path apps/theia-render-lab/Cargo.toml -- --check
cargo clippy --locked --manifest-path apps/theia-render-lab/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path apps/theia-render-lab/Cargo.toml
```

CI builds/lints/tests both applications on Linux and Windows. A Linux shader job
fetches the [official Slang 2026.19 release](https://github.com/shader-slang/slang/releases/tag/v2026.19)
with a pinned SHA-256 and compiles/validates mesh, fragment and preserved compute
shaders. Hosted CI does not execute GPU workloads. Actual target-GPU evidence is
recorded in [CURRENT_STATUS](../../docs/CURRENT_STATUS.md) and
[the mesh triangle qualification record](../../docs/validation/M0D_MESH_TRIANGLE.md).
