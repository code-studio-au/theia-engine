# M0-D task amplification qualification — 2026-10-10

Observed on Linux Mint/X11, RX 7900 XTX, RADV Mesa 26.2.4, Vulkan loader and
validation layer 1.3.275, Rust/Cargo 1.99.0, Slang 2026.19. This completes the
roadmap's task amplification step; compute-written indirect work is outstanding.

## Shader qualification and compiler limitation

```bash
python3 tools/compile_render_shaders.py
```

All four outputs compile and pass `spirv-val --target-env vulkan1.3`: standalone
mesh, fragment, task (Slang `amplification` stage), and payload-consuming mesh.
Task and mesh compilation explicitly select `spvMeshShadingEXT`. Task disassembly:

```text
OpEntryPoint TaskEXT %taskMain "main" %taskPayload %gl_LocalInvocationIndex
OpExecutionMode %taskMain LocalSize 32 1 1
OpControlBarrier %uint_2 %uint_2 %uint_264
OpEmitMeshTasksEXT %uint_2 %uint_1 %uint_1 %taskPayload
```

Task output and mesh input use `TaskPayloadWorkgroupEXT`. One task group writes
XY offsets `(-0.5,0)` / `(0.5,0)` and scale `0.45` (20 bytes of logical float data),
synchronizes the group, and uniformly emits two mesh workgroups. Each mesh uses
`SV_GroupID.x` to select its transform and emits three vertices / one triangle.
The host submits `vkCmdDrawMeshTasksEXT(1,1,1)` once. No vertex/index buffers or
host per-triangle submissions are involved; work counts are still direct constants.

The initial documented [task output-parameter form](https://docs.shader-slang.org/en/stable/coming-from-glsl.html),
`taskMain(uint lane : SV_GroupIndex, out payload TaskPayload payload)`, produced
warning E38040 (payload treated as uniform) and then a Slang 2026.19 segmentation
fault when compiled for SPIR-V with these flags. No task output file was produced.
Replacing the task output parameter with explicit `groupshared TaskPayload
 taskPayload` storage compiles, validates and executes. The mesh keeps its `in
payload` parameter. This specific compiler limitation is recorded; no HLSL/GLSL
fallback occurred. [DispatchMesh's official reference](https://docs.shader-slang.org/en/stable/external/core-module-reference/global-decls/dispatchmesh-08.html)
describes downstream mesh group launch and its SPIR-V capability requirement.

## Runtime smoke

```bash
cargo build --locked --manifest-path apps/theia-render-lab/Cargo.toml
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json timeout 30s apps/theia-render-lab/target/debug/theia-render-lab --smoke-test
```

Exit 0, explicit Khronos layer and synchronization-validation request. The
intentional callback-delivery diagnostic is excluded from totals. Output:

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
  amplification limits: true
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
  amplification limits: true
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
Task amplification pipeline created: TaskEXT -> 2 MeshEXT groups -> fragment; 32 lanes per stage, 20-byte transform payload; no vertex/index buffers
Swapchain: 960x540, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 1 ready
Window resized: 960x540
Task amplification pipeline created: TaskEXT -> 2 MeshEXT groups -> fragment; 32 lanes per stage, 20-byte transform payload; no vertex/index buffers
Swapchain: 960x540, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 2 ready
Window resized: 640x360
Task amplification pipeline created: TaskEXT -> 2 MeshEXT groups -> fragment; 32 lanes per stage, 20-byte transform payload; no vertex/index buffers
Swapchain: 640x360, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 3 ready
Window resized: 1280x720
Task amplification pipeline created: TaskEXT -> 2 MeshEXT groups -> fragment; 32 lanes per stage, 20-byte transform payload; no vertex/index buffers
Swapchain: 1280x720, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 4 ready
Smoke: zero-size draw deferred
Smoke: requested minimize
Smoke: requested restore
Task amplification pipeline created: TaskEXT -> 2 MeshEXT groups -> fragment; 32 lanes per stage, 20-byte transform payload; no vertex/index buffers
Swapchain: 1280x720, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 5 ready
Window resized: 800x600
Task amplification pipeline created: TaskEXT -> 2 MeshEXT groups -> fragment; 32 lanes per stage, 20-byte transform payload; no vertex/index buffers
Swapchain: 800x600, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 6 ready
Presented 120 frames
Render Lab teardown: 6 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors
```

## Framebuffer check

A separate `--frames 300` run exited 0:

```text
Presented 300 frames
Render Lab teardown: 2 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors
```

Captured only the application's 960x540 X11 window using `xwd`, decoded it to PNG,
and inspected two separated RGB triangles. Automated pixel checks passed:

| Region | Observed RGB |
|---|---|
| background | (44, 75, 97) |
| gap | (44, 75, 97) |
| left_center | (137, 138, 187) |
| right_center | (137, 138, 187) |
| left_red | (228, 70, 114) |
| left_green | (68, 228, 114) |
| left_blue | (81, 82, 235) |
| right_red | (228, 70, 114) |
| right_green | (68, 228, 114) |
| right_blue | (81, 82, 235) |

Centers are sampled at `(0.25,0.5)` / `(0.75,0.5)` of image dimensions and match
sRGB conversion of interpolated linear `(0.25,0.25,0.5)` within two levels. Color
samples use left `(0.1375,0.59)`, `(0.3625,0.59)`, `(0.25,0.41)` and the same
positions shifted by +0.5 X for the right triangle; each has its expected dominant
channel. Background at `(0.05,0.05)` and gap at `(0.5,0.5)` match the qualified clear.
These are window-image checks, not GPU-buffer readback or a general raster oracle.
Captures and raw logs are ignored artifacts under `build/qualification/`.

## Regressions and error paths

- Mesh-only and clear-only smoke modes each pass 120 frames / six generations,
  with zero warnings/errors including teardown.
- Missing task module after valid amplified mesh and fragment module creation:
  exit 1, actionable error, zero teardown diagnostics. No silent mesh-only fallback.
- `--mesh-only --clear-only` is rejected before window creation with exit 1.
- Formatting, Clippy with denied warnings and both extent tests pass.
- Original headless app and compute source remain unchanged; regression output:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
     Running `apps/theia-gpu-test/target/debug/theia-gpu-test`
Vulkan debug messenger initialized.
GPU: AMD Radeon RX 7900 XTX (RADV NAVI31)
All 256 GPU values verified!
First 8: [1, 3, 5, 7, 9, 11, 13, 15]
Last 8: [497, 499, 501, 503, 505, 507, 509, 511]
```

## Limits

Compute-written descriptors/indirect counts and their barriers remain outstanding.
Windows/Wayland GPU execution, other vendors, split queue families, actual
out-of-date returns, allocation/device/surface failures, GPU-assisted validation
and performance remain unqualified. The validation layer is older than the driver.
Linux/Windows CI covers Rust; Linux shader CI compiles/validates all shader variants
and preserved compute, without GPU execution. The lab's presentation-fence
extension requirements continue to apply; no locked architectural decision changed.
