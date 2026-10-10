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

Supported: selected default scene (or sole scene), position-only unmaterialed
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
source/lock/binary hashes and debug configuration. Cooking preserves this
provenance plus native cooker/library/configuration digests and normalized JSON
hash. Outputs vary with compiler/binary/configuration by design; repeatability is
qualified for the same binary/input. File names/absolute source paths are not
persistent logical identities. Cooked THEIAMES v1 stays unchanged.

Four unit tests cover analytic nested TRS/reflection, independent instances,
invalid bounds/URIs/tree/features and external/embedded/GLB geometry parity.
Rust fmt, denied-warning Clippy and tests run locally; Windows execution and
arbitrary third-party authoring exports remain unqualified.
