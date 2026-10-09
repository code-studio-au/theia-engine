# Theia — Open Questions, Risks and Qualification Strategy

This is an experimental GPU-first engine. **Consensus on an architecture is not evidence of achievable performance.**

| Risk | Why it matters | Validation / fallback |
|---|---|---|
| Slang task/mesh support | Mesh shader codegen and driver behavior may vary | First real task+mesh+fragment test on RADV; alternate HLSL/GLSL specific modules if needed |
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
