# Theia Engine — Architecture v0.1

**Status:** architecture agreed in conversations; experimental project. **No complete engine exists yet.** Last updated 2026-10-09.

## Product and boundaries

Theia is a native, Linux-first, Windows-first-class, WoW-like MMO engine for primarily outdoor terrain plus cities, caves, interiors and flight. Users explicitly selected **static authored environment geometry**: no player-made structures, deformable terrain, generalized building destruction, or ordinary navigation-changing doors. Characters, monsters, inventory, combat, spawns, weather, lighting, water and visual effects remain dynamic. Characters do not physically block one another by default, though they collide with the world and still participate in combat/interaction queries. Special boss/vehicle exceptions are possible.

No Unreal, Unity, or custom C++ engine/server/client source. Offline tools written in C/C++ are fine. Primary source languages: **Rust for host/runtime/server/gameplay, Go for services and operations, Slang for GPU code**. Zig was considered then removed. WebAssembly was considered for scripts then explicitly deferred. Native Rust + declarative gameplay data preferred.

## System of systems

```
Immutable cooked content ──> Content hashes/manifests ───────┐
PostgreSQL transaction store ─> Rust authoritative ECS        │
Go accounts/social/ops ─────────> Rust simulation workers      │
                               ↘ authoritative change streams  │
                       interest cohorts + shared encoding      │
                               ↘ QUIC via Quinn                 │
                           Rust native client host <───────────┘
                   [input, QUIC, decode, asset I/O, Vulkan]
                               ↓ batched transfers
                  GPU resident presentation/visual ECS
        ┌──────────────────────┼─────────────────────────┐
        │ 2.5D spatial grid     │ static scene database    │
        │ GPU animation/physics│ GPU work scheduling      │
        └───────────┬──────────┴─────────────────────────┘
           GPU culling / meshlets / task+mesh shaders
                 visibility + depth buffers
        material resolve / clustered lighting / shadows
          sparse static SDF + shared dynamic radiance cache
            temporal reconstruction / Bézier vector UI
                             display
```

## Authority vs representations

- **World ID:** persistent logical identity, independent of ECS storage positions.
- **Server ECS:** authority for NPC decisions, gameplay, movement legality, combat and components; no client trust.
- **Client/network replica:** interest-filtered state; not entire server world.
- **GPU presentation ECS:** versioned dense buffers, GPU-managed visual-only entities; no authoritative item/combat decisions.
- **Static authored scene:** GPU-addressable immutable read-mostly content; do not force all static props into mutable ECS archetypes.
- **Spatial membership:** independent index, not entity identity, archetype identity, or persistence ownership.
- **View:** independent visibility/LOD selection over shared resident GPU scene.

## Non-negotiable cross-cutting invariants

1. Persistent world identity != local generational GPU handle != physical (archetype, chunk, row) address.
2. No world-scale octree is required for normal spatial queries; bounded world uses flat XY direct addressing with sparse vertical detail; navigation and meshlet hierarchies remain distinct.
3. Entity movement through cells must not automatically migrate archetypes.
4. GPUs allocate within preallocated host-backed arenas, but Vulkan host creates/manages physical allocations, sparse binds and external I/O.
5. Structural changes publish at consistent epochs; no reclaimed address can be read by in-flight GPU work.
6. A queued journal write is not a durable commit; inventory/currency/trades require genuine transactional outcomes.
7. Static environment SDF geometry can be preprocessed offline; **dynamic radiance** still updates on GPU. GI SDF is not authoritative collision geometry.
8. Geometry LOD, animation LOD, network LOD, streaming residency and server simulation frequency are separate concepts.
9. Shader source language/descriptor backend are replaceable behind a binary interface; Vulkan consumes validated SPIR-V.
10. Profiling should measure entire critical paths and CPU↔GPU handoff costs, not isolated shader durations.

## Design style

- GPU-first compute and raster pipelines with indirect work generation.
- No CPU-per-object draw list or per-glyph rasterization.
- Compiler-constructed dependency graph; GPU generates active work sizes, work queues and indirect commands. Use resource-scoped barriers; no routine per-pass CPU waits.
- Runtime representations specialized by domain (CPU archetype ECS, GPU visual ECS, spatial indexes, network bitpacking, durable storage); use generated schemas, not identical raw structs everywhere.
- Default to mature offline tools, custom runtime only where it differentiates Theia.

## Deferred, not forgotten

Large-scale 10k-player combat architecture, optional hardware RT, ReSTIR/neural techniques, async-compute tuning, cross-vendor minimum GPU/VRAM, complete Unicode GPU text shaping, fully dynamic GPU archetype mutation performance and automatic realm worker handoff: all need staged measurement/implementation.
