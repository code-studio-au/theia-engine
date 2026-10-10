# M0-D clear-screen qualification — 2026-10-10

Observed on the development machine by Codex, not inferred from feature support.
Linux Mint/X11; AMD RX 7900 XTX, RADV Mesa 26.2.4; Vulkan loader and validation
layer 1.3.275; device Vulkan 1.4.354. Rust/Cargo 1.99.0; Slang 2026.19.

## Final smoke command and output

```bash
cargo build --locked --manifest-path apps/theia-render-lab/Cargo.toml
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json timeout 30s apps/theia-render-lab/target/debug/theia-render-lab --smoke-test
```

Exit status 0. Synchronization validation is requested by the application's pNext,
not dependent on environment injection. The deliberate diagnostic proves callback
delivery; it is not a layer-generated warning. The initial run found two hazards
per frame: acquisition-to-layout-transition and transition-to-presentation. The
corrected acquisition barrier participates in the color-output semaphore wait;
the final barrier exposes writes through an explicit destination scope. Final output:

```text
Validation layer explicitly enabled: VK_LAYER_KHRONOS_validation; synchronization validation requested via pNext
Vulkan WARNING: Theia diagnostic: debug callback delivery confirmed (intentional, excluded from totals)
Native surface created; loader 1.3.275
GPU candidate: AMD Radeon RX 7900 XTX (RADV NAVI31); driver radv / Mesa 26.2.4 - kisak-mesa PPA
  Vulkan 1.3: true
  VK_KHR_swapchain: true
  VK_EXT_mesh_shader: true
  meshShader: true
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
Swapchain: 960x540, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 1 ready
Window resized: 960x540
Swapchain: 960x540, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 2 ready
Window resized: 640x360
Swapchain: 640x360, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 3 ready
Window resized: 1280x720
Swapchain: 1280x720, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 4 ready
Smoke: zero-size draw deferred
Smoke: requested minimize
Smoke: requested restore
Swapchain: 1280x720, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 5 ready
Window resized: 800x600
Swapchain: 800x600, B8G8R8A8_SRGB, FIFO, 4 images
Swapchain generation 6 ready
Presented 120 frames
Render Lab teardown: 6 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors
```

## Visual and regression evidence

A second run with `--frames 300` exited 0, logging:

```text
Presented 300 frames
Render Lab teardown: 2 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors
```

Captured only the application's X11 window with `xwd`. Inspected the resulting
960x540 PNG: uniform dark blue, center RGB `(44, 75, 97)`, consistent with the sRGB
conversion of linear clear `(0.025, 0.07, 0.12)`. The capture and raw execution logs
are generated artifacts under ignored `build/qualification/`, not source assets.

Recompiled the existing Slang compute shader with SPIR-V 1.6; `spirv-val
--target-env vulkan1.3` exited 0. Headless regression output:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
     Running `apps/theia-gpu-test/target/debug/theia-gpu-test`
Vulkan debug messenger initialized.
GPU: AMD Radeon RX 7900 XTX (RADV NAVI31)
All 256 GPU values verified!
First 8: [1, 3, 5, 7, 9, 11, 13, 15]
Last 8: [497, 499, 501, 503, 505, 507, 509, 511]
```

Both applications pass formatting, Clippy with denied warnings, and `cargo test`.
Render Lab has two tests for zero/fixed extents and variable-extent clamping;
headless app has no unit tests. An unmatched GPU filter exits 1 with zero validation
diagnostics after cleanup; `--frames 0` exits 1 before window initialization.

## Limits of this evidence

- This implements the roadmap's clear-screen bootstrap only. No mesh/task/fragment
  shader pipeline or GPU-generated indirect work exists in Render Lab yet.
- Minimize/restore are window-manager requests; the zero-size test is explicit
  because X11 does not necessarily emit a zero-sized resize on minimize.
- No actual out-of-date return was logged; recovery is implemented but that error
  return, allocation failures and device/surface loss are not runtime-qualified.
- Wayland, Windows execution, other GPU vendors, and separate graphics/present
  families remain untested. CI is Rust build/lint/unit tests, not GPU qualification.
- The installed validation layer is older than the driver. Zero diagnostics are
  evidence for this tested setup, not proof against every possible Vulkan misuse.
- GPU-assisted validation is not enabled; no performance benchmark is claimed.
- The lab requires EXT presentation-fence support. See its README for why and for
  the distinction from the approved Vulkan 1.3 engine hardware profile.
