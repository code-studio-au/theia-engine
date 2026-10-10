# M0-D GPU indirect work qualification — 2026-10-10

Observed on Linux Mint/X11, RX 7900 XTX, RADV Mesa 26.2.4, Vulkan loader and
validation layer 1.3.275, Rust/Cargo 1.99.0, Slang 2026.19. This implements the
roadmap's bounded compute-written work / indirect mesh-task dispatch gate.

## Shader and buffer interface

`python3 tools/compile_render_shaders.py` compiles and validates all six render
outputs for Vulkan 1.3: two mesh variants, fragment, direct task, indirect task,
and work-generation compute. No language fallback occurred. The original triangle
and task-amplification sources remain unchanged.

Compute writes one device-local 64-byte storage/indirect allocation. Its byte ABI:
command group X/Y/Z at 0/4/8, draw count at 12, float2 offsets at 16/24, scale at
32, and downstream mesh count at 36. All used fields are written each frame.
`drawIndirectCount` is explicitly queried/enabled. The
[Khronos indirect-count command](https://docs.vulkan.org/refpages/latest/refpages/source/vkCmdDrawMeshTasksIndirectCountEXT.html)
reads both the command and the count from this buffer; maximum draw capacity is 1,
stride is 12, and the count offset is aligned and distinct from the command.

The host pushes a bounded test input 0..2 to compute and dispatches one compute
group. Compute clamps it, writes zero draws for zero input or one task group for
positive input, and publishes the selected downstream mesh count and transforms.
The task loads the descriptor at set 0/binding 0, synchronizes shared payload data,
and launches that many mesh groups. The CPU never maps or reads the buffer, builds
per-object draws, or supplies the indirect command/count values. This is a functional
qualification test; no scene culling or general GPU scheduler is implemented.

Resource-scoped Sync2 barriers:
- Before reuse: indirect reads, task reads and prior compute writes -> compute writes.
- After generation: compute storage writes -> indirect command/count reads and task
  storage reads. Compute and graphics use the same graphics+compute queue.

The buffer, memory, descriptor pool/set/layout and compute pipeline have RAII
ownership and persist across swapchain recreation. Graphics pipelines use the same
descriptor layout and are retired before work resources, then the device. Error
paths clean up partial ownership; shader modules are freed after pipeline creation.

## Cycling workload and WSI smoke

```bash
cargo build --locked --manifest-path apps/theia-render-lab/Cargo.toml
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json timeout 30s apps/theia-render-lab/target/debug/theia-render-lab --smoke-test
```

Exit 0. Inputs cycle 0,1,2 every 20 queued presentations, twice, while requesting
resize/minimize/restore and explicitly checking zero-size deferral. Logs report
qualification inputs, not buffer readback. Explicit validation and synchronization
validation are enabled; the intentional callback diagnostic is excluded from totals.

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
  drawIndirectCount: true
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
  drawIndirectCount: true
  bufferDeviceAddress: true
  runtimeDescriptorArray: true
  descriptorBindingPartiallyBound: true
  shaderSampledImageArrayNonUniformIndexing: true
  lab presentation fences: true
  Mesh limits: 256 vertices, 256 primitives; task payload 16384 bytes; preferred task/mesh invocations 64/128
Selected GPU: AMD Radeon RX 7900 XTX (RADV NAVI31); graphics/compute family 0, present family 0; presentation-fence retirement enabled
GPU indirect work initialized: 64-byte device-local buffer; command at 0, count at 12, transforms/mesh count at 16..40; no mapping/readback
GPU indirect graphics pipeline created: task reads compute-generated work descriptor; mesh uses shared transforms
Swapchain: 960x540, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 1 ready
Window resized: 960x540
GPU indirect graphics pipeline created: task reads compute-generated work descriptor; mesh uses shared transforms
Swapchain: 960x540, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 2 ready
Indirect qualification input: 0 work items (GPU produces command/count; no host readback)
Indirect qualification input: 1 work items (GPU produces command/count; no host readback)
Window resized: 640x360
GPU indirect graphics pipeline created: task reads compute-generated work descriptor; mesh uses shared transforms
Swapchain: 640x360, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 3 ready
Indirect qualification input: 2 work items (GPU produces command/count; no host readback)
Window resized: 1280x720
GPU indirect graphics pipeline created: task reads compute-generated work descriptor; mesh uses shared transforms
Swapchain: 1280x720, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 4 ready
Smoke: zero-size draw deferred
Smoke: requested minimize
Smoke: requested restore
GPU indirect graphics pipeline created: task reads compute-generated work descriptor; mesh uses shared transforms
Swapchain: 1280x720, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 5 ready
Indirect qualification input: 0 work items (GPU produces command/count; no host readback)
Window resized: 800x600
GPU indirect graphics pipeline created: task reads compute-generated work descriptor; mesh uses shared transforms
Swapchain: 800x600, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 6 ready
Indirect qualification input: 1 work items (GPU produces command/count; no host readback)
Indirect qualification input: 2 work items (GPU produces command/count; no host readback)
Presented 120 frames
Render Lab teardown: 6 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors
```

## Fixed-input image checks

For each N=0,1,2, ran `--frames 120 --work-items N` and captured only the 960x540
application window with `xwd`, decoded to PNG and inspected. All three runs exit 0:

```text
Input 0:
Presented 120 frames
Render Lab teardown: 2 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors

Input 1:
Presented 120 frames
Render Lab teardown: 2 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors

Input 2:
Presented 120 frames
Render Lab teardown: 2 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors
```

| Input | Left center RGB | Right center RGB |
|---|---|---|
| 0 | (44, 75, 97) | (44, 75, 97) |
| 1 | (137, 138, 187) | (44, 75, 97) |
| 2 | (137, 138, 187) | (137, 138, 187) |

Centers are `(0.25,0.5)` and `(0.75,0.5)` of image dimensions. Zero input leaves
both clear; one input produces only the left triangle; two produce both. Each
active center matches sRGB conversion of interpolated `(0.25,0.25,0.5)` within two
levels. Three color-region samples per active triangle have their expected dominant
channels; inactive regions, background `(0.05,0.05)` and central gap `(0.5,0.5)`
all match clear RGB `(44,75,97)`. Samples use left `(0.1375,0.59)`, `(0.3625,0.59)`,
`(0.25,0.41)`, shifted +0.5 X for right. These end-to-end window-image checks do not
measure the exact buffer values independently or provide a general raster oracle.
Captures/raw logs remain ignored artifacts under `build/qualification/`.

## Regression and error-path evidence

Direct task, mesh-only and clear-only smoke modes each pass 120 presentations / six
swapchain generations with zero validation warnings/errors including teardown.
Missing indirect task after compute pipeline, work buffer/descriptors and mesh/
fragment modules have been created exits 1 with an actionable error and zero cleanup
diagnostics. Invalid work counts and work inputs in regression modes fail before
initialization. Formatting, Clippy with denied warnings and both extent tests pass.

Headless compute source is unchanged; target-GPU regression:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
     Running `apps/theia-gpu-test/target/debug/theia-gpu-test`
Vulkan debug messenger initialized.
GPU: AMD Radeon RX 7900 XTX (RADV NAVI31)
All 256 GPU values verified!
First 8: [1, 3, 5, 7, 9, 11, 13, 15]
Last 8: [497, 499, 501, 503, 505, 507, 509, 511]
```

## Closeout and limits

M0-D remains open for remaining WSI error-return qualification and the RenderDoc
capture/debug workflow before M1. RenderDoc is not installed in this environment;
no RenderDoc capture is claimed. Actual out-of-date returns, allocation/device/
surface failures, Windows/Wayland GPU execution, other vendors, split queue families,
GPU-assisted validation and performance remain unqualified. The validation layer
is older than the driver. Linux/Windows CI builds/lints/tests Rust and Linux shader
CI compiles/validates all outputs without GPU execution. Lab presentation-fence
extension requirements remain in effect; no locked architectural decision changed.
