# M3 bounded visibility/material reference — 2026-10-10

Executed on RX 7900 XTX, Linux Mint/X11, RADV Mesa 26.2.4, Vulkan
loader/validation 1.3.275, Rust 1.99.0, Slang 2026.19 and RenderDoc 1.46.

## Slice and contracts

`--visibility-test` preserves all earlier modes and adds a separate two-pass path.
Slang compute writes indirect work; task/mesh generates up to two overlapping
triangles without vertex/index buffers. The first pass writes frame-local
`(instance+1, primitive)` into `R32G32_UINT` and depth into `D32_SFLOAT`.
Zero identity is background; both test instances contain primitive zero. Smaller
near geometry is at depth 0.25, larger far geometry at 0.75.

A fullscreen mesh/fragment pass loads identity/depth, reconstructs barycentric
attributes for fixed w=1 geometry, and samples a runtime array of two RGBA8 material
textures using `NonUniformResourceIndex`. The near material is white; the far
material is `(89,153,255)/255`. Both textures are GPU-cleared 1×1 images. There is
no host identity/depth/count readback in rendering. The production identity bit
budget, material table and general geometry representation remain undecided.

The host queries format capabilities before selection. Offscreen images, sampler,
descriptor pool/layout and both pipelines have RAII; construction failures release
children before device destruction. Resource-scoped Sync2 image barriers order
old sampled reads, new attachment/transfer writes and resolve sampling. Integer
and depth images are recreated with the swapchain. Runtime descriptor-array,
nonuniform sampled indexing and partially bound features are queried/enabled;
the two material slots are fully populated and no in-flight descriptor update is used.

## Observed verification

```bash
python3 tools/compile_render_shaders.py
cargo fmt --manifest-path apps/theia-render-lab/Cargo.toml -- --check
cargo clippy --locked --manifest-path apps/theia-render-lab/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path apps/theia-render-lab/Cargo.toml
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
  cargo run --locked --manifest-path apps/theia-render-lab/Cargo.toml -- \
  --gpu 7900 --visibility-test --wsi-smoke-test
```

Fourteen render outputs compile and pass Vulkan 1.3 SPIR-V validation. Resolve SPIR-V
contains `RuntimeDescriptorArray`, `ShaderNonUniform`,
`SampledImageArrayNonUniformIndexing`, `NonUniform` decorations and
`OpImageSampleExplicitLod`. Slang emits E41012 profile-upgrade warning for material
sampling; disassembly records the emitted capabilities rather than treating the
compiler's broader profile list as executed device features. No language fallback.

Nine CPU tests pass, including reference overlap/depth/barycentric checks. M3 smoke
cycles empty, near-only and near+far inputs, executes all four labeled WSI probes,
resizes/minimizes/restores and completes 120 frames/ten generations with **zero
validation warnings/errors including teardown**. A run temporarily stalled while
DPMS had the monitor off; waking the display allowed completion. No device-loss
or powered-off presentation qualification is claimed.

Three separate RenderDoc captures (validation disabled as documented) each finish
180 frames and verify **84 pixels** against an independent CPU reference. Integer
identity matches exactly, depth within absolute `1e-6`, and resolved sRGB color
within two byte levels. Samples include center overlap, far-only area and clear
background, plus a 9×9 sample grid. Scenario 0 has clear identity/depth; scenario 1
has near geometry only; scenario 2 shows near identity winning overlap and far
identity/material outside it. Replay PNG visually shows the correct layered image.
Diagnostic texture readback is confined to the offline debugger helper.

Indirect, direct-task, mesh-only, clear-only and spatial smoke regressions all pass
with zero warnings/errors. Missing resolve fragment after work, target images,
sampler/descriptors and the identity pipeline are created exits 1 with explicit
file error and zero teardown diagnostics. The original headless compute and M1
qualification applications remain unchanged. Raw artifacts live under ignored
`build/qualification/`; [capture commands](../DEVELOPMENT.md#renderdoc-capture-and-replay).

## Boundary

This qualifies the bounded local M3 reference gate, not a complete material renderer.
Two one-primitive instances with w=1 do not prove general perspective/derivative
reconstruction, meshlet decoding, material families, patterned UVs, mip gradients,
transparency, lighting, or performance. Runtime array selection is executed;
sparse descriptor holes, update-after-bind and texture residency are not. M1/M2
scene integration, Windows/Wayland and other vendors remain separate work.
No locked ADR changed. All changes are local; no GitHub push or hosted CI claim.
