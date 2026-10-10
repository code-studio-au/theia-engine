# Bounded static glTF normalizer — M4 prototype

Offline Rust executable using pinned `gltf` 1.4.1 (MIT OR Apache-2.0; license files
verified in the downloaded crate). No glTF/image/native importer enters runtime
packages. [Parser documentation](https://docs.rs/gltf/1.4.1/gltf/),
[Khronos glTF coordinates/transforms](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units).

```bash
cargo run --locked --manifest-path tools/theia-gltf-import/Cargo.toml -- fixtures/gltf/transformed_scene.gltf build/assets/imported-scene.json
python3 tools/cook_meshlet_fixture.py --geometry build/assets/imported-scene.json --library <meshoptimizer-library> --source-archive <qualified-archive>
```

Read `build/assets/imported-report.json` for the resulting hash-named mesh path,
then use Render Lab `--asset <path> --camera-test`. Fixture is synthetic authored
geometry: two nodes share one mesh, one reflected/rotated/raised. Normalizer bakes
parent/local transforms, converts glTF RH Y-up meters to RH Z-up `(x,-z,y)`, and
swaps indices 1/2 for negative world determinant to preserve the intended front
face. Instances get separate dense geometry ranges; this is not GPU instancing.
Node indices/names are provenance, not persistent world IDs.

Default mode supports: selected default scene (or sole scene), position-only unmaterialed
triangles, embedded base64 buffers, GLB binary buffers, and plain relative external
buffer paths confined to the source directory. Buffer/accessor extents/strides
are checked before decoding. Source/total-buffer cap is 64 MiB each (not an RSS
cap); output is at most 8,192 vertices/2,048 triangles, traversal at most 4,096
nodes/128 edges deep. Sparse/normalized accessors, other attributes/topologies,
assigned materials, animation, skin/morph data, extensions and authored cameras
fail explicitly. Invalid/nonunit rotations, singular/nonaffine/nonfinite transforms,
degenerate triangles and ambiguous/repeated/cyclic scene traversal also fail.
The reader is a bounded converter, not the complete Khronos validation tool.

Normalized JSON schema 1 records positions/indices plus source/buffer hashes,
selected node ranges/reflections, exact parser/tool version, compiler, OS/arch,
source/manifest/lock/binary hashes and debug configuration. Cooking preserves this
provenance plus native cooker/library/configuration digests and normalized JSON
hash. Outputs vary with compiler/binary/configuration by design; repeatability is
qualified for the same binary/input. File names/absolute source paths are not
persistent logical identities. Cooked THEIAMES v1 stays unchanged.

Six unit tests cover analytic nested TRS/reflection, independent instances,
invalid bounds/URIs/tree/features, external/embedded/GLB geometry parity, and
unlit primitive/instance assignment with unsupported-material rejection.
Rust fmt, denied-warning Clippy and tests run locally; Windows execution and
arbitrary third-party authoring exports remain unqualified.

## Optional opaque unlit factor import

```bash
cargo run --locked --manifest-path tools/theia-gltf-import/Cargo.toml -- fixtures/gltf/transformed_scene_unlit.gltf build/assets/imported-unlit-scene.json --unlit-factors
python3 tools/cook_meshlet_fixture.py --geometry build/assets/imported-unlit-scene.json --library <meshoptimizer-library> --source-archive <qualified-archive>
```

The opt-in mode requires an explicit `KHR_materials_unlit` material on every
selected primitive, `doubleSided: true`, alpha mode OPAQUE, finite [0,1] linear
RGBA factor with alpha exactly 1, and no base-color texture or extra vertex
attributes. Other extensions remain unsupported. The current renderer disables
face culling; requiring double-sided source materials preserves that authored
intent. Unlit ignores core PBR lighting/fallback properties as specified by
[Khronos](https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_unlit/README.md).
This is not general glTF material, lighting, UV/normal/vertex-color, texture,
mipmap or transparency support.

Normalized schema 2 adds dense `base_colors` and one `triangle_materials` index
per source-order triangle. Source material indices and selected primitive ranges
remain provenance; reused source materials retain one dense index. Reflected
nodes swap triangle winding while retaining the assignment. The cooker matches
winding-preserving cyclic global-index triangle identities through native meshlet
reordering, verifies the full coverage multiset and rejects conflicting material
assignments on duplicate indexed triangles. The resulting THEIAMAT v1 sidecar
binds the exact THEIAMES v1 hash; `imported-report.json` includes both file paths,
material counts/mapping and source/cooker/remapper provenance. No runtime glTF
parser is added and existing binary schemas remain unchanged.

Pass those paths to Render Lab `--asset <mesh> --materials <sidecar>` or the Rust
`theia-assets --example material` validator. The fixture contains two transformed
quads with distinct authored red/green materials, including a reflected node.
Repeated normalization/cooking with the same binary/input is byte-identical.
Old position-only mode remains supported; raw grid/quad/triangle fixture hashes
are unchanged. General source-material dependency streaming is still open.
