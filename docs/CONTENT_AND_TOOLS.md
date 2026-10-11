# Theia Content Pipeline and Offline Tool Policy

## Core policy (LOCKED)

Use **established offline tools first**, regardless of C/C++ internals. No need to rewrite successful algorithms. Prefer CLI invocation or isolated tool processes; FFI limited to bulk offline work. Custom runtime Rust/Slang asset formats, residency tables and decoders where Theia's design genuinely requires them. Pin tool versions and licenses, content hashes and build settings; isolate untrusted asset processing.

## Initial candidate tools (evaluate licenses and versions before pinning)

- Blender for modelling, rigging, animation and scene authoring, headless automation/export.
- glTF 2.0 interchange first; USD evaluation for larger scene pipelines.
- meshoptimizer for offline meshlet generation, simplification, cluster and topology operations (C++ offline acceptable).
- KTX-Software/Basis Universal for GPU texture import/transcode, runtime format targeting.
- Recast for offline navigation-mesh generation; tiled navmesh and region portal metadata.
- Animation Compression Library as candidate offline animation codec; GPU decoder suitability must be measured.
- FontTools/OpenType for font outline preprocessing, character maps and curve metadata.
- Slang compiler and SPIRV-Tools as offline build tools.

## Cooking graph

Source authoring assets → import/coordinate validation → normalized intermediate asset → static geometry clustering and hierarchy LOD → compress/paginate → texture mip/tile encoding → cook collision, tiled navmesh, static SDF → compress skeleton/clip assets → generate logical asset ID and content hashes → manifest/package with immutable versioned dependencies. Separate authoring IDs and precise hashes of cooked contents. Hash must include compiler version/config/target features.

## Runtime residency

Stream independent geometry, texture, collision, navigation, SDF, animation and UI resources. Collision/navigation critical ahead of cosmetic detail; protect fallback LOD pages; external Rust I/O, transfers and Vulkan allocation; GPU page-demand queues and safe publication/retirement. Asset page size, Vulkan sparse allocation size and ECS processing chunk size are independent tuning parameters. Ordinary small textures can remain resident. Static scene GPU resource DB is distinct from mutable ECS.

## Virtualized geometry challenges

meshoptimizer primitives do not deliver a complete Nanite-class virtualized renderer. Theia must integrate hierarchical simplification/error, crack/seam correctness, cluster-page dependencies, streaming fallback and mesh-shader decode; GPU work selection and HZB across epochs. Benchmark rather than assume zero gaps.

The bounded heightfield prototype now calls official meshoptimizer 1.3
`meshopt_simplify` with `LockBorder|ErrorAbsolute`, then existing meshlet cooking.
Native configuration/error is recorded with source/tool/library hashes. Independent
checks preserve exact source boundary edges and measure sampled vertical error;
this is not a general hierarchy, geometric error bound or authoring importer.
See [meshoptimizer's simplification contract](https://github.com/zeux/meshoptimizer/blob/v1.3/README.md#simplification)
and [local qualification evidence](validation/M4_COOKED_MESHLETS.md).

## No full editor first

Pinned offline KTX-Software 4.4.2 now qualifies bounded synthetic opaque RGBA8
sRGB single-mip textures. The source build excludes the separately licensed ETC
decoder; no native tool enters the runtime. The pure Rust KTX2 reader borrows
verified dense texels. A resident Vulkan upload/nearest-repeat mip-zero path now
qualifies perspective sampling with unlit factors. Opt-in glTF shared external
PNG import now feeds that fixed bundle, with bounded source/sampler checks and
offline decode. Multiple per-material texture dependencies, mip/gradient and
residency integration remain open.
[Profile/evidence](validation/M4_TEXTURES.md).

`tools/theia-gltf-import` now evaluates pinned Rust `gltf` 1.4.1 offline for a
bounded static position-only subset. It bakes node transforms/reflections into
Z-up geometry and supplies the existing meshlet cooker, preserving tool/source
provenance. Optional `--unlit-factors` now imports bounded opaque double-sided
KHR_materials_unlit factors with material assignments preserved through native
triangle reordering. Optional `--unlit-uv` adds authored float2 TEXCOORD_0 with
exact-geometry binding and qualified perspective reconstruction. Other material/
UV/normal/animation/extension/camera inputs fail;
this is not a general glTF scene importer or GPU instancing implementation.
[Workflow/limits](../tools/theia-gltf-import/README.md).

Begin with Blender/CLI exporters and focused visualization tools (asset viewer, world cell inspector, residency debugging, ECS inspection). Dedicated native Rust world editor only when clear authoring requirements exceed existing tools. Use immutable base world + authored overrides + mutable gameplay-state layer; but **environment physical geometry is static in gameplay**.

Multiple authored external PNGs now normalize with exact encoded-image alias
deduplication and cook into a material-bound table plus THEIABND v2. Complete
root/leaf admission and fixed resident GPU sampling pass local qualification.
Next: bounded whole-resource-set CPU staging/publication and retained-reader
accounting before extending asynchronous GPU publication. Mips, gradients,
filtering and virtual texture streaming remain open.

The complete-bundle CPU store now qualifies atomic full-set reservation and
publication with retained-reader ownership and unique encoded-byte accounting.
It includes manifests, shares exact children across v1/v2 sets, and excludes
I/O/decoded temporary memory and physical GPU allocation. Next qualify the
separate asynchronous GPU publication/retirement path.

The bounded two-v2-bundle viewer now performs asynchronous complete I/O and
uploads, then frame-safe complete generation publication. CPU child sharing,
active/pending physical buffer/image budgets, image-only budgets and count
changes qualify locally. Publication currently retires/rebuilds a graphics
generation with device idle; general world streaming and allocation reuse remain
open. Next qualify a bounded LOD error/selection contract alongside existing
immutable bundle ownership, keeping advanced GI deferred.
