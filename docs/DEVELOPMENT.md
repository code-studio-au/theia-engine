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
