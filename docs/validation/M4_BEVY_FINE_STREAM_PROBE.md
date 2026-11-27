# Optional Bevy reference: one cooked fine-stream lifetime gate

Locally qualified on 2026-10-16, Linux/RADV Radeon RX 7900 XTX, native Wayland
inside the owned headless Weston compositor. This is a controlled correctness
step toward equivalent streaming. The ordinary comparison still preloads/replays;
no camera-driven Bevy streamer or streaming performance comparison is qualified.
No Theia runtime, shader, dependency or locked architecture decision changes.

## Implemented operation

The optional reference executable accepts `--streaming-probe <region 0..3>`.
It requires a newly exported fixture with eight actual cooked bundle paths and
at least 140 route frames. This mode rejects ordinary snapshot, HUD and GPU-timing
flags: its three automatic lifecycle captures and completion waits are diagnostic
instrumentation, unsuitable for performance measurements.

Startup semantically prevalidates the eight source graphs, matching the current
Theia fixture policy. Only the four coarse graphs enter the shared
`WorldResidency` / `ProtectedWorld` encoded store. The replay's decoded world
payloads and input JSON bytes are released; the source-ground metadata remains
for CPU proxy queries. GPU world resources are constructed from the actual cooked
files, rather than the fixture's decoded position/texel arrays.

One capacity-one file worker reopens the selected fine bundle at logical frame
20. Bounded reads authenticate the manifest filename, exact child lengths/hashes,
geometry/material/UV bindings and cooked KTX2 textures through `theia-assets`.
The isolated adapter expands meshlet triangles into Bevy meshes with the original
unlit factors, UVs and texels. No alternate runtime asset format or new dependency
is introduced.

A complete shared CPU reservation precedes candidate asset creation. Fine
entities remain hidden while their mesh, image and prepared-material IDs are
absent from any required render-asset map or while queue work is incomplete.
A render-world check runs after rendering, flushes queue writes with a submission
and waits for completion. Main-world publication needs that acknowledgement and
holds until at least frame 40. Pending ready data never changes the four-region
selection. The protected coarse resources remain owned throughout.

At frame 80, selection returns to coarse while fine handles remain retained.
A complete coarse frame and preceding queue work retire before candidate entities
and main assets are removed. The ID-only watch deliberately holds no strong
handles: pinned Bevy unloads render assets on last-handle **Unused**, not just
main-world **Removed**. After all fine render entries disappear and the queue
retires again, the fine encoded publication/reservation releases. No fine GPU
entry is eligible for selection during this process. Worker shutdown joins; file
reads and shutdown have no hard latency/cancellation deadline.

Normal completion requires a terminal state, completed captures and the original
coarse encoded charge. Missing completion cannot publish a successful report.
The history is bounded to sixteen rows; this one-operation schedule uses at most
seven. Selection, encoded and logical charges are recorded per frame.

## Accounting and controls

`--probe-cpu-budget` limits unique encoded owners in the shared store. It excludes
raw file-worker buffers, decoded/transient representations, fixture metadata and
process RSS. `--probe-upload-budget` is a separate adapter-specific **logical**
charge: f32 position/normal/UV arrays, copied RGBA texels and two vec4 material
fields. It includes coarse plus pending/retiring fine payloads. It excludes
allocator slabs/alignment, samplers/bind groups, targets, proxy/ground resources,
staging and driver overhead. Physical GPU bytes are explicitly null. Removing a
render-asset entry does not prove an allocator slab or physical VRAM was returned.
These logical bytes cannot be substituted for Theia's Vulkan allocation charges.

For the current source fixture:

| Ownership state, region 2 | Unique encoded bytes | Adapted logical bytes |
|---|---:|---:|
| Four protected coarse graphs | 142040 | 428544 |
| Coarse plus pending/published/retiring fine | 165852 | 542976 |
| Fine render entries retired, encoded owners released | 142040 | 428544 |

The exact CPU/logical coexistence limits pass. One byte below either limit keeps
coarse coverage, with no fine publication. A coarse-only logical limit also
retains coverage. Source/admission errors remain fatal; capacity deferrals are
explicitly separate.

`--probe-cancel-frame 21..39` cancels the request before publication. Frame 21
exercises cancellation before consuming the worker result. Frame 30 exercises
an already GPU-ready candidate, retaining its reservation through render-asset
removal and completion acknowledgement. Neither publishes fine data.

## Executed evidence

`build/qualification/bevy-stream-probe-final/report.json` passes ten native plans:
all four regions, exact coexistence, one-byte-short CPU/logical limits, coarse-only
capacity, worker cancellation and GPU-ready cancellation. There are 180 route
frames per plan, pre-roll 60, Immediate, validation enabled, and 0/10000 proxies.
Logs contain no VUID, validation-error, ERROR or panic diagnostics; the benign
window-decoration warning remains.

Independent checks verify every recorded selection and phase charge against
source-resource hash unions and adapted-array sizes. Twenty-five captures cover
initial coarse, published fine and restored/cancelled/deferred coarse states:
**3411 source ray/colour probes**, maximum RGB error one byte. Observed selections
are saved in separate `observed-fixture.json` files; they are explicitly not
Theia's replay selections. Screenshot-request selections must match their recorded
frame. CPU pose/source-ground checks cover **350000 proxy poses**, maximum ground
error below **1.67e-7 metres**. These checks do not prove animation, authority,
network scale or arbitrary source asset support.

Five negative plans reject invalid/unscoped controls, insufficient coarse CPU or
logical capacity and a corrupted private copy of a cooked UV child. The source
corruption is startup qualification; post-startup source-failure recovery and
mid-operation shutdown are not separately qualified. Earlier intermediate failed
lifetime attempts are not promoted as final evidence.

The final executable also passes preserved native comparison runs:

- `bevy-stream-preserved-immediate/comparison.json`: 10000 proxies, 140 frames,
  GPU timestamps and both HUDs enabled, plus frozen captures at 0/139.
- `bevy-stream-preserved-fifo/comparison.json`: default FIFO and GPU queries off,
  zero proxies, 60 frames.

`bevy-stream-probe-{build,clippy,tests}.log` records release build, denied-warning
Clippy and all ten reference tests passing. Rust formatting and changed Python's
Python 3.8 syntax pass. Product Rust/Windows packages and shaders are unchanged;
this optional Bevy tool still has no native Windows GPU qualification.

## Reproduce

Build the matching reference executable first, using the README's local extracted
Wayland development-library setup if needed. Then run from the repository root:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
PYTHONPYCACHEPREFIX="$PWD/build/qualification/python-cache" \
python3 tools/qualify_bevy_stream_probe.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --source-report build/qualification/bevy-uncapped-final/02-10000-proxies/theia/00-streamed-10000-proxies/report.json \
  --headless-dri-prime pci-0000_03_00_0 \
  --output build/qualification/bevy-stream-probe-final
```

Use an existing completed, overlay-disabled Theia streamed report for the same
scene, with at least 140 frames. The qualifier exports authenticated cooked paths,
runs the matrix on an owned display and writes a failed summary before starting.
Controlled population overrides are labelled as such. It copies corrupt-source
inputs into its own output directory, never modifying the original assets.

## Next gate

Generalize the proven load/publication/removal acknowledgement into repeated
camera demand using the same 20/35-metre bands, priorities and bounded fine slots.
Compare demand, coverage, publication lag and missing detail, allowing readiness
schedules to differ. A later timing profile must separate capture/readback
instrumentation, cold/warm file-cache policy and backend allocation scopes.
Identical replayed selections alone are not equivalent streaming.
