# M0-D local reference closeout — 2026-10-10

Reference: RX 7900 XTX, RADV Mesa 26.2.4, Linux Mint 22.3/X11, Vulkan
loader/validation 1.3.275, Rust 1.99.0, Slang 2026.19, RenderDoc 1.46.

## WSI recovery and validation

```bash
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
  cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- \
  --gpu 7900 --wsi-smoke-test
```

Observed excerpt:

```text
WSI INJECTED probe: AcquireOutOfDate at presented frame 10
WSI DRIVER acquire result: Ok((2, true))
WSI DRIVER present result: Ok(true)
WSI INJECTED probe: AcquireSuboptimal at presented frame 30
WSI INJECTED probe: PresentOutOfDate at presented frame 50
Smoke: zero-size draw deferred
Smoke: requested minimize
Smoke: requested restore
WSI INJECTED probe: PresentSuboptimal at presented frame 80
Presented 120 frames
Render Lab teardown: 10 swapchain generations
Validation totals (including teardown): 0 warnings, 0 errors
```

Probes are optional, bounded and separately labeled. Acquire out-of-date skips
the real acquire and exits before frame-fence reset. Acquire suboptimal follows a
successful acquire. Present overrides follow only a successful real present,
preserving the enqueued waits and present fence; real driver errors are never
overridden. Completion checks that all four probes ran. This exercises application
branches, including retirement/recreation on actual GPU work. It does **not**
prove driver error-return enqueue behavior. Genuine suboptimal returns occurred;
genuine out-of-date/fatal driver returns did not. Surface/fullscreen loss, device
loss and out-of-memory present classification have CPU tests, not forced GPU tests.

Without an ICD override, an installed Dozen ICD failed instance creation and
generated one loader warning. RADV-only qualification removes that unrelated ICD
from enumeration; it does not filter or ignore validation messages.

Normal indirect smoke: 120 frames/six generations, zero warnings/errors. Preserved
headless compute: all 256 expected values verified. Both applications' local Rust
checks and all six render shader validations pass. Five Render Lab unit tests pass.
No CI monitoring or GitHub push was performed for this local work.

## RenderDoc capture/replay

The official portable Linux 1.46 build was extracted below ignored `build/`.
A corrected local layer manifest was placed under a qualification-only
`XDG_DATA_HOME`; no system-wide layer registration was needed. Reproducible commands
and GUI inspection steps are in [DEVELOPMENT](../DEVELOPMENT.md#renderdoc-capture-and-replay).

Observed replay excerpt:

```text
Open: <Result: 'Success'>
Replay: <Result: 'Success'>
11 vkCmdDispatch()
19 vkCmdDrawMeshTasksIndirectCountEXT(<1>)
20 Indirect sub-command[0](<1, 1, 1>)
Command/count (1, 1, 1, 1) downstream 2
Image <Result: 'Success'>
```

Captured 64-byte work storage has transforms `(-0.5,0,0.5,0,0.45)` and mesh
count 2. Replay PNG visually contains both RGB triangles and the expected clear
background. Readback occurs only inside this offline debugger check, not in the
renderer. The committed script also checks these values and fails on missing
compute/indirect/mesh actions. Generated reports, images and captures remain under
ignored `build/qualification/`.

**Capture and validation are separate qualification runs.** RenderDoc's default
muted debug output suppressed the deliberate callback check, causing a correctly
reported startup failure. Unmuted combined capture/validation reached rendering,
but produced 2 warnings and 182 image-layout errors around RenderDoc presentation
images (`GENERAL` versus `PRESENT_SRC_KHR`); that run failed. An unmuted capture
with both application and RenderDoc validation disabled completed 180 frames and
replayed correctly. Zero printed totals in that capture are not validation evidence.
No engine layout workaround or relaxed validation success criterion was added.

## Qualification boundary

This closes the bounded local RADV/X11 M0-D reference slice and permits M1 work.
It is not production WSI, a performance benchmark, cross-vendor/Windows/Wayland
qualification, proof of actual fatal driver recovery, or proof of simultaneous
RenderDoc + validation compatibility. No architecture decision changed.
