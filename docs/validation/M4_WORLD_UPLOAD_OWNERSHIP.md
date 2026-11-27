# M4 independent GPU chunk-upload ownership (2026-10-15)

Render Lab's isolated `--world-upload-test <scene.json>` admits the four checked
cooked quadrants using existing indexed-bundle admission and upload primitives.
It exercises GPU allocations and dedicated transfer fences on RX 7900 XTX/RADV;
the uploaded chunks are **not drawn or sampled**, and no content readback is made.
The three presented frames are clear-only. This is ownership/budget evidence,
not proof of shared chunk visibility/depth/resolve or streaming performance.

On the qualified device, four complete sets own 16 root buffers and 16 images:

| Chunk | Buffer + image physical bytes | Image physical bytes |
|---|---|---|
| 0 | 89376 | 65536 |
| 1 | 89392 | 65536 |
| 2 | 89184 | 65536 |
| 3 | 89520 | 65536 |
| Total | 357472 | 262144 |

The shared CPU pool charges 165148 unique encoded bytes. GPU allocations are
independent per chunk; neither physical charge includes staging, descriptors,
queries, swapchain/targets, driver or host-container overhead.

The executable checks:

- Cancelled ready ownership releases all GPU/CPU charges without publication.
- All four complete chunks upload and publish CPU metadata independently.
- Each buffer/image transfer fence retires before exposing resident owners.
- Removing chunk 0 while retaining a reader keeps its physical charge live.
  An exact aggregate cap rejects another allocation until that reader drops.
- Last-reader release permits chunk 0 to be reuploaded under the exact cap.
- Removing all chunks releases all tracked physical and CPU bytes.
- Aggregate cap 357471 and independent image cap 262143 each reject the fourth
  set after partial submission, with the first three owners/CPU epoch unchanged.
  Pending transfers retire during failure cleanup.

`tools/qualify_world_upload.py` runs the owner probe, missing/invalid fixture
cleanup, pre-Vulkan conflicting-mode rejection and preserved streamed cooked and
10000-proxy mixed routes under an owned native Wayland display. Runtime cases
require zero Vulkan warnings/errors through teardown. Regression routes complete
120 frames each (10 excluded warmup); the cooked route must actually publish a
replacement. Shader sources and the original headless compute app are unchanged.

Local evidence under ignored `build/qualification/`: `world-upload-final/report.json`
and case logs, `world-upload-build.log`, `world-upload-clippy.log` and
`world-upload-tests.log`. No generated assets or binaries are committed.

Renderer CPU tests pass 28; formatting and warning-denied all-target Clippy pass.
Windows GNU cross-link/Clippy passes eleven packages and Wine CPU tests pass
(`windows-world-upload/report.json`). No Windows GPU execution is claimed.

The owner is deliberately private and synchronous with one pending set per call.
No chunk resource has a graphics/descriptor reader in this probe; retaining an
owner is not itself evidence that an arbitrary frame fence has retired. Frame-
loop integration still needs protected fallback coverage, actual renderer readers,
shared identity/depth/resolve, asynchronous demand and GPU-content proof. Native
Windows/GPU remains unqualified, and M4 remains open.
