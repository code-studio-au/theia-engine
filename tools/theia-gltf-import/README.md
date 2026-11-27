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

Ten unit tests cover analytic nested TRS/reflection, independent instances,
invalid bounds/URIs/tree/features, external/embedded/GLB geometry parity, and
unlit primitive/instance assignment with unsupported-material rejection, and
float2 UV count/type/finite/seam/reflection checks, and bounded shared-image
bytes/material/sampler/URI/PNG-envelope admission.
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

## Optional authored float2 UVs

Use `--unlit-uv` instead of `--unlit-factors` with
`fixtures/gltf/transformed_scene_uv.gltf`. Normalized schema 3 adds one float2
TEXCOORD_0 per dense global vertex. Selected primitives require FLOAT VEC2 UVs
with POSITION-matching count; other sets/attributes and normalized/integer UVs
fail. Node transforms/reflections change world positions/winding while leaving
UV values unchanged. Coincident positions with different UVs retain distinct
vertex rows. Existing unlit factor restrictions still apply; textures remain
unsupported by this mode.

The native cooker emits geometry, material and UV file paths in
`imported-report.json`. Validate with Rust `theia-assets --example uv <mesh> <uv>`
and run Render Lab `--asset <mesh> --uv-test <uv>`. That mode displays reconstructed
UVs as diagnostic color and qualifies perspective interpolation; it does not
sample authored textures or implement material/UV dependency residency.

## Optional authored shared PNG texture

```bash
cargo run --locked --manifest-path tools/theia-gltf-import/Cargo.toml -- fixtures/gltf/transformed_scene_texture.gltf build/assets/imported-textured-scene.json --unlit-texture
python3 tools/cook_meshlet_fixture.py --geometry build/assets/imported-textured-scene.json \
  --library <meshoptimizer-library> --source-archive <qualified-meshoptimizer-archive> \
  --ktx <ktx-build>/Release/ktx --ktx-build <ktx-build> --ktx-source-archive <qualified-ktx-archive>
apps/theia-render-lab/target/debug/theia-render-lab --bundle <bundle-from-report> --gpu 7900 --wsi-smoke-test
```

The opt-in mode retains schema-3 geometry/UV/unlit requirements and requires
every selected material to reference the same source base-colour texture with
TEXCOORD_0 and explicit NEAREST min/mag filters, REPEAT S/T and no texture/info/
sampler/image extensions. Other texture indices, missing base-colour textures,
other filters/wrap modes and texture transforms fail. Lighting fallback
properties remain ignored by unlit semantics; this is not a PBR importer.

Only an external unescaped relative image path confined to the source directory
is supported, with optional MIME image/png. Absolute/network/data URIs, directory
traversal, symlink escape and image buffer views fail. Encoded PNG cap is 4 MiB,
dimensions 1..1024, noninterlaced 8-bit RGBA envelope; total buffers plus image
remain <=64 MiB. Rust checks the envelope and carries exact encoded PNG bytes;
KTX performs full decoding/CRC/opaque-alpha checks offline during cooking.

Normalized schema 4 adds a single encoded `texture` record with source image/
texture indices, base64 PNG, digest/dimensions/sampler, and `texture_materials`
containing one zero index per dense material. Geometry provenance includes the
image dependency. The cooker checks source hash/dimensions/mapping, invokes
pinned KTX-Software, writes unchanged geometry/material/UV formats plus KTX2,
and packages the four-role bundle. Runtime still has no glTF/PNG/native decoder.
Opaque sRGB texture sampling multiplies the authored linear factors. The
synthetic transformed/reflected fixture exercises two factors, eight UV rows,
four triangles and repeating UVs outside [0,1]. No multiple-texture material
table, mip/gradient/filtering, transparency or dependency streaming claim.

## Optional material-selected PNG set

Use `--unlit-textures` with `fixtures/gltf/transformed_scene_textures.gltf`.
The same cooker arguments shown above produce schema-5 geometry, factors, UVs,
KTX2 images, a dense material texture table and a THEIABND v2 manifest. Run
`--bundle <bundle-from-report>` to select the complete indexed set.

All shared-PNG restrictions still apply. Up to 16 source texture bindings are
accepted; exact PNG bytes deduplicate image aliases. Source bindings remain in
provenance; normalized material indices address unique images. Unique images
plus source buffers and the final JSON each have a 64 MiB cap. Different PNG
encodings are not merged by decoded pixels. The original `--unlit-texture`
profile and v1 bundle remain available. Twelve importer tests pass locally;
GPU qualification covers distinct two-image bindings and a one-image alias set.
