# Optional Bevy reference: repeated camera-demand streaming

Qualified locally on 2026-10-16: Linux/RADV Radeon RX 7900 XTX, native Wayland
inside owned headless Weston, pinned Bevy 0.19.1 / wgpu 29.0.4. The separate
reference now supports repeated actual cooked-file loading and retirement under
Theia's shared demand policy. This is a correctness gate, not a streaming
throughput, physical-memory or whole-engine performance comparison. Theia product
runtime/shaders and locked decisions are unchanged.

## Operation and ownership

`--camera-stream` builds on the verified fine-probe lifecycle. Four coarse graphs
remain protected. The shared `ProtectedWorld::plan` uses actual committed fine
residency, horizontal source bounds, 20 m load / 35 m retain bands, deterministic
nearest/ID priority and `--stream-fine-slots 1..4` (default two). Camera inputs come
from the authenticated route fixture; Theia's recorded selections are ignored.
The default matched visual replay and controlled single-fine probe remain separate.

One file worker or pending GPU candidate operates at a time. A complete encoded
reservation precedes hidden candidate creation. Prepared mesh/image/material maps
and a completed queue submission acknowledge readiness before fine publication.
When a candidate becomes unwanted, its worker result is discarded or its GPU set
retires without publication. Main-world handles release only after a complete
coarse frame and earlier queue work retire; encoded release waits for last-handle
Unused, render-entry removal and another queue completion. Other committed fine
regions remain owned while one region retires.

Idle scheduling prefers obsolete committed removals before new requests. An
existing pending operation completes or cancels before another committed region's
retirement begins. This serialized lifecycle ordering differs from Theia's
renderer; desired plans, slot limits and coverage are shared, but publication and
selection timestamps need not match. GPU-ready candidates are rechecked against
current demand before publication. Failed capacity is blocked for that region at
the current CPU epoch, avoiding repeated identical reads until residency changes.

An explicit CPU per-view request selects coarse during retirement even while old
fine encoded ownership remains pinned. Every selected representation must exist
in its CPU epoch. This reference use does not adopt per-view LOD in the Theia
renderer or implement a geometry-error policy.

After the recorded route, the reference stops requesting, cancels pending work,
retires every fine graph and waits for capture completion. This unmeasured drain
must restore the original coarse charge before success is written; it has a
600-update progress bound, without a hard I/O latency guarantee. Final counters
and `wall_seconds` include this drain; recorded frame samples do not. Final
publication/eviction totals therefore differ from route-only counters. GPU waits
retain the existing ten-second completion timeout.

`--stream-cpu-budget` charges unique encoded resources through the same shared
store. `--stream-upload-budget` charges adapted f32 position/normal/UV arrays,
copied texels and material uniform payloads, including pending/retiring owners.
The older probe budget names remain aliases. Work/targets, allocator slabs and
alignment, bind groups, proxy/ground data, staging, driver overhead, raw I/O and
fixture metadata have separate/excluded scopes. Physical GPU bytes remain null;
render-entry removal does not prove VRAM was returned. These adapted logical
charges cannot be equated with Theia's Vulkan allocation bytes.

Event history retains 64 rows and a dropped-prefix count. Per-frame records keep
current phase, candidate, desired/load/removal/deferred sets, committed IDs,
selection, CPU/protected epochs, pending ownership, charges and counters. They do
not duplicate history. Full source-ground metadata remains for CPU proxy queries;
world GPU payloads are recreated from real cooked file loads.

## Native evidence

Ignored `build/qualification/bevy-camera-final/report.json` passes nine plans:
zero and 10000 proxies, one-slot policy, tight CPU/logical coexistence, coarse-only
CPU/logical capacity, a rapid route and a long repeated route. All use Immediate,
actual validation, pre-roll 60 and 1280x720. The source fixtures run Theia's
unchanged streamed renderer under validation, then export the shared camera and
cooked sources. Normal routes repeat every 120 logical frames; rapid repeats
every twelve. Logical frame time is scripted, not real-time camera travel.

Across **6120 recorded Bevy frames**, independent Python source/geometry oracles
check every demand plan, deterministic unblocked request priority, true capacity
deferrals, complete coverage, CPU epochs, protected coarse epochs and unique
encoded/adapted logical charges. Final drain returns to coarse-only ownership.

| Plan | Recorded frames | Fine publications / retirements, including drain | Relevant observation |
|---|---:|---:|---|
| Zero / 10000 proxies | 660 each | 43 / 43 each | Two-slot repeated ownership |
| One slot | 660 | 38 / 38 | Slot bound retained |
| Tight CPU / tight logical | 660 each | 27 / 27 each | 32 corresponding capacity deferrals each |
| Coarse-only CPU / logical | 660 each | 0 / 0 | Four deferrals each, coverage retained |
| Rapid | 180 | 4 / 4 | 26 GPU-ready candidates cancelled without publication |
| Long | 1320 | 88 / 88 | 64 retained history rows, 465 dropped |

Forty-nine request-associated captures pass **6400 colour/ray probes**, maximum
RGB error one byte. CPU/source-ground checks pass **430000 proxy poses**. Observed
selections are saved separately from Theia replay selections, and screenshot
request metadata must match its recorded frame. These are visual proxies, not
networked or animated players.

`bevy-camera-supplement/report.json` additionally passes four-slot simultaneous
fine residency, FIFO camera streaming and the preserved controlled-fine probe.
Supplementary checks use the same independent ledger/pose/pixel routines; the
four-slot run observes all four fine graphs resident at once. Capture frames are
0/85/95/659 for four slots, 0/85/659 for FIFO; the legacy probe uses its original
three lifecycle captures and original strict pixel sampling.

`bevy-camera-preserved/comparison.json` preserves the matched visual replay with
10000 proxies, GPU timestamps, HUDs, 140 frames and frozen captures at 0/139.
`bevy-camera-{build,clippy,tests}.log` records release build, denied-warning Clippy
and ten reference tests passing. Rust formatting, changed Python's Python 3.8
syntax and `provenance-proof.json` GPU/backend/extent/mode/source/executable hashes
pass. Product packages/shaders are unchanged. Native Windows/GPU and other vendors
remain unqualified for this optional reference.

### Explicit image-sampling limit

The faster route exposed near-overlapping proxies with different colours whose
nearest CPU-oracle depths round to the same **canonical forward D32** code.
Bevy actually uses its own reverse-Z projection, so this does not establish its
GPU depth winner. The camera qualifier explicitly excludes these ambiguous rays
before looking at actual RGB. It checks nearby distinct proxy hits, requires
exact equality of their canonical codes, records both identities/depths/code and
marks `gpu_depth_order_verified=false`. It neither accepts a different colour nor
widens the original two-byte RGB tolerance.

The main matrix records **19 such excluded rays**, separately from the 6400
checked pixels and existing narrow nearest-texel boundary exclusions. Each
capture must retain at least fifty colour probes and exclude no more than ten
percent of its coordinates for proxy-depth ambiguity. Actual GPU depth/identity
readback for these rays is still unqualified. The exclusion is opt-in; historical
replay/probe checks retain their original sampling. A failed verifier cannot leave
an older successful pixel-proof file behind.

### Excluded platform failure

An earlier source-preparation attempt lost its Vulkan device. Kernel logs name
the private Weston compositor's SDMA fault; RADV reported Theia's context as
innocent. Device-loss teardown produced cascading validation messages. The failed
session is preserved in `bevy-camera-device-lost/` and is not successful evidence.
A fresh native compute run verified all 256 expected values after recovery;
fresh owned-display source, camera and supplementary runs passed. Root cause and
clean device-loss handling are not qualified by that recovery. This incident
must not be interpreted as an engine or platform stability pass.

## Reproduce

Build the matching reference executable with the README's native development
library setup, then run:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
PYTHONPYCACHEPREFIX="$PWD/build/qualification/python-cache" \
python3 tools/qualify_bevy_camera_stream.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --headless-dri-prime pci-0000_03_00_0 \
  --output build/qualification/bevy-camera-final
```

The qualifier prepares 1320-/180-frame source fixtures, runs the nine plans and
negative control checks, and writes a failed summary before execution. Direct
reference runs accept `--camera-stream`, the budget/slot controls and
`--stream-capture-frames 0,30,75,...` (unique, in range, at most sixteen).
Captures default to start/thirds/endpoint. As with the fine probe, ordinary HUD,
GPU-timing and frozen-snapshot flags are rejected in this diagnostic mode.

## Remaining comparison work

Integrate a separately measured paired streaming profile and stats display.
Control wall-clock demand rate or replay pace: equal logical camera frames at
different uncapped rendering speeds impose different I/O deadlines. Separate
captures/readbacks and lifecycle waits from timing conclusions, record file-cache
policy, and retain backend-specific physical allocation scopes. No cold-cache,
production streaming throughput, failure recovery, representative art or
whole-engine ranking is established here. Theia M4 geometry/texture virtualization
and general LOD work remain outstanding independently of the comparison tool.
