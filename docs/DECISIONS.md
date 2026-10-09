# Theia Engine — Architecture Decision Register

**Meaning:** LOCKED = explicitly agreed target; it is not implemented or validated. PROPOSED = architecture suggested, not explicitly locked. BENCHMARK = must be empirically assessed. SUPERSEDED = do not follow earlier recommendation.

| ID | Status | Decision | Key qualification |
|---|---|---|---|
| ADR-001 | LOCKED | Native custom MMO engine; no UE/Unity/custom C++ | Permits third-party offline tools in C++ |
| ADR-002 | LOCKED | Rust engine/client + authoritative ECS, Go distributed services, Slang GPU language preference | Slang feature/compiler support is a validation gate |
| ADR-003 | LOCKED | Linux primary, Windows first-class | Wayland primary; X11 support planned |
| ADR-004 | LOCKED TARGET | Vulkan 1.3 baseline, mesh + task capable GPUs | Mesh-only shaders permitted when beneficial; per-device limits queried |
| ADR-005 | LOCKED TARGET | GPU-first execution; CPU authority/state/I/O | Not GPU-exclusive; performance, correctness and latency govern placement |
| ADR-006 | LOCKED TARGET | GPU ECS with contiguous SoA/AoSoA, chunked archetypes, generational handles, epochs, structural mutation queues | Specialized GPU streams also first-class; GPU memory allocation in host-backed arenas |
| ADR-007 | LOCKED | Bounded, terrain-focused flat 2.5D spatial indexing, direct XY grid + sparse vertical layers + CSR memberships | Special 3D local indexes for caves, interiors, airspace; Morton selectively for locality |
| ADR-008 | LOCKED TARGET | GPU-driven render work lists, compute culling, task/mesh shader geometry, virtualized meshlets | No full-scene per-frame CPU sorting/draw submission |
| ADR-009 | LOCKED TARGET | Visibility-buffer-first opaque pipeline and deferred material resolve | Transparent/water/particles use separate paths; benchmark vis buffer vs G-buffer |
| ADR-010 | LOCKED | Sparse SDF **software** dynamic GI with shared GPU radiance cache | Static SDF geometry cooked offline; hardware RT not mandatory; specular quality limitations |
| ADR-011 | LOCKED | Standard characters non-blocking with one another | World collision, hit volumes and interaction queries remain; boss/vehicle exceptions |
| ADR-012 | LOCKED | QUIC protocol via Rust Quinn | DATAGRAM for supersedable snapshots; streams for reliable operations; `quinn-proto` optimization only if needed |
| ADR-013 | AGREED | Group replication, shared canonical encoding, spatial/semantic cohorts, per-client finalization | Personal authorization, baselines, congestion and packet encryption remain individual |
| ADR-014 | AGREED | Network LOD + distant aggregate crowd proxies | Do not fabricate authoritative individual gameplay events; promote important individuals |
| ADR-015 | LOCKED | Static authored terrain/buildings/caves; no construction, destruction or navigation-changing doors | Dynamic characters, weather, lighting, gameplay objects remain |
| ADR-016 | LOCKED | Native Rust + declarative gameplay definitions; WASM and Zig removed/deferred | Native code updates via rolling worker deployment; declarative hot reload at tick boundary |
| ADR-017 | LOCKED | Live-update strategy: hot-reload definitions, rolling Rust worker deployments, stable Quinn gateways | No arbitrary native DLL hot swapping; zero downtime is an objective, not guarantee |
| ADR-018 | LOCKED POLICY | Use mature offline compilers, geometry/navigation cookers, authoring apps | C++ allowed offline, not custom shipped runtime/game code |
| ADR-019 | PROPOSED | Content-addressed, versioned immutable cooked assets with Rust packager | Meshlet LOD and SDF cooker correctness remain high-risk |
| ADR-020 | PROPOSED | PostgreSQL transactions + durable journals and snapshots | Exact DB/log design and recovery policy not finalized |
| ADR-021 | PROPOSED | Recast offline navigation + native Rust tiled hierarchical pathfinding | Specialized flight and cave topology; CPU-first authoritative navigation |
| ADR-022 | PROPOSED | Linux Vulkan client via ash/winit/ash-window | Actual builds/driver compatibility validated per platform |
| ADR-023 | PROPOSED | Debug layers, reference tests, client/server telemetry, rolling deploy path | Tooling staged with milestones |
| ADR-024 | LOCKED | Engine name: Theia; `theia-engine` repo naming | Trademark/branding clearance not performed |
| ADR-025 | VERIFIED ENVIRONMENT | RX 7900 XTX on Mesa RADV, Slang compute + Vulkan execution | See CURRENT_STATUS; no mesh/task execution test yet |

## Superseded choices / important reversals

- Initial server/engine/client/scripting split including Zig → **Zig removed**. Native Rust code + data-driven rules, no WASM requirement.
- Initial Rust/Zig WASM scripting idea → **native Rust first**, declarative content hot reload, optional WASM only if isolation/mods later justify it.
- QUIC vs specialized custom UDP debated → **QUIC via Quinn** selected; do not write RUDP absent measured severe limitation.
- Mutable terrain, player construction, destroyed buildings, geometry-driven runtime navmesh/SDF rebuilding → **immutable authored world**.
- Full GPU multilingual text shaping → **host shaping is acceptable initially**, direct Bézier-outline GPU rendering remains desired.
- Full native editor from day one → **existing Blender/authoring tools and asset scripts first**.
- Mandatory task stage for every render path → require feature capability but use task stage only when worthwhile.
- All data in one universal ECS format → **specialized dense streams coexist with general archetypes**.

## Approval process

Before changing a locked decision, describe performance/correctness impact, alternatives, the proposed new ADR and explicit user approval. Record concrete benchmarks separately from design assertions. Maintain short per-ADR files if decisions grow beyond this register.
