# Theia Render Lab — M0-D / M2 / M3 reference slices

A separate qualification application; the headless `theia-gpu-test` remains unchanged.
The default path dispatches Slang compute to write an indirect command, draw count,
transforms and downstream mesh count into device-local storage. Vulkan reads the
command/count and the task shader consumes the work descriptor; mesh groups generate
the triangles. No vertex/index buffers, vertex shader, buffer mapping or CPU count
readback are used. See [the roadmap](../../docs/ROADMAP.md) for M0-D closeout gates.

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

- `--shader-dir <directory>` selects compiled SPIR-V. Indirect mode uses
  `work.compute.spv`, `triangle.indirect.task.spv`, `triangle.amplified.mesh.spv`,
  and `triangle.frag.spv`. Default: the build checkout's `build/shaders`, independent
  of working directory. Pass an explicit directory when relocating the executable.
- `--work-items <0|1|2>` supplies a fixed qualification input to compute (default 2).
  It requires indirect, spatial or visibility mode without the smoke test. This is a bounded
  test input, not an authoritative gameplay decision.
- `--direct-task` runs the qualified two-triangle task path using `triangle.task.spv`.
- `--spatial-test` runs the separate bounded M2 grid/height visibility path. Here
  `--work-items 0|1|2` selects empty, left/low or full views over 32 GPU-generated
  resident bounds. It does not provide a host object/draw list. Smoke tests cycle
  those views. See [M2 evidence](../../docs/validation/M2_SPATIAL_VISIBILITY.md).
- `--visibility-test` runs M3 identity/depth and material resolve. GPU-generated
  overlapping near/far triangles write `R32G32_UINT` frame-local instance/primitive
  references and `D32_SFLOAT` depth. A fullscreen mesh pass reconstructs barycentrics
  and samples two materials using a nonuniform runtime descriptor array.
  `--work-items 0|1|2` selects empty, near-only or near+far geometry.
- `--visibility-reference x y width height count` prints the independent CPU pixel
  oracle without Vulkan. See [M3 evidence](../../docs/validation/M3_VISIBILITY_MATERIALS.md).
- `--spatial-reference 0|1|2` prints independent CPU reference JSON and exits before
  creating a window/Vulkan instance; used only for offline correctness comparison.
- `--mesh-only` runs the qualified single-triangle mesh path using `triangle.mesh.spv`.
- `--clear-only` runs the bootstrap without loading shader files. These three
  regression modes are mutually exclusive and do not create the indirect work buffer.
- `--gpu <name substring>` restricts selection; otherwise prefer the reference RX
  7900 XTX, then any qualified discrete GPU, then other qualified devices.
- `--frames <positive count>` exits after that many successfully queued presents.
- `--smoke-test` requests two resizes, explicitly checks the zero-size draw path,
  requests minimize for 300 ms, restores/resizes, and exits after 120 frames.
  Indirect mode also cycles inputs 0, 1, 2 every 20 presented frames, twice.
  Window-manager actions are requests; X11 need not report zero size on minimize.
- `--wsi-smoke-test` adds four labeled one-shot recovery probes to the smoke test:
  acquire/present out-of-date and suboptimal. Injected acquire failure skips the
  API call; injected present results follow a successful real present. Driver
  returns are logged separately. This validates application recovery, not the
  driver's error-enqueue behavior. See [closeout evidence](../../docs/validation/M0D_CLOSEOUT.md).
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
and at least three output vertices / one primitive, plus 32 task invocations,
the payload/shared-memory budget and two downstream mesh groups. Capability support is not
shader-execution evidence.
This lab also queries/enables Vulkan 1.2 `drawIndirectCount` for the count-buffer API.
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
`spvMeshShadingEXT` explicitly for task and mesh stages (not the NV mesh variant). It
publishes all fourteen outputs only after they validate for Vulkan 1.3. Missing shaders,
compiler errors and pipeline errors fail explicitly; no language fallback occurred.
On Windows use Python 3 with Slang/SPIR-V tools on PATH or pass their executable
paths to the script's `--slangc` / `--spirv-val` options.

Indirect mode uses one 64-byte `STORAGE_BUFFER | INDIRECT_BUFFER` allocation, bound
once at set 0/binding 0 for compute and task stages. Compute writes every used field:

| Byte offset | Value |
|---|---|
| 0..12 | `VkDrawMeshTasksIndirectCommandEXT`: task group X/Y/Z |
| 12..16 | Draw count (0 or 1) |
| 16..24 / 24..32 | Two XY offsets |
| 32..36 | Scale |
| 36..40 | Downstream mesh count (0..2) |

The host pushes a four-byte test input and dispatches compute once. A buffer barrier
orders prior indirect/task reads and compute writes before reuse; another publishes
compute writes to `DRAW_INDIRECT/INDIRECT_COMMAND_READ` and
`TASK_SHADER_EXT/SHADER_STORAGE_READ`. The graphics pipeline then records
`vkCmdDrawMeshTasksIndirectCountEXT`, with one command slot and `maxDrawCount=1`.
The GPU chooses both the draw count and task group dimensions. Zero input writes
zero draws; positive input writes one task group, which launches one or two mesh
groups using the compute-written transforms. The task publishes these transforms
through synchronized `groupshared` payload storage. No host reads the work buffer.

The work buffer, descriptor resources and compute pipeline survive swapchain extent
changes. Teardown retires submissions and releases graphics pipelines before work
resources, then destroys the Vulkan device. RAII also releases partially initialized
buffer/memory/descriptors/pipelines on errors.

The documented `out payload` task parameter form crashed Slang 2026.19 during
qualification. The explicit `groupshared` form emits `TaskPayloadWorkgroupEXT`,
compiles/validates/executes successfully, and preserves the Slang language choice.
See the qualification record for this compiler limitation.

The deliberately simple one-frame path and waits during recreation are qualification
scaffolding, not throughput benchmarks or the future production scheduler.

## Bounded M2 spatial mode

This mode requires at least 32 downstream mesh groups and a 64-lane compute group
with 524 shared bytes; limits are queried. Its 9,728-byte device-local arena contains
indirect command/count, visible IDs, 32 AABBs, 64 cell counts, 65 CSR offsets and
space for 2,048 membership references. Direct cell addressing is `x + y * 8`.
Multi-cell memberships are deduplicated into a 32-bit candidate mask, then exact
XYZ overlap filters height before atomically compacting visible IDs. A single task
group launches the visible number of mesh groups; each emits one triangle using
its selected bounds. Mesh storage reads participate in both work-reuse and
compute-publication barriers. No host buffer mapping/count readback occurs.

All 32 complete XY bounds are resident inside the fixed world. Scene initialization
is GPU-generated qualification data in a specialized stream, not M1 ECS integration
or content streaming. IDs are frame-local indices, not durable world identities.
Views are axis-aligned bounds, not a complete perspective/HZB implementation. One
compute group and its prefix scan are reference scaffolding; larger worlds need
multi-dispatch/scalable allocation qualification. Maximum membership occupancy,
arbitrary scene inputs and GPU scale have not been executed.

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
with a pinned SHA-256 and compiles/validates both task variants, both mesh variants,
fragment, work-generation compute and the preserved headless compute shader. Hosted CI does not execute GPU workloads. Actual target-GPU evidence is
recorded in [CURRENT_STATUS](../../docs/CURRENT_STATUS.md) and
[the indirect work qualification record](../../docs/validation/M0D_INDIRECT_WORK.md).
RenderDoc capture/replay commands and the validation interaction are documented
in [DEVELOPMENT](../../docs/DEVELOPMENT.md#renderdoc-capture-and-replay).

## Bounded M3 visibility/material mode

Format support is checked for integer identity color+sampling, sampled D32 depth,
and sampled/transfer-destination RGBA8 materials. Each swapchain generation owns
identity/depth images, two 1×1 material images, sampler, descriptor sets and two
pipelines. Attachment writes are published to fragment sampled reads; transfer
clears initialize material images before sampling. RAII retires these resources
with the swapchain and cleans up partial pipeline construction.

Identity is `(instance+1, primitive)` with zero instance meaning background. Both
fixture instances contain primitive zero; these values are not durable world IDs
or approved production bit budgets. Depth selects the smaller near triangle over
the larger far triangle. The resolve reconstructs affine barycentrics for fixed
`w=1` geometry, validates depth/identity, and uses nonuniform material indexing into
a runtime descriptor array with two populated entries and explicit LOD zero.
Slots are fully populated; descriptor holes and updates during rendering are not
qualified. Constant 1×1 textures qualify material selection, not UV patterns,
mip gradients or production asset residency. General perspective reconstruction,
compressed geometry/material tables and transparency remain later work.
