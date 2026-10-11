# M4 complete indexed-bundle streaming evidence (2026-10-11)

Executed locally on RX 7900 XTX, RADV Mesa 26.2.4, Vulkan loader/validation 1.3.275.
No hosted CI, Windows GPU, performance or production streaming claim.

The viewer alternates the preceding two-image and alias v2 bundles (identities
in [texture evidence](M4_TEXTURES.md)). One worker reads the entire bounded
fixed graph. All roots/leaves pass complete CPU admission/reservation before
four buffer plus all image transfers are submitted. Every dedicated fence must
be ready, and the sole frame fence retired, before a complete graphics generation
and CPU epoch are published. Count changes rebuild the descriptor layout/pipeline.
Generation retirement includes device idle. Partial budget failures release
pending transfers before CPU reservations and preserve the current set.

Observed physical buffer/image charges:

| Set | Resource bytes | Image bytes | GPU upload staging bytes | CPU encoded bytes |
| --- | ---: | ---: | ---: | ---: |
| Two-image | 12704 | 8192 | 4544 | 6528 |
| Alias | 8416 | 4096 | 4336 | 5416 |
| Both live | 21120 | 12288 | One pending set above | 11012 unique |

CPU sharing deduplicates the 932-byte primary KTX2; GPU uploads intentionally
duplicate shared images between sets. Manifest bytes are CPU-only. GPU budgets
exclude work/target/descriptor/sampler/pool/staging allocation; staging is logged
separately. I/O input buffers and decoded admission temporaries are excluded
from CPU ownership accounting. These are bounded qualification budgets, not
complete process RSS or total device VRAM caps.

Six standalone configurations each present 120 frames and publish five complete
sets at observed frames 22/42/62/82/102: default, shared identity, reverse
direction, physical exact 21120, CPU exact 11012 and image exact 12288.
Each executes WSI resize/minimize/zero-size/recovery probes, reaching fifteen
generations with zero validation warnings/errors through teardown.

Physical cap 21119, CPU cap 11011 and image cap 12287 each keep the initial
complete set, record zero publications/five deferrals and finish 120 frames/ten
generations with zero diagnostics. Physical/image cancellation logs return CPU
live ownership to exactly 6528 bytes after every failed attempt. Initial physical
cap 12703 and CPU cap 6527 reject safely before rendering with zero diagnostics.

Four separate RenderDoc executions disable capture validation, present 180 frames
and verify initial frame 10, alias frame 30, restored frame 50 and shared alias
frame 30. Each passes 85 CPU/GPU samples including all four triangles, exact
geometry/factor/UV/table bytes, complete texture pixels, work IDs, identity/depth
and sampled colours. Preceding depth/RGB tolerances apply. These runs reach eight
complete publications. They prove the active set rather than only checking logs.

A copied replacement leaf is removed after initial graphics initialization:
worker I/O fails, no replacement publishes and teardown has zero diagnostics.
A copied indexed fragment shader is removed after initial target creation:
replacement uploads finish, new target creation fails, no CPU epoch publishes
and teardown has zero diagnostics. Source/driver/shader errors terminate this
viewer; budget deferral is the only retry/fallback policy. Target failure does
not guarantee rollback to the prior graphics generation.

Renderer unit tests (14), formatting and denied-warning Clippy pass. Assets
remain covered by the 33-test CPU suite; no shader sources changed. Preserved
headless compute executes all 256 expected values again. Native Windows, all
16 textures, frame overlap, multi-frame hazards, target-memory budgets, general
streaming scheduling and performance remain unqualified.

Artifacts under ignored `build/qualification/`:

- `m4-bundle-stream-{smoke,shared,reverse,physical-exact,physical-reject,cpu-exact,cpu-reject,image-exact,image-reject}.log`.
- `m4-bundle-stream-{initial,replacement,restored,shared-replacement}/report.json`, captures/images and capture logs.
- `m4-bundle-stream-negative/{late-missing-leaf,late-missing-shader,initial-physical,initial-cpu}.log`.
- `m4-bundle-stream-{tests,headless}.log`.

M4 remains open. Next establish a bounded LOD error/selection contract; general
hierarchy/seams and streamed terrain/mip/virtual texture work still remain.
