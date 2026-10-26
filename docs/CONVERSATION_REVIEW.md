# Theia Conversation Architecture Review

Reviewed 2026-10-10. Theia is a GPU-first native MMO engine with a server-authoritative world, immutable authored environments, and separate representations for simulation, replication and presentation. The original discussion supports the current architectural direction. The immediate engineering task remains **M0-D Render Lab**, while many detailed formats, algorithms and performance budgets remain open.

This review preserves reasoning and implementation constraints from [the archived conversation](THEIA_FULL_CONVERSATION.md). [DECISIONS.md](DECISIONS.md) remains the primary source for approved decisions; this review does not add or change locked ADRs. [CURRENT_STATUS.md](CURRENT_STATUS.md) separates implementation evidence from plans.

## Review scope and source limits

The review covers all 332 archived user and assistant messages in conversation order, including code examples, user approvals, revisions, diagnostic logs and the final handoff. Repeated responses and excerpts were checked against their containing responses; the archive itself remains unchanged. The two attachment-only messages identify `Pasted text.txt` and `main.rs`, but their attachment contents are unavailable in the archive.

The conversation is an architectural history. An assistant saying “locked,” an interactive control's default selection, an illustrative dashboard, or expected terminal output does not independently establish user approval or runtime success. Historical ADR numbers and milestone names also changed. Resolve current status through the repository register and roadmap.

External product comparisons, library descriptions, release dates, checksums and citations in the transcript are historical claims. This review does not independently validate all those external sources. In particular, claims about WoW: Forever's implementation do not establish Theia's algorithm, performance or feasibility. Recheck official documentation when a specific external capability becomes an implementation dependency.

## Project intent and development priorities

The initial request sought advanced GPU techniques, but the conversation repeatedly refined that ambition around total cost, correctness and manageable scope. “GPU-first” means bulk coherent work should stay resident on the GPU when that improves the whole dependency path. It allows host-side input, I/O, Vulkan orchestration, text shaping, audio and latency-sensitive logic. Moving a small CPU-consumed query to a GPU can add more latency than it saves.

The user explicitly interrupted premature planning for a 10,000-player benchmark because no engine existed yet. The later review favored an early playable vertical slice and concurrent client/server development once interfaces stabilize. Theia should not spend years as an isolated rendering experiment before testing authoritative movement and two-client play. These are development tracks, not an instruction to introduce networking during M0-D. See [the scope correction](THEIA_FULL_CONVERSATION.md#151--user) and [the consolidated review](THEIA_FULL_CONVERSATION.md#236--assistant).

The 10,000-player battlefield, 30 Hz target, 20 Hz stress minimum, 60 FPS objective and proposed 25% transport CPU threshold were planning suggestions. None is a measured result or a finalized acceptance budget in the current register. World dimensions, cell sizes, chunk capacities, ray counts and memory percentages in calculators likewise remain examples.

## Decisions that changed during the conversation

| Topic | Earlier discussion | Current approved direction | Original context |
|---|---|---|---|
| Language stack | Rust, Go and Zig, including proposed Zig utilities and scripts | Rust host/runtime/gameplay, Go services, Slang GPU code; Zig removed | [Language boundaries](THEIA_FULL_CONVERSATION.md#163--assistant), [native gameplay revision](THEIA_FULL_CONVERSATION.md#182--assistant) |
| Gameplay execution | Rust/Zig WASM extensions and Wasmtime | Native Rust plus declarative definitions; WASM deferred | [WASM discussion](THEIA_FULL_CONVERSATION.md#179--assistant), [revision](THEIA_FULL_CONVERSATION.md#182--assistant) |
| Live code updates | Possible dynamically loaded native modules | Definition hot reload and rolling worker deployment behind stable gateways | [Live updates](THEIA_FULL_CONVERSATION.md#186--assistant) |
| Spatial indexing | Global Morton hierarchy, quadtree/octree proposals | Bounded flat XY addressing, sparse vertical detail and CSR membership; geometry hierarchies remain separate | [Flat arrays](THEIA_FULL_CONVERSATION.md#042--assistant), [2.5D model](THEIA_FULL_CONVERSATION.md#046--assistant) |
| Gameplay transport | QUIC, then a proposed custom secure UDP preference | QUIC via Quinn; optimize the application layer first, then consider `quinn-proto` if measured | [Reconsideration](THEIA_FULL_CONVERSATION.md#119--user), [explicit lock](THEIA_FULL_CONVERSATION.md#135--user) |
| Indirect lighting | Baked probes, standalone SSGI/AO and optional future SDF GI | Sparse SDF software dynamic GI with a shared GPU radiance cache; no mandatory hardware RT | [User approval](THEIA_FULL_CONVERSATION.md#085--user), [GI decision](THEIA_FULL_CONVERSATION.md#090--assistant) |
| Character physics | General crowd contact generation and separation | Ordinary characters do not physically block one another; world collision and gameplay queries remain | [User approval](THEIA_FULL_CONVERSATION.md#100--user), [decision](THEIA_FULL_CONVERSATION.md#101--assistant) |
| Environment | Construction, destruction, mutable terrain and door connectivity | Immutable authored physical environment; dynamic gameplay and lighting remain | [User proposal](THEIA_FULL_CONVERSATION.md#209--user), [approval](THEIA_FULL_CONVERSATION.md#214--user) |
| Platforms | Windows primary, Linux first-class | Linux primary, Windows first-class | [User correction](THEIA_FULL_CONVERSATION.md#221--user), [platform revision](THEIA_FULL_CONVERSATION.md#222--assistant) |
| Offline tooling | Custom cookers and an early complete editor | Established tools first, including offline C/C++ tools; custom runtime formats and focused inspectors | [User preference](THEIA_FULL_CONVERSATION.md#194--user), [tooling revision](THEIA_FULL_CONVERSATION.md#196--assistant) |
| GPU storage | General archetypes for nearly everything | General GPU archetypes coexist with specialized dense streams and a read-mostly static scene database | [Consolidated review](THEIA_FULL_CONVERSATION.md#236--assistant) |
| Task stages and shaping | Broad task-stage usage and full GPU shaping | Require mesh/task device capability; use task stages selectively; allow initial host shaping | [Consolidated review](THEIA_FULL_CONVERSATION.md#236--assistant) |

Earlier sections discussing destroyed bridges, player buildings, door-state collision, dynamic terrain journals and structural navmesh rebuilding are superseded by the static-environment decision. Precomputed SDF **geometry** is compatible with dynamic GI: radiance still changes with lights, time of day and weather.

## Context to preserve when implementing subsystems

### World identity and representation ownership

The logical world, runtime ECS, scene instance, streaming cell, GPU scene and view have distinct meanings. Their relationships need not form one containment tree. Unloading a chest's presentation must not delete its persistent identity or inventory. A streaming cell must not automatically determine worker authority or ECS storage.

Relationships such as attachment, ownership, containment and scene membership need separate lifetime policies. A transform parent should not automatically confer persistence ownership or recursive destruction semantics. Large assemblies may use local reference frames independently of authority partitions.

Canonical world coordinates should survive camera movement. The conversation proposes coarse integer cells plus bounded local positions, with camera-relative GPU coordinates and region-relative network encodings. Exact ranges and numerical layouts remain open. Persistent, simulated, replicated, resident and visible are independent states. See [world and scene separation](THEIA_FULL_CONVERSATION.md#011--assistant).

### Schemas and module boundaries

Shared logical semantics should generate specialized CPU ECS types, GPU layouts, network codecs and persistence encodings. One raw struct should not serve every domain. Stable type/field IDs, compatibility rules, defaults, alignment validation and migrations matter before those interfaces become durable.

The proposed schema compiler is a future design direction, not an implemented system or a requirement to build a framework during M0. Similarly, the many suggested crate names are provisional. Extract modules when their boundaries become useful. The proposed dependency rule is that rendering consumes GPU ECS interfaces; GPU ECS should not require the renderer, and persistence should not depend on a transport implementation. Rust/Go services communicate through versioned bulk messages, avoiding calls per entity or tick. See [language and schema contracts](THEIA_FULL_CONVERSATION.md#163--assistant).

### GPU memory and structural mutation

GPU shaders allocate within host-provisioned pools. Rust owns physical Vulkan allocations, resource binding and lifetime. Chunk sizes and component layouts should follow workload measurements rather than a universal byte size.

Structural requests need conflict resolution, destination reservation, initialization, publication and delayed reclamation. A despawn concurrent with an add-component request needs an explicit rule. Handle generations require wrap and reuse policies; generations alone do not make reuse safe while readers remain in flight. Pool exhaustion must produce bounded failure/capacity records rather than silently lose essential entities.

Local compaction and global consolidation have different costs. Both should account for relocation bytes and hysteresis. Stable data need not be compacted routinely, and sparse backing is not required for hot ECS buffers. See [GPU memory architecture](THEIA_FULL_CONVERSATION.md#032--assistant).

### Scheduling and state epochs

The host prepares the legal execution graph and synchronization; the GPU selects active chunks and writes work counts. Systems declare accesses, structural mutation rights and current/previous-state requirements. Workgroup barriers do not substitute for dependencies across an entire dispatch.

Indirect dispatch is the initial scheduling mechanism. Persistent kernels and work graphs remain optional experiments. Software workload priorities are not guarantees of hardware preemption. Visual simulation shares execution and bandwidth with rendering, so background work needs bounded dispatches and budgets.

ECS epochs, server ticks, rendered frames, temporal histories and presentation timestamps are independent. Version only mutable resources that need overlapping readers. Late camera updates require consistent culling/reprojection inputs or conservative bounds. See [scheduling](THEIA_FULL_CONVERSATION.md#024--assistant) and [frame architecture](THEIA_FULL_CONVERSATION.md#066--assistant).

### Spatial indexes and residency

Direct cell lookup does not make a crowded query constant-time. Enumerate resident regions and conservative candidate ranges rather than scanning every world cell. Render, collision, combat, navigation and replication indexes may share coordinates while keeping distinct membership arrays and semantics.

A base CSR index plus deltas must mask stale references and avoid duplicates. Compare incremental maintenance with full rebuilds under high movement. Large multicell objects need ownership and deduplication policies; a coarse index may suit them better than replication into every fine cell.

Residency proceeds through request, staging, upload, publication and retirement. Collision/navigation data and coarse visual fallbacks take precedence over optional detail. Predictive streaming can consider movement, flight, teleports and interiors. Hysteresis and protected fallback pages help prevent eviction/reload churn. Asset I/O pages, Vulkan sparse granularity and ECS chunks are independent sizes. See [world residency](THEIA_FULL_CONVERSATION.md#038--assistant), [spatial refinement](THEIA_FULL_CONVERSATION.md#046--assistant) and [asset cooking](THEIA_FULL_CONVERSATION.md#192--assistant).

### Rendering and temporal reconstruction

The world grid discovers objects; meshlet hierarchies select triangles. Direct ranges, fully compacted lists and hybrid chunk descriptors are alternatives to compare by total visibility-plus-rendering cost. One task group per world cell and mandatory global material sorting are not defaults.

Visibility IDs are frame-local references with a defined bit budget, distinct from world IDs. Material resolve needs valid attribute reconstruction and texture gradients. Optional surface caches and material bins must justify their extra traffic. Transparency, water, hair and effects have separate paths.

Virtual shadow demand from visible receivers still needs independent light-space caster discovery. Camera visibility cannot identify every caster. Previous-frame occlusion requires conservative treatment of newly visible objects and periodic revalidation. Shared scene data supports multiple views, but each view has separate work and bounded demand.

Temporal reconstruction needs current/previous deformation, jitter conventions, exposure, reactive information and invalidation for spawn, teleport, newly resident content and camera cuts. Native-resolution screen UI follows world reconstruction. See [work generation](THEIA_FULL_CONVERSATION.md#054--assistant), [rendering](THEIA_FULL_CONVERSATION.md#060--assistant) and [temporal design](THEIA_FULL_CONVERSATION.md#066--assistant).

### Dynamic GI and character animation

The locked GI target uses streamed static SDF geometry and dynamic radiance updates. It does not require detailed animated crowd geometry in the tracing representation. Selected large movers may have coarse proxies; ordinary characters mainly receive GI. Cache freshness, light leakage, thin surfaces and specular approximation need explicit quality and fallback policies.

Animation graph decisions, pose evaluation and vertex deformation are separate workloads. Pose sharing depends on compatible skeletons, clips, phase, blends and modifiers. Important characters retain unique details. Compare compute skinning reused across views/shadows with mesh-shader deformation of selected clusters. Conservative bounds can permit early culling without complete skinning. Animation LOD preserves action meaning and phase continuity independently of geometry LOD. See [GI rationale](THEIA_FULL_CONVERSATION.md#083--assistant) and [animation](THEIA_FULL_CONVERSATION.md#090--assistant).

### Physics and navigation

Non-blocking characters remove ordinary body-response work, not range, targeting, aggro, line-of-sight or interaction queries. World collision, combat volumes and selection use distinct channels. GI SDF accuracy and update cadence do not qualify it as authoritative collision data.

The discussion proposes heightfield fast paths, local cave/interior collision, swept queries and specialized movement controllers. Cosmetic cloth, particles and ragdolls can use GPU solvers without determining gameplay outcomes. Client prediction may retain a small CPU path where GPU submission and readback would hurt responsiveness.

Navigation topology is separate from the XY proximity index. Recast offline cooking, native Rust hierarchical routing, clearance profiles, route sharing and event-driven AI are proposed directions. Static geometry removes general rebaking; gameplay traversal filters and explicit links can still matter. Flight/swimming and moving transport need separate treatment if included. See [physics](THEIA_FULL_CONVERSATION.md#096--assistant), [non-blocking interactions](THEIA_FULL_CONVERSATION.md#099--assistant) and [navigation](THEIA_FULL_CONVERSATION.md#207--assistant).

### Networking and aggregate crowds

The transport history establishes why Quinn was retained: custom serialization, batching and stale-state suppression do not require a new UDP protocol. The application still needs command IDs, input sequencing, repair and acknowledged baselines. Reliable transport alone does not establish an accepted gameplay outcome.

Canonical encoding includes coordinate region and precision tier; identical world values need not produce identical bytes under different origins. Cohorts share candidates, not permissions. Shared delta caches need bounded baseline/version combinations and full-state repair. Application state acknowledgement is distinct from packet acknowledgement.

Group replication reduces repeated encoding and discovery, while recipient-specific delivery, encryption and bandwidth remain. Push fan-out suits sparse events; cohort-driven snapshots suit dense movement. NetLOD is recipient- and component-specific. A far party member may need immediate health with infrequent position updates.

Aggregate proxies cannot become targets or invent deaths, casts or other authoritative activity. Promotion needs exact baselines before interaction, versioned overlap masks, population accounting, hysteresis and a separate burst budget. Authorization applies to aggregate counts/activity too. See [final transport review](THEIA_FULL_CONVERSATION.md#131--assistant), [group replication](THEIA_FULL_CONVERSATION.md#139--assistant) and [NetLOD](THEIA_FULL_CONVERSATION.md#145--assistant).

### Gameplay updates and durable state

Native Rust evaluates trusted gameplay; declarative content handles routine abilities, quests, loot and configuration. Definitions activate at safe tick boundaries, with policies for in-progress encounters pinned to earlier versions. Avoid arbitrary executable hot swapping in the initial design.

Stable Quinn gateways and stateful worker handoff are separate from QUIC network-path migration. Handoff requires a consistent snapshot, input cutover, timers, definition versions, journal position and fencing enforced where writes are accepted. Container rolling updates alone do not supply those semantics.

PostgreSQL, journals, snapshots and outbox processing remain proposed persistence choices. Valuable trades require a durable, idempotent outcome, including recovery from an ambiguous commit result. Snapshots carry replay sequence boundaries. Runtime chunks are not a durable persistence ABI. Static-world geometry does not remove mutable character, inventory, quest and spawn state. See [live updates](THEIA_FULL_CONVERSATION.md#186--assistant) and [persistence](THEIA_FULL_CONVERSATION.md#200--assistant).

### UI audio platform recovery and diagnostics

Direct Bézier glyph rendering is distinct from multilingual shaping and pixel coverage. The conversation calls for winding, holes, endpoints and antialiasing tests at small sizes and fractional scales. Shared glyph curves and instances avoid rebuilding outlines. UI locality optimizations must preserve compositing order.

The host supports text input, clipboard, accessibility, focus and semantic actions. GPU hit-test feedback needs event sequencing rather than a routine synchronous wait. Audio should use compact host-readable presentation state and run independently of GPU frame cadence. Kira/CPAL remain candidates.

Authoritative replicas, resource manifests and essential local state should permit reconstruction after device loss; disposable GPU cosmetics need not survive. Seamless recovery is not promised. HDR output, UI reference luminance, VSync/VRR and minimum VRAM remain qualification work.

Diagnostics should inspect GPU invariants deliberately without creating permanent readback stalls. Capture seeds, versions and authoritative input/event sequences for reproducible failures. Measure critical paths and tail latency, including surrounding data movement, not only isolated kernels. See [UI](THEIA_FULL_CONVERSATION.md#007--assistant), [platforms](THEIA_FULL_CONVERSATION.md#219--assistant) and [tooling](THEIA_FULL_CONVERSATION.md#227--assistant).

## Documentation gaps and qualifications

The current architecture and subsystem documents capture the final direction well. The following details are compressed or absent and should be carried into later subsystem specifications when relevant:

| Detail | Practical consequence |
|---|---|
| Relationship lifetime policies and canonical coordinate frames | Prevent accidental deletion, identity changes or artificial motion during unloading/rebasing |
| Schema compatibility and domain-specific generated representations | Avoid coupling GPU ABI, network wire format and durable storage |
| Generation wrap, mutation conflicts and capacity failure | Make allocation and structural mutation correct under stress |
| Late camera coherence and temporal invalidation | Avoid missing geometry and false motion histories |
| UI event sequencing and device-loss reconstruction | Preserve usable host behavior with a GPU-primary presentation runtime |
| Navigation agent profiles, scheduled AI and bounded path arenas | Keep CPU authoritative queries predictable without introducing per-NPC object graphs |
| Encounter version pinning and ambiguous durable commits | Preserve gameplay continuity and transaction correctness during updates/failures |
| Aggregate promotion budgets and authorization of group summaries | Avoid transition spikes, duplicate visuals and information leakage |

GPU UI, animation and schema generation have substantial subsystem context but are not separate locked rows in the current decision register. Preserve their documented intent without silently promoting every suggested technique into a new locked decision. Exact tools, formats and numeric budgets remain open.

`CURRENT_STATUS.md` is a historical handoff dated 2026-10-09. The inspected checkout now has the initial compute commit, the architecture handoff commit and an `origin` remote configured. Remote configuration alone does not prove current remote contents or push status. The current source creates a debug messenger but supplies no explicit validation layer names at instance creation; environment-based activation and callback delivery still need independent evidence.

The archive contains citation tokens, ChatGPT UI markup, unavailable attachment links and some incomplete response fragments. Preserve them as history. The final chat's suggestion to commit an extracted transcript does not override repository privacy instructions; this repository archive comes from the explicitly supplied public share, not an account-wide export.

## Implementation evidence and next task

| Capability | Evidence available | Qualification |
|---|---|---|
| RADV device detection | [User device report](THEIA_FULL_CONVERSATION.md#253--user) identifies RX 7900 XTX and integrated Radeon | Historical output, not a new hardware probe |
| Mesh/task support | [User feature output](THEIA_FULL_CONVERSATION.md#255--user) reports both feature bits and limits in two unlabeled device blocks | Advertisement is not pipeline execution; query the selected device in Render Lab |
| Slang compute SPIR-V | [Compilation and disassembly](THEIA_FULL_CONVERSATION.md#264--user) show SPIR-V 1.6, 64 threads and set 0/binding 0 | Does not qualify task/mesh codegen |
| Headless compute | [User run](THEIA_FULL_CONVERSATION.md#305--user) reports 256/256 expected values; corresponding Rust/Slang source exists | Preserve as M0-C regression; not rerun for this review |
| Formatting and linting | The same user log reports completion of formatting/Clippy commands | Historical result |
| Debug messenger | Source creates/destroys it; user log prints its initialization | Does not independently prove layer activation or callback delivery |
| Render Lab | No `apps/theia-render-lab` or task/mesh/fragment source exists in the inspected checkout | M0-D outstanding |
| Advanced engine systems | Architecture documents describe their intended behavior | No ECS, GI, streaming, UI, navigation or networking implementation in the inspected tracked files |

Follow [the current M0-D roadmap](ROADMAP.md), retaining the headless app. The first useful increment is a separate native window with a Vulkan surface/swapchain, selected-device capability reporting, explicit validation diagnostics, safe resource ownership and resize/minimize handling. Then qualify mesh-only rendering, task amplification and compute-written indirect mesh/task work with SPIR-V validation and target-GPU logs.

The full frame graph, schema compiler, ECS allocator, GI, audio, network replication and distributed services belong to later slices. M0-D should preserve room for their contracts without implementing them speculatively.

## Conversation navigation

Message numbers refer to the immutable archive headers. Some topics span repeated excerpts and commentary.

| Messages | Topic |
|---|---|
| 001–007 | Initial GPU renderer and direct Bézier UI |
| 008–018 | World/scene/view split, ECS identity and contiguous data |
| 019–032 | GPU scheduling, structural mutation and memory |
| 033–049 | Residency, flat arrays, sparse vertical detail and render integration |
| 050–060 | Render work generation, visibility buffers and lighting |
| 061–066 | Frame clocks, state epochs and temporal reconstruction |
| 067–090 | GI alternatives, product comparisons, SDF decision and animation |
| 091–107 | Physics, non-blocking characters and initial server architecture |
| 108–136 | QUIC concerns, custom UDP proposals and final Quinn choice |
| 137–145 | Group replication and aggregate NetLOD |
| 146–156 | Extreme-density proposals and correction back to pre-implementation scope |
| 157–186 | Language boundaries, scripting revisions and live updates |
| 187–200 | Asset cooking, offline tooling and persistence |
| 201–215 | Navigation and the immutable-environment revision |
| 216–227 | Hardware/platform policy, Linux priority and operations |
| 228–242 | Consolidated architecture review, staged roadmap and M0 foundation |
| 243–266 | Target GPU, RADV, Slang installation and compute shader validation |
| 267–287 | Theia naming, Rust bootstrap and headless compute source |
| 288–306 | Compute execution logs, debug messenger and M0-D proposal |
| 307–318 | Git workflow and nested repository correction |
| 319–332 | Architecture handoff, context preservation and export instructions |
