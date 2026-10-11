# Theia — Open Questions, Risks and Qualification Strategy

This is an experimental GPU-first engine. **Consensus on an architecture is not evidence of achievable performance.**

| Risk | Why it matters | Validation / fallback |
|---|---|---|
| Slang task/mesh support | Mesh shader codegen and driver behavior may vary | Minimal task+mesh+fragment executed on RADV (see CURRENT_STATUS); broader features/vendors remain gates. Slang 2026.19 task output-parameter crash worked around with groupshared; alternate HLSL/GLSL modules only for specific failed features |
| GPU archetype structural mutation | allocator races, fragmentation, stale handles, compaction barriers | CPU reference oracle, randomized mutation fuzzing and safety checks; constrain dynamic composition / specialized stores if needed |
| GPU ECS epochs | in-flight references after relocation/eviction | per-resource lifetime tracking, multi-frame hazard tests; version selected mutable streams only |
| GPU workload DAG | performance lost to many barriers/dispatches | measure total critical path and compare direct/task/indirect variants |
| Virtualized meshlet cluster LOD | cracks, temporal popping, page dependencies | offline geometry correctness and edge tests; discrete meshlet LOD fallback |
| Visibility buffer | material derivative and random attribute fetch overhead | benchmark compact G-buffer and vis-buffer in representative scenes |
| SDF GI shared radiance cache | leakage/thin geometry/reflections, GPU cost | sample caves, interiors, day-night, spells, crowds; cheap GI fallback; no mandatory RT |
| GPU UI Bézier curves | antialiasing of small text, cubic math, RTL/shaping/IME | host shaping first, visual GPU curves; accessibility semantic bridge |
| Large-world spatial index | dense headers, high occupancy, dynamic maintenance | flat XY + sparse vertical plus CSR/deltas; benchmark rebuild vs incremental |
| Server authority distribution | cross-partition conflicts under dense combat | avoid premature live repartitioning; single-writer state ownership, fencing/epochs; targeted workload tests later |
| Group replication | baseline fragmentation and N×M fanout | per-client ACK + shared canonical values, bounded caches, hybrid push/pull |
| QUIC CPU/congestion | connection/crypto state might be expensive | optimized Quinn first, profile packets/sec/core and p99; quinn-proto customized runtime before RUDP |
| Windows first-class | Vulkan/window/cross-toolchain divergence | CI Windows build early; per-vendor execution later |
| 24 GiB dev GPU bias | engine might only run on high end | bounded resource budgets and lower VRAM tier later |
| GPU text/UI semantics | I/O/IME/accessibility cannot be GPU-only | host semantic/input bridge; avoid GPU readback critical path |

## Decisions needing concrete values later

- Actual minimum GPU/VRAM models; baseline fps and target resolution; vendor matrix.
- Actual chunk classes, pool sizes, meshlet vertex/primitive counts.
- Render work descriptor representation, frame-local visibility ID bit budget.
- Current and previous pose memory vs recomputation; animation LOD thresholds.
- Network tick cadence, interest radius, per-client bandwidth cap and loss/latency percentile goals.
- PostgreSQL schema, journal storage choice and exact durability SLOs.
- GI probe/brick spacing, update rates, representation of moving occluders.
- HDR/UI luminance conventions, text shaping implementation, audio backend.

## Correctness principles

Use Vulkan validation and GPU-assisted validation in diagnostic builds (not perf baselines), diff GPU kernels vs CPU reference, test worst-case allocation pressure and stale handles, and maintain benchmark artifacts labelled with GPU driver/compiler versions. No phantom successes; only mark tests passed from observed logs.

## Early authority/network reference evidence (2026-10-11)

Bounded Rust movement/combat, independent request sequences, encrypted two-client
Quinn IPv4 loopback, controlled post-handshake packet faults and readonly snapshot/
CPU interpolation oracles now have local qualification logs. No WAN/load, accounts,
persistence, general baseline repair or client prediction is qualified. Separate
loopback GPU viewers now consume verified readonly authority snapshots; bounded
arrival-relative smoothing is not network clock synchronization. Actual authority
full/delta baseline ACK/repair now qualifies on the live GPU path; this remains a
fixed two-actor profile with no general lifecycle/persistence or WAN/load evidence. Prototype units/tick/gap profiles are test settings, not locked network
precision or cadence choices. IPv6 is disabled on the local host; Windows/hosted
CI remain unexecuted. See CURRENT_STATUS and M7/M8 validation reports.

## Nested Wayland platform evidence (2026-10-11)

Actual Wayland Vulkan surfaces execute within an owned Weston-on-X11 compositor;
six standalone modes and a faulted encrypted full/delta/repair pair pass Vulkan
validation. Winit's unsupported programmatic unminimize is explicitly excluded
from automated smoke. Available RenderDoc 1.46 rejects the Wayland surface extension;
that capture attempt fails and is not counted as validation/replay success. Native
seat/user minimize-restore, Windows and other vendors remain gates. No timing/
performance claim. [Evidence](validation/WAYLAND_NESTED.md).
