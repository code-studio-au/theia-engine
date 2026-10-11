# Theia — Linux Developer Workflow

## Verified reference environment

Linux Mint, RX 7900 XTX (Navi 31), Mesa RADV 26.2.4, `VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json`, Rust + Cargo 1.99.0, ash 0.38, Slang 2026.19 at `~/dev/tools/slang/bin`. Local repository: `~/dev/theia-engine`. Windows is a first-class intended platform but no Windows runtime execution was observed yet.

## Known-good commands

```bash
cd ~/dev/theia-engine
slangc -version
slangc shaders/compute/buffer_test.slang -entry computeMain -stage compute -target spirv -profile spirv_1_6 -o build/shaders/buffer_test.spv
spirv-val --target-env vulkan1.3 build/shaders/buffer_test.spv
spirv-dis build/shaders/buffer_test.spv | head -45
cargo fmt --manifest-path apps/theia-gpu-test/Cargo.toml
cargo clippy --manifest-path apps/theia-gpu-test/Cargo.toml
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation cargo run --manifest-path apps/theia-gpu-test/Cargo.toml
```

Expected GPU output: `GPU: AMD Radeon RX 7900 XTX (RADV NAVI31)`, `All 256 GPU values verified!`, first 8 odds 1…15, last 8 odds 497…511. Existing program prints `Vulkan debug messenger initialized.`. No validation warnings were observed; messenger callback delivery not independently proven.

## Project hygiene

`cargo new --bin --vcs none apps/theia-render-lab` (avoid nested `.git`). Commit `.slang`, `Cargo.lock`, `Cargo.toml` and source. Ignore `target/`, `**/target/`, `build/`, compiled `.spv`, logs and private ChatGPT data exports. Use topic branches, PRs and CI. Run `git status` before pushing.

## Direct Rust/Vulkan caution

Early compute test uses host-visible memory and `queue_wait_idle` on purpose and has large `unsafe` scopes. Production modules should add RAII, scoped unsafe with rationale, validated lifetime dependencies, device-local buffers and staged transfers. Query Vulkan feature chains; only enable supported features; do not assume mesh/task support from API version alone.

## Shader pipelines

Slang `[[vk::binding(0,0)]] RWStructuredBuffer<uint>` shader uses `[numthreads(64,1,1)]` entry `computeMain`. SPIR-V renamed entry `main` (disassembly verified). Compiled command `-profile spirv_1_6`; validate with Vulkan 1.3 target. M0-D qualification should test Slang task/mesh stages and alternate frontend only if specific codegen fails.

## If build fails

- Missing `cargo`: install Rust via official rustup.
- Crates.io DNS temporarily failed earlier: check `getent ahostsv4 static.crates.io` and HTTPS; later builds succeeded.
- Missing Slang: `~/dev/tools/slang/bin/slangc -version` expected 2026.19.
- Extra Vulkan ICD warning: use `VK_DRIVER_FILES` set to exact RADV JSON manifest.
- Multiple GPUs: explicitly select physical GPU using device properties, not ordinal 0 alone.
- Shader file relative path currently assumes launch from repository root.

## RenderDoc capture and replay

Run ordinary validation first; capture is a separate run. Tested: official
[RenderDoc 1.46 portable Linux build](https://renderdoc.org/builds), RADV/X11.
See [closeout evidence](validation/M0D_CLOSEOUT.md) for combined validation/capture
diagnostics. Windows capture instructions below are not yet runtime-qualified.

Build shaders and the Render Lab, then extract RenderDoc under ignored `build/`:

```bash
python3 tools/compile_render_shaders.py
cargo build --locked --manifest-path apps/theia-render-lab/Cargo.toml
mkdir -p build/qualification/renderdoc-tool
curl --fail --location https://renderdoc.org/stable/1.46/renderdoc_1.46.tar.gz \
  -o build/qualification/renderdoc-tool/renderdoc_1.46.tar.gz
tar -xzf build/qualification/renderdoc-tool/renderdoc_1.46.tar.gz \
  -C build/qualification/renderdoc-tool
```

The portable manifest contains its packaging-time library path. Correct a local
copy and expose it only to the qualification processes through `XDG_DATA_HOME`:

```bash
python3 - <<'PY'
import json
from pathlib import Path
tool = Path('build/qualification/renderdoc-tool/renderdoc_1.46').resolve()
layer = json.loads((tool / 'etc/vulkan/implicit_layer.d/renderdoc_capture.json').read_text())
layer['layer']['library_path'] = str(tool / 'lib/librenderdoc.so')
target = Path('build/qualification/renderdoc-data/vulkan/implicit_layer.d/renderdoc_capture.json')
target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(json.dumps(layer, indent=2) + '\n')
PY
XDG_DATA_HOME="$PWD/build/qualification/renderdoc-data" \
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc \
  --python tools/renderdoc_qualify.py
```

This starts a dedicated RenderDoc process, launches the 180-frame RX 7900 reference
run with validation explicitly disabled and debug output unmuted, captures frame
10, waits for target exit, replays and checks the compute/indirect mesh actions and
work buffer, and saves `build/qualification/renderdoc/report.json` and `replay.png`.
The helper exits that dedicated RenderDoc process; do not run it inside an existing
UI session. A missing capture or incorrect work produces nonzero exit/report error.
It does not inspect the child process's exit status; retain its terminal log and
verify the 180-frame completion when claiming runtime success.

Open the reported `.rdc` with `qrenderdoc` for interactive inspection. In the Event
Browser select compute dispatch and the indirect mesh sub-command. Inspect the
64-byte storage buffer, task/mesh pipeline stages, color output and mesh output.
Use RenderDoc shader debugging only where supported; this qualification verifies
capture, replay, work data and image, not interactive shader single-stepping.

On Windows, install the official matching RenderDoc build, build the `.exe`, and
run `qrenderdoc.exe --python tools/renderdoc_qualify.py` from the repository root.
The Linux layer/ICD environment commands do not apply. Alternatively use Launch
Application with the executable, repo-root working directory, `--gpu 7900
--frames 180 --no-validation`, API validation disabled, debug output unmuted, and
capture a frame manually. The helper's GPU filter is intentionally the reference
device; other hardware needs its own qualification input/evidence.

References: [RenderDoc quick start](https://renderdoc.org/docs/getting_started/quick_start.html),
[capture/replay API](https://renderdoc.org/docs/python_api/in_depth/capture_access.html).

For the bounded M2 visibility path, add `THEIA_CAPTURE_SPATIAL=1` and
`THEIA_CAPTURE_SCENARIO=0`, `1` or `2`, plus a separate
`THEIA_CAPTURE_OUTPUT=build/qualification/m2-renderdoc-0` (adjust per scenario)
to the same dedicated RenderDoc command. It captures the spatial mode, compares
resident bounds/CSR/candidate mask/visible IDs/indirect counts with the executable's
CPU-only `--spatial-reference` output, and exports the final presentation image.
Bounds comparison uses explicit absolute tolerance `1e-6`; integer lists/masks and
counts must match exactly. The three tested images contain 0, 8 and 32 triangles.

For M3 use `THEIA_CAPTURE_VISIBILITY=1` (without the spatial variable), a scenario
0/1/2 and separate `THEIA_CAPTURE_OUTPUT=build/qualification/m3-renderdoc-2`.
The helper verifies 84 identity/depth/resolved-color samples against CPU-only
`--visibility-reference`, including overlap, far-only and background. Depth
uses absolute `1e-6`; sRGB byte colors allow two levels of quantization tolerance.

FIFO desktop qualification can stall with the monitor powered off. An M3 smoke
run resumed and completed cleanly after temporarily waking the display with
`DISPLAY=:0 xset dpms force on`; permanent power settings were not changed.
This is a desktop-environment condition, not device-loss recovery evidence.

## M4 offline cooked fixtures

The initial slice uses official meshoptimizer 1.3 only in an offline process.
Run from the repo root; all native/binary outputs stay in ignored build directories:

```bash
mkdir -p build/qualification/meshoptimizer
curl --fail --location https://github.com/zeux/meshoptimizer/releases/download/v1.3/meshoptimizer-1.3.tar.gz \
  -o build/qualification/meshoptimizer/meshoptimizer-1.3.tar.gz
echo "d50a0b5e1323ba4379cdb32f89436c20652b7393445f8e18c88ae64445b67b77  build/qualification/meshoptimizer/meshoptimizer-1.3.tar.gz" | sha256sum --check -
tar -xzf build/qualification/meshoptimizer/meshoptimizer-1.3.tar.gz -C build/qualification/meshoptimizer
cmake -S build/qualification/meshoptimizer/meshoptimizer-1.3 \
  -B build/qualification/meshoptimizer/lib-build -DCMAKE_BUILD_TYPE=Release \
  -DMESHOPT_BUILD_SHARED_LIBS=ON -DCMAKE_CXX_COMPILER="$(command -v c++)"
cmake --build build/qualification/meshoptimizer/lib-build --parallel 4
python3 tools/cook_meshlet_fixture.py --library build/qualification/meshoptimizer/lib-build/libmeshoptimizer.so \
  --source-archive build/qualification/meshoptimizer/meshoptimizer-1.3.tar.gz
python3 tools/cook_meshlet_fixture.py --fixture quad --library build/qualification/meshoptimizer/lib-build/libmeshoptimizer.so \
  --source-archive build/qualification/meshoptimizer/meshoptimizer-1.3.tar.gz
```

Read generated `build/assets/grid16-report.json` and `quad-report.json` for paths.
Inspect with `cargo run --locked --manifest-path crates/theia-assets/Cargo.toml
--example inspect -- <file>`. Compile shaders/build Render Lab as above, then use
`--asset <grid-file> --gpu 7900 --wsi-smoke-test`. For capacity fallback add
`--fallback <quad-file> --asset-budget 2048`; omit `--frames` in smoke mode.

For captures set `THEIA_CAPTURE_ASSET=<grid-file>` without the spatial/visibility
variables, scenario 0/1/2 and separate `THEIA_CAPTURE_OUTPUT`. To verify capacity
fallback also set `THEIA_CAPTURE_FALLBACK=<quad-file>` and
`THEIA_CAPTURE_BUDGET=2048`. The dedicated RenderDoc helper compares the selected
GPU asset bytes and 81 CPU raster/color samples. Keep standalone validation runs
separate. Native cooker qualification is Linux-only; the Rust reader/viewer has
no native-tool dependency and Windows runtime is still untested.

[Format contract](M4_ASSET_CONTRACT.md), [executed evidence](validation/M4_COOKED_MESHLETS.md).

### M4 cell/page CPU residency qualification

Package the grid and quad paths from the cooker reports:

```bash
python3 tools/package_cell.py --cell 7 <grid-file> <quad-file>
cargo run --locked --manifest-path crates/theia-assets/Cargo.toml --example residency -- <printed-cell-file>
cargo test --locked --manifest-path crates/theia-assets/Cargo.toml
```

The example expects exactly two pages colocated with the manifest. It validates
hash-named files and demonstrates retained-reader budget pressure and release.
It performs CPU publication only; the quad is a second page in this test, with
no automatic fallback or GPU upload. Shader/viewer workflows above stay separate.

### M4 asynchronous two-page GPU residency

Run the same two-page manifest with:

```bash
cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- --cell <cell-file> --gpu 7900 --wsi-smoke-test
```

`--resident-budget` controls the fixed geometry arena's physical allocation cap
and logical capacity. On the qualified RX 7900 XTX fixture, 8,096 bytes permits
replacement and 8,080 defers it while retaining the coarse page. An 8,092 request
fails because backing requires 8,096 physical bytes. Limits are device
requirements, not universal constants. Default geometry budget is 16 MiB;
one staging allocation is separately bounded to 64 MiB. Omit smoke/frames to keep
the qualification cycling in a window. This is a bounded same-queue/single-frame
prototype, not complete world streaming.

For capture set `THEIA_CAPTURE_CELL=<cell-file>` plus `THEIA_CAPTURE_ASSET=<expected-bound-page>`
and `THEIA_CAPTURE_FRAME=<frame>` (0..149). Frame 10 verifies initial coarse, frame
30 fine, and frame 50 replacement coarse on the observed local run. Use
`THEIA_CAPTURE_RESIDENT_BUDGET=8080` with coarse expected at frame 30 for pressure.
Set `THEIA_CAPTURE_ARENA_BYTES` to arena logical capacity (default 16777216) and
`THEIA_CAPTURE_ARENA_OFFSET` to the expected page range: quad 0/grid 848 for
observed cell 7; terrain LOD 0/full 5528 for cell 8 on this GPU's 4-byte alignment.
For pressure use arena bytes 8080. These offsets are checked against replay data;
inspect runtime range logs on other devices rather than assuming alignment.
Upload timing is checked by matching actual captured bytes/counts/pixels; an
unexpected page fails the capture check. Keep capture separate from validation.

### M4 bounded offline heightfield LOD

```bash
python3 -m unittest discover -s tools -p test_height_lod.py
python3 tools/cook_meshlet_fixture.py --fixture terrain16 --library <meshoptimizer-library> --source-archive <qualified-archive>
python3 tools/cook_meshlet_fixture.py --fixture terrain16 --simplify-ratio .25 --library <meshoptimizer-library> --source-archive <qualified-archive>
```

Full/coarse paths appear in `build/assets/terrain16-report.json` and
`terrain16-lod-report.json`. Package them with cell 8 and use the cell workflows
above. `--simplify-error` is the native absolute metric (default 0.02);
`--max-height-error` is the independent sampled vertical limit (default 0.03).
They are different metrics. Sampling includes vertices, both triangulations'
centroids and a 65x65 grid; it does not bound unsampled error. Exact boundary
edges, winding and projected area must also pass. Meshlets preserve the simplified
input's complete triangle multiset; simplification intentionally changes the
original triangulation. Existing fixture recipes/content addresses remain intact.

### M4 perspective/depth qualification

Add `--camera-test` to `--asset` or `--cell`. The fixed Z-up rig looks from
(0,-1.7,1.5) toward (0,0,0.5), with 60-degree vertical FOV, 0.1/10 near/far,
Vulkan 0..1 depth and an explicit Y flip for a positive-height viewport. Aspect
changes with the actual swapchain extent. D32 depth uses LESS and clears to one.
The XY qualification viewer remains the default without this flag.

For capture add `THEIA_CAPTURE_CAMERA=1` to the existing expected page/arena
settings. The helper checks 81 perspective color/depth samples against the Rust
CPU oracle (`--camera-reference <mesh-file> <x y width height scenario>`). It
rejects inputs requiring near/far triangle clipping; this oracle is bounded,
while ordinary Vulkan clipping still applies to the GPU path. Captures remain
separate from standalone validation.

Cook `--fixture occlusion` with the existing native tool settings for a one-meshlet
near/far test. Its four triangles are deliberately ordered near (z=0.65) before
far (z=0.35), verified after meshlet optimization, so subsequent far primitives
must fail depth where overlapped. This is a diagnostic recipe, not a general
triangle reordering policy. CPU analytic tests also verify the center ray result.

### M4 bounded authored glTF geometry

```bash
cargo test --locked --manifest-path tools/theia-gltf-import/Cargo.toml
cargo run --locked --manifest-path tools/theia-gltf-import/Cargo.toml -- fixtures/gltf/transformed_scene.gltf build/assets/imported-scene.json
python3 tools/cook_meshlet_fixture.py --geometry build/assets/imported-scene.json --library <meshoptimizer-library> --source-archive <qualified-archive>
```

Use the path in `build/assets/imported-report.json` with `--asset <path>
--camera-test --wsi-smoke-test`. For replay use `THEIA_CAPTURE_CAMERA=1` plus
`THEIA_CAPTURE_ASSET=<path>` (no cell/arena variables for this standalone viewer).
The source fixture tests shared-mesh static nodes, nested transforms, reflection,
rotation and differing height. This converter rejects assigned materials, other
attributes, skin/animation, extensions and authored cameras. Full workflow and
source/buffer/geometry bounds: [normalizer contract](../tools/theia-gltf-import/README.md).

### M4 GPU meshlet frustum selection

Add `--frustum-test` to `--asset`/`--cell`. `--work-items 0|1|2` selects an
empty translated camera, narrow left camera (20-degree FOV), or full camera;
smoke mode cycles these views. This mode already includes perspective/depth.
Compute intersects outside-plane masks across each meshlet's referenced vertices,
rejects only a shared outside plane, compacts candidates and generates task work.
Candidates can still rasterize nothing; this is conservative, with no HZB/LOD.

For capture use `THEIA_CAPTURE_CULL=1` and expected page/arena settings as before.
The helper verifies GPU IDs/mask/counts against `--frustum-reference <mesh-file>
<x y width height view>`, then checks perspective pixels/depth. Full terrain's
observed masks are 0, 0x5f and 0xff at 960x540; candidate counts 0, 6, 8. Cell
frame 30 narrow capture also checks full terrain at arena offset 5528.

### Optional shared-vertex identity shaders

The specific Slang fragment PerPrimitiveEXT qualification failure permits a
small GLSL fragment fallback. Compile into a separate directory to preserve
default qualification shaders:

```sh
python3 tools/compile_render_shaders.py --slangc /path/to/slangc \
  --shared-primitive-interface --glslang /path/to/glslangValidator \
  --output-dir build/shaders-shared
apps/theia-render-lab/target/debug/theia-render-lab --asset /path/to/hash.theia-mesh \
  --material-test --shader-dir build/shaders-shared --wsi-smoke-test
```

Use `THEIA_CAPTURE_SHADER_DIR=build/shaders-shared` with the existing material
capture environment to replay that exact pair. Pinned compiler provenance and
limits are in [M4 qualification evidence](validation/M4_COOKED_MESHLETS.md).

### Authored opaque factors

Cook explicit material assignments against an exact geometry page, then validate
with the independent Rust reader before running GPU qualification:

```sh
python3 tools/cook_material_page.py --mesh build/assets/<hash>.theia-mesh \
  --assignments fixtures/materials/imported_unlit.json
cargo run --manifest-path crates/theia-assets/Cargo.toml --example material -- \
  build/assets/<hash>.theia-mesh build/assets/<material-hash>.theia-material
apps/theia-render-lab/target/debug/theia-render-lab --asset build/assets/<hash>.theia-mesh \
  --materials build/assets/<material-hash>.theia-material --wsi-smoke-test
```

The fixture's geometry hash is explicit; use its bound imported asset or author
new assignments for a different cooked file. `THEIA_CAPTURE_MATERIAL_PAGE`
selects authored factor replay and checks every visible triangle plus the grid.
Optional `THEIA_CAPTURE_SHADER_DIR=build/shaders-shared` checks the alternate
shared primitive interface. These tests do not implement glTF material import.

### Authored UV reconstruction qualification

```sh
cargo run --manifest-path tools/theia-gltf-import/Cargo.toml -- \
  fixtures/gltf/transformed_scene_uv.gltf build/assets/imported-uv-scene.json --unlit-uv
python3 tools/cook_meshlet_fixture.py --geometry build/assets/imported-uv-scene.json \
  --library <qualified-meshoptimizer-library> --source-archive <qualified-archive>
apps/theia-render-lab/target/debug/theia-render-lab --asset <mesh-path-from-report> \
  --uv-test <uv-path-from-report> --wsi-smoke-test
```

`THEIA_CAPTURE_UV_PAGE` selects identity/depth/UV-color replay; optional
`THEIA_CAPTURE_SHADER_DIR=build/shaders-shared` uses the qualified shared identity
pair. This diagnostic does not sample authored textures. Use only fixed geometry;
UV/cell/fallback/material combinations are not implemented by this mode.

## Bounded offline KTX2 texture qualification

Use official KTX-Software 4.4.2 source with SHA-256
`9412cb45045a503005acd47d98f9e8b47154634a50b4df21e17a1dfa8971d323`.
Build recipe and licence scope: [qualification](validation/M4_TEXTURES.md).

```bash
cmake -S <ktx-source> -B <ktx-build> -DCMAKE_BUILD_TYPE=Release \
  -DKTX_GIT_VERSION_FULL=v4.4.2 -DKTX_FEATURE_ETC_UNPACK=OFF \
  -DKTX_FEATURE_TESTS=OFF -DKTX_FEATURE_DOC=OFF -DKTX_FEATURE_TOOLS=ON
cmake --build <ktx-build> --target ktxtools --parallel 4
python3 tools/cook_texture.py --ktx <ktx-build>/Release/ktx --build-dir <ktx-build> \
  --source-archive <qualified-source-archive> --source fixtures/textures/opaque_2x2.json
cargo run --locked --manifest-path crates/theia-assets/Cargo.toml --example texture -- <hash.ktx2>
```

The initial source is explicit row-major RGBA8 JSON, not a general image importer.
The official validator's custom-provenance-key warning is expected and recorded.

## Resident texture sampling qualification

```bash
apps/theia-render-lab/target/debug/theia-render-lab --asset <mesh> \
  --materials <material-page> --uv-test <uv-page> --texture-test <hash.ktx2> \
  --gpu 7900 --wsi-smoke-test
```

This fixed-asset profile applies one nearest-repeat mip-zero texture to all
opaque unlit factors. `--texture-budget <bytes>` caps actual image memory
requirements before allocation. It rejects cell/fallback combinations.
CPU reference: `--texture-reference <mesh material uv texture x y width height view>`.
Capture uses `THEIA_CAPTURE_TEXTURE_PAGE` together with `THEIA_CAPTURE_ASSET`,
`THEIA_CAPTURE_UV_PAGE` and `THEIA_CAPTURE_MATERIAL_PAGE`; select default/shared
identity shaders as before. Texture replay checks exact GPU texels, both sidecar
buffers, identities, depth and sampled colours against the CPU reference.

## Fixed textured dependency bundle

```bash
python3 tools/package_texture_bundle.py --mesh <mesh> --materials <materials> \
  --uvs <uvs> --texture <ktx2>
cargo run --locked --manifest-path crates/theia-assets/Cargo.toml --example bundle -- <hash.theia-bundle>
apps/theia-render-lab/target/debug/theia-render-lab --bundle <hash.theia-bundle> \
  --gpu 7900 --wsi-smoke-test
```

Children must be colocated in the output directory; no automatic copying occurs.
Bundle admission checks all four children before Vulkan initialization and derives
their sibling paths. Explicit asset/cell/material/UV/texture paths are mutually
exclusive with `--bundle`. For replay set `THEIA_CAPTURE_BUNDLE` plus all four
expected child variables; the helper verifies the manifest matches those expected
resources and launches the bundle entry path. Physical texture budget and shader
directory options remain available.

For the authored-image path use `theia-gltf-import --unlit-texture` with
`fixtures/gltf/transformed_scene_texture.gltf`, then pass the qualified
`--ktx`, `--ktx-build` and `--ktx-source-archive` options to the meshlet cooker.
`build/assets/imported-report.json` includes geometry, factors, UV, texture and
bundle paths. Run the emitted bundle as above. Standalone image cooking supports
`tools/cook_texture.py --png --source <rgba8.png>` with the same qualified tool
arguments. These narrow source/sampler limits are documented in the
[normalizer workflow](../tools/theia-gltf-import/README.md).

## Bounded material-to-texture CPU table

```bash
python3 tools/cook_texture_table.py --materials <material-page> --assignments <mapping.json>
cargo run --locked --manifest-path crates/theia-assets/Cargo.toml --example texture_table -- <mesh> <material-page> <table>
```

Assignments are schema 1, exact `material_sha256`, `textures` hash-named KTX2
paths, and one `material_textures` source index per material. All listed textures
must be used. The cooker canonicalizes hash order and remaps the indices; the
Rust example validates every sibling texture before admitting the set. This
table's file-backed admission is independent of its subsequent GPU path.

GPU qualification uses `--asset <mesh> --materials <page> --uv-test <uv>
--texture-table <table> --gpu 7900 --wsi-smoke-test`. `--texture-budget` caps
aggregate physical image allocations; table/staging have separate limits. It
rejects single-texture/bundle/cell/fallback combinations. CPU oracle:
`--texture-table-reference <mesh material uv table x y width height view>`.
For replay set `THEIA_CAPTURE_TEXTURE_TABLE` with expected asset, factor and UV
paths, without single texture/bundle variables. Reports include table/GPU texel
checks and sampled material/texture indices. Only one/two texture counts have
executed GPU qualification so far.

## Material-selected authored images

Normalize `fixtures/gltf/transformed_scene_textures.gltf` using the importer
with `--unlit-textures`; feed its JSON to the existing meshlet cooker with the
qualified meshoptimizer library/archive and KTX tool/build/archive arguments.
`build/assets/imported-report.json` gives geometry, material, UV, texture table,
images and v2 bundle paths. Launch `--bundle <report bundle path> --gpu 7900
--wsi-smoke-test` for standalone validation. For separate RenderDoc replay,
set `THEIA_CAPTURE_BUNDLE`, `THEIA_CAPTURE_ASSET`,
`THEIA_CAPTURE_MATERIAL_PAGE`, `THEIA_CAPTURE_UV_PAGE` and
`THEIA_CAPTURE_TEXTURE_TABLE` to the matching report paths; omit
`THEIA_CAPTURE_TEXTURE_PAGE`. Capture validation remains disabled; standalone
synchronization validation supplies independent diagnostic evidence.

## Complete-bundle CPU ownership qualification

`cargo run --locked --manifest-path crates/theia-assets/Cargo.toml --example
bundle_residency -- <first bundle> <second bundle>` loads two hash-named complete
sets from their sibling directories. Choose different non-contained sets, such
as the two-image and alias fixture bundles. It verifies duplicate-stage sharing,
atomic replacement, retained-reader budget rejection and last-owner reuse. This
example performs no Vulkan work.

## Complete indexed-bundle streaming

Launch `--bundle <two-image v2 bundle> --stream-bundle <alias v2 bundle>
--gpu 7900 --wsi-smoke-test`. The reference pair requires 11012 unique CPU
encoded bytes, 21120 combined active/pending resource bytes, and 12288 combined
image bytes on the qualified RADV device. Set `--bundle-byte-budget`,
`--bundle-gpu-budget` and `--texture-budget` for explicit pressure tests. Defaults
are 64 MiB, 32 MiB and 4 MiB respectively; physical charges vary by device.

For replay retain `THEIA_CAPTURE_BUNDLE` and expected role paths for the set
that should be active at the capture frame. Add `THEIA_CAPTURE_INITIAL_BUNDLE`
and `THEIA_CAPTURE_STREAM_BUNDLE` for actual launch. The reference pair captures
initial at frame 10, alias at 30, restored initial at 50. Every capture independently
checks actual GPU bytes and table/sampled colours, so late publication cannot
silently pass as the expected set. This mode rebuilds graphics generations and
waits device idle at publication; it is not a performance test.

## Continuous heightfield LOD error

The existing terrain cooker with `--simplify-ratio .5` or `.25` and qualified
meshoptimizer arguments now emits exact continuous vertical-error metadata in
addition to sampled/native error evidence. `--max-height-error` gates both
checks. Run `python3 -m unittest discover -s tools -p test_continuous_height_error.py`
for analytic diagonal extrema, continuity/coverage failures and malformed
profiles. This offline check covers represented rectangular XY heightfields;
it does not select runtime LOD or qualify arbitrary 3D geometry.

## CPU projected LOD reference

`theia-render-lab --lod-reference <fine mesh> <coarse mesh> 960 540 2 10`
runs without Vulkan/window initialization. Use the full terrain and newly
continuous-qualified ratio-.25/.5 pages. Its last output line is JSON containing
source IDs, represented error, per-axis pixel bounds, chosen level/reason and
`proof_recomputed=false`. At ratio .25 a budget 10 chooses coarse at 960x540,
fine at 1920x1080 or view 1 (narrow); budget 1 keeps fine. This is a CPU oracle
and does not change the viewer's GPU page selection.

## GPU resident LOD qualification

Launch `--lod-test <full terrain page> <continuous-qualified coarse page>
--lod-pixels 6 --gpu 7900 --wsi-smoke-test`. Both pages remain resident; GPU
compute chooses a page, culls it and feeds indirect task/mesh draws.
`--lod-budget 13232` qualifies exact geometry allocation on the reference
128-triangle pair; 13231 fails safely after the fine upload. Default is 4 MiB.

For capture set `THEIA_CAPTURE_LOD_FINE`, `THEIA_CAPTURE_LOD_COARSE`,
`THEIA_CAPTURE_LOD_PIXELS` and `THEIA_CAPTURE_ASSET` to the expected selected
page. Set scenario 2/budget 6 for coarse, scenario 2/budget 1 or scenario 1/budget
6 for fine. Replay verifies both buffers, selected header/work, CPU bounds and
framebuffer. Static LOD captures can use frame 10/30. Streaming bundle captures
must explicitly choose the correct epoch frame (e.g. alias at frame 30).

`tools/qualify_lod_guards.py --fine <page> --coarse <page> --translation-y 32
--output-dir <ignored directory>` makes test-only far-plane fixtures. Use -1.5
for the near-crossing case; exact continuous/boundary verification is recomputed
after represented position rewriting. Original native topology/provenance are
retained as nested evidence, without claiming a new native simplification.
`--small-flat-bound` on the flat pair supplies a conservative 1e-8 bound over
true zero error, exercising unsupported-numeric fallback. For near-only work
replay set `THEIA_CAPTURE_LOD_GUARD_ONLY=1`; its report explicitly leaves
framebuffer verification false. `--frustum-ids <mesh width height view>` permits
CPU culling-ID checks without the unclipped raster oracle.

## Local authoritative movement lab

```bash
cargo test --locked --manifest-path crates/theia-sim/Cargo.toml
cargo clippy --locked --manifest-path crates/theia-sim/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path apps/theia-server-lab/Cargo.toml -- --ticks 10000
```

The headless lab runs fixed **simulation** steps as fast as the local test loop
can execute; it does not listen on sockets or pace a realtime server. Tick count
is bounded 1..100000. Three actors exercise server-owned movement/collision,
replay/invalid-intent rejection and change streams. The collider fixture is a
flat floor plus static rectangles, independent of renderer LOD or GI. Nine
library tests and default/100000-tick executions pass locally.

## Local authoritative combat lab

`cargo run --locked --manifest-path apps/theia-server-lab/Cargo.toml --
--combat-test` executes a 5000-tick independent health/cooldown comparison plus
static LOS, movement-before-combat and same-tick death/order cases. It cannot
combine with `--ticks`, which selects the separate movement run. Both operate
without sockets/GPU. Sixteen library tests qualify both movement and combat.

## M8 typed request qualification

```sh
cargo test --locked --manifest-path crates/theia-protocol/Cargo.toml
cargo clippy --locked --manifest-path crates/theia-protocol/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path apps/theia-server-lab/Cargo.toml -- --protocol-test
```

This runs typed messages locally, without sockets or encryption. Test modes are
mutually exclusive with `--ticks`/`--combat-test`. The lab validates mixed sequence
spaces, shuffled request arrivals, duplicate results, ACK holes, bounded cache
pressure and an independent health model. See
[request evidence](validation/M8_REQUEST_SEQUENCING.md) for limits.

## M8 encrypted loopback qualification

```sh
cargo test --locked --manifest-path apps/theia-network-lab/Cargo.toml
cargo clippy --locked --manifest-path apps/theia-network-lab/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path apps/theia-network-lab/Cargo.toml
# On a host with enabled IPv6 loopback (not qualified on this machine):
cargo run --locked --manifest-path apps/theia-network-lab/Cargo.toml -- --ipv6
```

This separate process hosts two clients and one authority over ephemeral
localhost UDP ports. It generates a certificate in memory and trusts that
specific certificate; no verification bypass or private-key file exists.
The whole qualification has a 20-second deadline. No account service or persistent
state is involved. See [evidence and limits](validation/M8_QUINN_LOOPBACK.md).

Run the same encrypted two-client checks through controlled UDP faults:

```sh
cargo run --locked --manifest-path apps/theia-network-lab/Cargo.toml -- --faults
```

Faults activate after verified TLS handshakes and preserve encrypted packets.
The relay holds at most two packets and logs observed drop/reorder/duplicate
counts. Movement retries until the applied watermark; complete snapshots recover
lost state. See [fault evidence](validation/M8_NETWORK_FAULTS.md) for limits.

The network lab now routes consumed snapshots through the readonly replica in
`theia-protocol::replica`. Its five tests run with the protocol suite. Connection
reset clears previous state; older/conflicting snapshots cannot overwrite newer
presentation data. See [client snapshot evidence](validation/M8_CLIENT_SNAPSHOTS.md).

`theia-protocol::presentation` now provides explicit-clock bounded interpolation
against the last two accepted complete snapshots. The protocol tests include
3000 independent numeric samples; both network modes check finite readonly
presentation samples. These sample times are test inputs, not a synchronized
screen clock. See [evidence](validation/M8_PRESENTATION_INTERPOLATION.md).

## M7 fixed GPU authority presentation

```sh
python3 tools/compile_render_shaders.py --slangc /path/to/slangc
cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- --authority-reference 0.5
cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- --gpu 7900 --authority-test --smoke-test --wsi-smoke-test
```

`--authority-alpha 0..1` chooses interpolation fraction, default .5. This fixture
uses actual server-owned movement/combat ticks and a readonly CPU oracle; no
keyboard control/live network is implemented yet. Capture/replay qualification:

```sh
THEIA_AUTHORITY_ALPHA=.5 THEIA_CAPTURE_OUTPUT="$PWD/build/qualification/authority-replay" qrenderdoc --python tools/renderdoc_authority_qualify.py
```

Use the preceding local RenderDoc layer/XDG setup. Captures explicitly disable
validation; standalone runs separately qualify Vulkan synchronization/lifetime.
See [evidence](validation/M7_GPU_AUTHORITY_PRESENTATION.md).

## M7 interactive local demo

```sh
cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- --gpu 7900 --authority-demo
cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- --authority-demo --authority-script --frames 600 --gpu 7900
cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- --authority-script-reference
```

Physical WASD/arrows move, Space press requests an attack on the second actor,
Escape quits. Focus loss clears held controls. Gameplay is the local authority
fixture in this process; no live server connection or account service exists.
`--authority-script` generates bounded directional/attack intents and requires
26 fixed ticks to qualify death and static collision. Choose enough frames for
the display refresh rate; an insufficient explicit limit fails qualification.
`--authority-alpha` remains fixed-test-only because the demo owns its clock.

Native X11/XTest qualification (offline Python/third-party Xlib/Xtst only):

```sh
python3 tools/qualify_authority_input.py
THEIA_AUTHORITY_SCRIPT=1 THEIA_CAPTURE_OUTPUT="$PWD/build/qualification/local-demo-capture" qrenderdoc --python tools/renderdoc_authority_qualify.py
```

The input harness identifies only its spawned PID/window, temporarily focuses it,
checks focus before each key-down, releases keys and restores focus when still
owned. Run it in the qualified X11 desktop session. Replay captures the stable
script result at frame 300 of 600; the selected display must allow at least 26
fixed ticks before capture. Windows/Wayland input runtime remain unqualified.
See [evidence and scheduler limits](validation/M7_INTERACTIVE_AUTHORITY.md).

The shared encrypted fixture's strict codec tests now run with
`cargo test --locked --manifest-path crates/theia-network-fixture/Cargo.toml`;
the network executable retains its app-local relay test and direct/faulted runs.
These are still bounded qualification APIs, not a production networking SDK.

## Separate encrypted network listener

```sh
cargo run --locked --manifest-path apps/theia-network-lab/Cargo.toml -- --serve
# Read address and public certificate path from the printed JSON, then in two terminals:
cargo run --locked --manifest-path apps/theia-network-lab/Cargo.toml -- --client-probe 127.0.0.1:PORT /path/to/localhost.der
# Portable process qualification (build first):
python3 tools/qualify_network_listener.py
python3 tools/qualify_network_listener.py --laboratory --output build/qualification/listener-lab
python3 tools/qualify_network_listener.py --second-client-delay 6 --output build/qualification/listener-delayed
```

Listener defaults to local-demo profile; `--laboratory-profile` selects the preceding
empty-floor test. `--listen` must name loopback; `--certificate` names a public DER
output only. No private key is written. Two successful peers are required; one
closing ends the pair. Probe commands validate assigned actor movement/readonly
state, then exit. This unit does not yet start a graphics client.
[Bootstrap evidence](validation/M8_NETWORK_LISTENER.md).

## Separate encrypted GPU viewers

Build both labs and shaders first, start `theia-network-lab --serve`, then use its
printed address/public certificate in **two** Render Lab processes:

```sh
cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- --network-client 127.0.0.1:PORT --network-certificate /path/to/localhost.der
# Repeatable desktop/GPU qualification after building both labs:
python3 tools/qualify_network_viewer.py --wsi
python3 tools/qualify_network_viewer.py --laboratory --output build/qualification/network-viewer-lab
python3 tools/qualify_authority_input.py --network --output build/qualification/network-input.log
THEIA_AUTHORITY_NETWORK=1 THEIA_CAPTURE_OUTPUT="$PWD/build/qualification/network-capture" qrenderdoc --python tools/renderdoc_authority_qualify.py
```

WASD/arrows and Space send intents to the separate authority. The listener waits
for two peers; closing either ends the pair. Certificates contain only the public
DER trust anchor; no signing key is saved. Live mode owns its input/arrival clock
and cannot combine local authority modes/alpha. Before bootstrap or after closure
it displays no actors. Replay variants use `THEIA_AUTHORITY_LABORATORY=1`,
`THEIA_AUTHORITY_UNPAIRED=1` or `THEIA_AUTHORITY_CLOSED=1` alongside NETWORK.
Unpaired replay checks empty geometry; the application's no-snapshot completion
check deliberately fails. These helpers require a qualified GPU/desktop; CI only
builds/lints/tests and executes CPU/network fixtures. Native input is X11-only.
[Evidence and limits](validation/M8_NETWORK_VIEWER.md).

Live encrypted packet-fault qualification:

```sh
python3 tools/qualify_network_viewer.py --faults --wsi --output build/qualification/live-fault-wsi
PYTHONPYCACHEPREFIX=build/python-cache python3 tools/qualify_authority_input.py --network --faults --output build/qualification/live-fault-input.log
THEIA_AUTHORITY_NETWORK=1 THEIA_AUTHORITY_FAULTS=1 THEIA_CAPTURE_OUTPUT="$PWD/build/qualification/live-fault-capture" qrenderdoc --python tools/renderdoc_authority_qualify.py
```

Manual clients can add `--network-faults`. Faults activate after verified bootstrap;
final relay reports require actual positive drop/delay/reorder/duplicate counts and
at most two retained packets. Native harness checks both viewers' reports.
[Evidence/teardown limits](validation/M8_LIVE_PACKET_FAULTS.md).

Transport-neutral bounded delta reconstruction/full-state repair oracle:

```sh
cargo test --locked --manifest-path crates/theia-protocol/Cargo.toml
cargo run --locked --manifest-path crates/theia-protocol/Cargo.toml --example delta_repair
```

This is a synthetic application-message loss/reordering test. The encrypted THN2
listener/viewers still exchange full snapshots; no transport baseline ACK/cache
policy or network repair request is added. [Evidence](validation/M8_DELTA_RECONSTRUCTION.md).

Explicit application-acknowledged sender baseline oracle:

```sh
cargo run --locked --manifest-path crates/theia-protocol/Cargo.toml --example acknowledged_baselines
```

Models bounded per-recipient prepared history, state/ACK loss/reordering, eviction
and full repair. Neither baseline ACKs nor deltas are wired into THN2 yet.
[Evidence](validation/M8_ACKNOWLEDGED_BASELINES.md).

The separate bounded TDR1 replication codec runs with the shared fixture tests:
`cargo test --locked --manifest-path crates/theia-network-fixture/Cargo.toml`.
It is not yet used by the THN2 listener/viewers. [Evidence](validation/M8_REPLICATION_CODEC.md).

Isolated encrypted full/delta/snapshot-ACK/full-repair qualification:

```sh
cargo run --locked --manifest-path apps/theia-network-lab/Cargo.toml -- --replication-test
cargo run --locked --manifest-path apps/theia-network-lab/Cargo.toml -- --replication-test --faults
```

Uses a synthetic complete-state source and independent client oracle, deliberately
withholds early ACKs to exercise missing-baseline repair, verifies final tick 120
before closure and bounds the whole run to 20 seconds. This test does not replace
live THN2 gameplay/viewer transport. [Evidence](validation/M8_ENCRYPTED_REPLICATION.md).

Live authority/viewers now require THN3/ALPN `theia-lab-3`; rebuild **both** labs
before pairing. Old versions reject. TDR1 snapshot ACKs govern only replication
baselines, independently of movement sequences and reliable combat outcome ACKs.
For explicit baseline-eviction/full-repair qualification:

```sh
python3 tools/qualify_network_viewer.py --faults --repair --wsi --output build/qualification/live-delta-repair
THEIA_AUTHORITY_NETWORK=1 THEIA_AUTHORITY_FAULTS=1 THEIA_AUTHORITY_REPAIR=1 THEIA_CAPTURE_OUTPUT="$PWD/build/qualification/live-delta-replay" qrenderdoc --python tools/renderdoc_authority_qualify.py
```

Manual `--network-repair-test` withholds snapshot ACKs after a reconstructed delta
until its missing baseline triggers full repair. It is explicit test injection,
never default input/network behavior. Live replication counters distinguish accepted
full/delta state, queued ACK/repair and intentionally withheld ACKs. Profile roster
stays exactly two server-assigned actors; general lifecycle replication remains open.
[Evidence](validation/M8_LIVE_DELTA_REPLICATION.md).

## Isolated nested Wayland qualification

Extract Ubuntu Noble Weston 13.0.0-4build3 `weston`/`libweston-13-0` packages below
`build/qualification/wayland-tool/root` using `dpkg-deb --extract`; no system install
is needed. Qualified package hashes and setup limits are in the evidence report.
Build both labs and keep the RADV ICD/display setup from earlier qualification.

```sh
python3 tools/qualify_wayland.py --network --output build/qualification/wayland-qualified
```

The helper creates an owned mode-700 temporary runtime/socket, starts only its
Weston X11/GL compositor, removes DISPLAY from Rust client environments, checks
actual Wayland backend logs, and closes its owned processes/runtime. It runs six
120-frame WSI modes and optionally the faulted live delta/full-repair pair.
Wayland smoke explicitly skips unsupported programmatic minimize/restore; no silent
X11 fallback. Weston remains a third-party offline qualification tool.

Optional `--capture` attempts the existing local RenderDoc pipeline and requires
replay success. The available 1.46 portable build rejects VK_KHR_wayland_surface,
so this option currently fails with that explicit limitation. Do not report a
capture/validation pass from that failed attempt. Native-seat, user-driven window
restore, Windows and cross-vendor execution remain unqualified.
[Evidence and primary references](validation/WAYLAND_NESTED.md).
