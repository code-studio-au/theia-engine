# M0-D mesh-only triangle qualification — 2026-10-10

Observed on the development machine by Codex. Linux Mint/X11; RX 7900 XTX,
RADV Mesa 26.2.4; Vulkan loader/validation layer 1.3.275; Rust/Cargo 1.99.0;
Slang 2026.19. This is the roadmap's mesh-only triangle step, not M0-D completion.

## Shader qualification

```bash
python3 tools/compile_render_shaders.py
```

Both outputs pass `spirv-val --target-env vulkan1.3`. The mesh compilation explicitly
selects `spvMeshShadingEXT`; disassembly includes:

```text
OpCapability MeshShadingEXT
OpExtension "SPV_EXT_mesh_shader"
OpEntryPoint MeshEXT %meshMain "main" ...
OpExecutionMode %meshMain OutputPrimitivesEXT 1
OpExecutionMode %meshMain OutputVertices 3
OpExecutionMode %meshMain OutputTrianglesEXT
OpExecutionMode %meshMain LocalSize 32 1 1
```

Both mesh output and fragment input explicitly use color location 0. Source entry
names are `meshMain` and `fragmentMain`; emitted SPIR-V entries are named `main`.
No HLSL/GLSL fallback was needed. The mesh generates positions, colors and triangle
indices; the host submits one mesh workgroup with no vertex/index buffer.

The CI compiler asset was also downloaded and qualified locally:
`slang-2026.19-linux-x86_64-glibc-2.28.tar.gz` from the
[official release](https://github.com/shader-slang/slang/releases/tag/v2026.19).
Its release SHA-256 is
`bd7afcb0956dee9304f44ed115e9aaa7140704271b5c3294bcb237e3ae67adc7`;
checksum, extraction and script compilation/validation all passed.

## Runtime qualification

```bash
cargo build --locked --manifest-path apps/theia-render-lab/Cargo.toml
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json timeout 30s apps/theia-render-lab/target/debug/theia-render-lab --smoke-test
```

Exit 0; explicit Khronos layer and synchronization-validation pNext request.
The intentional diagnostic is excluded from warning/error totals. Output:

```text
Validation layer explicitly enabled: VK_LAYER_KHRONOS_validation; synchronization validation requested via pNext
Vulkan WARNING: Theia diagnostic: debug callback delivery confirmed (intentional, excluded from totals)
Native surface created; loader 1.3.275
GPU candidate: AMD Radeon RX 7900 XTX (RADV NAVI31); driver radv / Mesa 26.2.4 - kisak-mesa PPA
  Vulkan 1.3: true
  VK_KHR_swapchain: true
  VK_EXT_mesh_shader: true
  meshShader: true
  triangle mesh limits: true
  taskShader: true
  dynamicRendering: true
  synchronization2: true
  timelineSemaphore: true
  bufferDeviceAddress: true
  runtimeDescriptorArray: true
  descriptorBindingPartiallyBound: true
  shaderSampledImageArrayNonUniformIndexing: true
  lab presentation fences: true
  Mesh limits: 256 vertices, 256 primitives; task payload 16384 bytes; preferred task/mesh invocations 64/128
GPU candidate: AMD Ryzen 7 7800X3D 8-Core Processor (RADV RAPHAEL_MENDOCINO); driver radv / Mesa 26.2.4 - kisak-mesa PPA
  Vulkan 1.3: true
  VK_KHR_swapchain: true
  VK_EXT_mesh_shader: true
  meshShader: true
  triangle mesh limits: true
  taskShader: true
  dynamicRendering: true
  synchronization2: true
  timelineSemaphore: true
  bufferDeviceAddress: true
  runtimeDescriptorArray: true
  descriptorBindingPartiallyBound: true
  shaderSampledImageArrayNonUniformIndexing: true
  lab presentation fences: true
  Mesh limits: 256 vertices, 256 primitives; task payload 16384 bytes; preferred task/mesh invocations 64/128
Selected GPU: AMD Radeon RX 7900 XTX (RADV NAVI31); graphics/compute family 0, present family 0; presentation-fence retirement enabled
Mesh-only triangle pipeline created: MeshEXT + fragment, 32 lanes, 3 vertices, 1 primitive; no vertex/index buffers
Swapchain: 960x540, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 1 ready
Window resized: 960x540
Mesh-only triangle pipeline created: MeshEXT + fragment, 32 lanes, 3 vertices, 1 primitive; no vertex/index buffers
Swapchain: 960x540, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 2 ready
Window resized: 640x360
Mesh-only triangle pipeline created: MeshEXT + fragment, 32 lanes, 3 vertices, 1 primitive; no vertex/index buffers
Swapchain: 640x360, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 3 ready
Window resized: 1280x720
Mesh-only triangle pipeline created: MeshEXT + fragment, 32 lanes, 3 vertices, 1 primitive; no vertex/index buffers
Swapchain: 1280x720, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 4 ready
Smoke: zero-size draw deferred
Smoke: requested minimize
Smoke: requested restore
Mesh-only triangle pipeline created: MeshEXT + fragment, 32 lanes, 3 vertices, 1 primitive; no vertex/index buffers
Swapchain: 1280x720, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 5 ready
Window resized: 800x600
Mesh-only triangle pipeline created: MeshEXT + fragment, 32 lanes, 3 vertices, 1 primitive; no vertex/index buffers
Swapchain: 800x600, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 6 ready
Presented 120 frames
Render Lab teardown: 6 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors
```

A separate 300-frame run of the final Rust code exited 0:

```text
Presented 300 frames
Render Lab teardown: 2 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors
```

Captured only the application's 960x540 X11 window via `xwd` and inspected the PNG.
Pixel checks passed (sample coordinates expressed as fractions of width/height):

| Region | Coordinate | Observed RGB |
|---|---|---|
| Background | (0.05, 0.05) | (44, 75, 97) |
| Center interpolation | (0.50, 0.50) | (137, 137, 187) |
| Red region | (0.25, 0.70) | (228, 70, 113) |
| Green region | (0.75, 0.70) | (69, 228, 113) |
| Blue region | (0.50, 0.30) | (81, 82, 235) |

Background matches the previously qualified clear. Center matches sRGB conversion
of interpolated linear `(0.25, 0.25, 0.5)` within two levels. Each color-region
sample has the expected dominant channel. These are window-image checks, not
GPU-buffer readback or a general rasterization oracle. Captures/raw logs remain
ignored generated artifacts in `build/qualification/`.

## Regressions and error paths

- `--smoke-test --clear-only`: 120 frames, six generations, exit 0, zero diagnostics.
- Missing all shaders: exit 1 with compilation instructions and zero teardown
  warnings/errors. Missing fragment with a valid mesh module: exit 1 with zero
  diagnostics, exercising cleanup after partial shader-module creation.
- Missing compiler executable: compilation script exits 1 with an actionable error.
- Formatting, Clippy with denied warnings and both extent tests pass.
- Headless compute source unchanged; target-GPU regression:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
     Running `apps/theia-gpu-test/target/debug/theia-gpu-test`
Vulkan debug messenger initialized.
GPU: AMD Radeon RX 7900 XTX (RADV NAVI31)
All 256 GPU values verified!
First 8: [1, 3, 5, 7, 9, 11, 13, 15]
Last 8: [497, 499, 501, 503, 505, 507, 509, 511]
```

## Outstanding qualification

Task shader compilation/execution and GPU-written indirect mesh/task counts are
next. Wayland/Windows runtime, other vendors, separate queue families, GPU-assisted
validation, actual out-of-date returns, allocation failure, device/surface loss and
performance remain unqualified. Rust CI runs on Linux/Windows; shader CI runs on
Linux without GPU execution. The validation layer is older than the driver, so
zero diagnostics qualify this observed setup rather than proving universal safety.
