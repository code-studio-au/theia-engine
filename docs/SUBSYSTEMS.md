# Theia Subsystem Specifications — GPU, World, Rendering, UI and Simulation

## 1. GPU ECS and memory

**Host:** Rust/Vulkan owns allocations and queue submissions. **GPU:** Slang compute owns bulk presentation component processing, active-set selection and in-arena suballocation. Dense chunked SoA/AoSoA archetypes for composable entities; separate fixed-layout streams for meshlets, foliage, particles, work queues and GI probes. Stable world IDs map to local generational handles, which map to archetype/chunk/row. Chunk descriptors specify column offsets/strides. Physical placement is never identity.

Arena domains: component chunks, entity metadata, spatial indexes, static geometry, dynamic work/scratch, UI/curve data, animation pose buffers, texture/geometry/GI residency. Use Vulkan buffer device addresses where useful but never expose raw device pointers as durable identities. Shader allocation draws from host-provisioned pools; expansion handled by host asynchronously.

GPU-generated structural commands: spawn/despawn/add/remove component, validate conflicts, sort/group destination archetypes, reserve slots, copy component columns, initialize, publish new location mappings, retire old storage when readers complete. Maintain overflow/error records for exhausted pools. Avoid one global contended allocator. Structural changes at explicit epoch boundaries. Mutations, free-list and publication tested against simple CPU reference.

**Scheduling:** compiler-prepared DAG based on system read/write declarations; GPU generates work lists, chunk ranges and indirect dispatch counts. Vulkan barriers between actual hazards. Minimize global barriers, avoid per-entity synchronization. Work graphs, device-generated commands and persistent kernels are optional accelerators, not foundational dependencies. Double/triple buffer only selected mutable data when overlapping GPU readers require it.

## 2. World, spatial and scene split

Persistent logical world; authoritative server worker ECS; client replica; resident GPU presentation ECS; independent view visibility; authored static scene database. Fixed world dimensions and horizontal grid enable `x + y * width` direct XY cell lookup. Fine grids resident only where streamed. Per-cell packed vertical intervals for city floors, caves and flight; local 3D structures when truly volumetric. CSR offsets/counts and contiguous entity-reference lists; optionally maintain a base index + per-cell delta overlay and periodically compact. Compare with full rebuild under high mutation. Morton coding is useful for sorting spatial workloads and constructing some acceleration structures, **not the global entity ID or required top-level cell address**.

Static world content not required to occupy mutable gameplay archetypes unless interactive. Collision, render visibility, replication interest, GI occupancy and navigation have independent specialized spatial membership/indexes based on shared coordinates. Distinct partitioning: content cells, authority workers, replication cohorts, render views, geometry LOD pages.

## 3. GPU-driven rendering

Resident region/cell enumeration → coarse frustum/vertical/HZB culling → candidate entity/chunk ranges → adaptive GPU work descriptors → task shader optional amplification + mesh shader cluster decode → depth/visibility buffer. Test direct ranges, compact instance lists and hybrid chunk descriptors; don't assume global sorting is beneficial. Meshlet/cluster hierarchy separate from XY grid. Hierarchical compressed cluster LOD, projected-error selection, conservative bounds, residency validation, protected fallback page. Offline cook crack/seam correctness. Hybrid compute microtriangle rasterization is a **later experiment**, not a first-milestone requirement.

Opaque shading baseline: compact frame-local instance/cluster/primitive reference + depth → resolve vertex attributes/barycentrics/material via bindless resources → optional resolved-surface cache → clustered direct lighting, shadows, indirect radiance. Derivative reconstruction is a known risk; bin material families only if benefits outweigh scans/sorting. Transparent glass/water/particles and hair require separate paths. No hardware ray tracing requirement.

Virtual textures: tiled mip/page tables, GPU demand feedback, host I/O, transfer and publication; fallback mips. Small textures may be conventional. Virtual shadow maps: demand-driven page atlas, light-space caster discovery (camera-visible objects alone are insufficient), cache invalidation of dynamic casters. Shadow paths independent of SDF GI.

## 4. Dynamic GI — explicit locked target

GPU **software** tracing through streamable sparse SDF bricks tied to static authored environment; shared radiance/irradiance cache updated dynamically for daylight, artificial/emissive lights, etc. Static terrain/buildings/caves cooked offline, no routine runtime static SDF rebuild. Significant movers may use coarse dynamic SDF proxies. Ordinary animated characters primarily receive GI; do not update detailed SDF for thousands of skinned meshes per frame. Cache update budgeting, invalidation, leak mitigation, temporal reuse and reflection limitations require benchmarking. Screen-space data can accelerate/fill the system. Probe/radiance-cache data have separate residency and history lifetimes from geometry. Full RT, ReSTIR and neural rendering deferred. Dynamic GI still needs a cheap fallback mode and distinct direct shadows.

## 5. Frame/temporal architecture

Separate authoritative tick, network update, client predicted state, GPU scene epoch, render frame, temporal-history index, streaming job and presentation timestamp. Epoch publication gives consistent component location/transform/pose references; not a full ECS copy each frame. Timeline semaphores and Sync2 used for queue and resource dependencies. Initial target two frames in flight, configurable. Motion vectors require previous camera, model, previous deformed/skinned positions (or reconstructable previous pose), correct rebasing; handle spawn, teleport, discontinuities. Temporal reconstruction/back-end neutral inputs (scene color, depth, motion, exposure, jitter, reactive masks), dynamic resolution with adaptive GI/geometry/shadow/animation budgets. Native-resolution GPU vector UI composited with explicit HDR color policy.

## 6. Animation and massive crowds

GPU animation graph/state selection + compressed clip sampling + hierarchical pose evaluation + shared pose cache keyed by compatible skeleton/clip/quantized time/blend state. Independent animation LOD (bones, layers, update rates) from geometry LOD. Adaptive compute skinning (multi-view/shadow reuse) versus mesh-shader skinning (selected visible clusters); compare total bandwidth/work. Shared poses do not override unique player/hero animation correctness. Attachments, previous poses and animation bounds versioned. Cosmetic physics (cloth, particles, ragdolls) primarily GPU; server controls gameplay outcomes.

## 7. Physics and interactions

Ordinary players/NPCs/monsters **do not body-block each other**. Terrain/static buildings/world collision remains. Combat hit volumes, proximity interaction and gameplay targeting are distinct channels; optional boss/vehicle blocking. Static collision/terrain heightfields and local cave/interior meshes cooked offline, not inferred from low-resolution GI SDF. CPU/server collision for latency-sensitive authoritative movement; GPU visual physics (XPBD for suitable deformables, specialized rigid-body solvers) for cosmetics, and optional batched GPU physics where measured. Fixed simulation timestep; gameplay never uses visual ragdoll state as authority.

## 8. GPU Bézier UI

GPU-resident retained-mode visual UI, layout/animation/hit-testing primarily GPU. Glyph source outlines (TrueType quadratics, CFF cubics) stored as curve records with spatial acceleration; mesh shader emits covering primitives, fragment/compute shader evaluates coverage. No requirement for bitmap/SDF glyph atlases. Font shaping, bidi, input method, accessibility semantic representation and clipboard can be host-driven initially. Preserve compositing order independently of Morton spatial sorting; specialized analytic rectangles/image sampling; clipping/masks need well-defined ordering. UI at display/native resolution and accessible input/semantic bridge.

## 9. Immutable world simplification

No player construction, dynamic terrain deformation, generalized structure destruction or navigation-changing doors. All environment meshes, heightfield collision, Recast navmesh, SDF geometric bricks and static proxy hierarchies can be prepared offline. Game world still has mutable entities/items/quests/spawn state and dynamic weather/water/day-night/radiance. No runtime general navmesh rebake or environment-geometry persistence required.
