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

## No full editor first

Begin with Blender/CLI exporters and focused visualization tools (asset viewer, world cell inspector, residency debugging, ECS inspection). Dedicated native Rust world editor only when clear authoring requirements exceed existing tools. Use immutable base world + authored overrides + mutable gameplay-state layer; but **environment physical geometry is static in gameplay**.
