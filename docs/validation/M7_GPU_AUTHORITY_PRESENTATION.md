# M7 fixed GPU authority presentation — 2026-10-11

Separate `--authority-test` uses existing Render Lab WSI/Vulkan ownership. No
existing headless test or asset/visibility/indirect shader is repurposed. Gameplay
is server-owned; GPU interpolation and generated squares are presentation only.

## Executed local evidence

RX 7900 XTX/RADV, Slang 2026.19, Linux/X11/Rust 1.99. Logs ignored under
`build/qualification/m7-gpu-authority-*`; captures under `m7-authority-capture-*`.

- Server fixture executes two movement ticks and one range-legal hit. Previous/
  current actor position [-850,-1000]/[-700,-1000] mm; alpha .5 yields [-775,-1000].
  Target health is 75. Logical IDs `(2<<32)+10/20` retain their high words.
- Default and shared shader-directory authority modes: 120 frames, ten swapchain
  generations, resize/minimize/restore, zero-size deferral, labeled injected WSI
  recovery and zero warnings/errors, including teardown. No new swapchain policy.
- RenderDoc 1.46 captures/replays alpha 0/.5/1. One direct three-group mesh draw,
  no compute dispatch. CPU raster interior/exterior/colour comparisons pass
  32371/32371/32387 sampled pixels respectively. Actor/obstacle/background samples:
  [112,120,15,32124], [112,120,15,32124], [120,120,15,32132]. A one-pixel boundary
  band is excluded; no hardware top-left/edge-rounding equivalence claim.
- Palette depends on XOR of low/high logical-ID words; omitting the nonzero high
  word changes the expected colour, so replay verifies that it contributes.
  Target colour scales by current health .75, independently of position alpha.
- Sixteen legacy 120-frame/WSI modes pass zero diagnostics: clear, mesh, task,
  indirect, spatial, visibility, asset, camera, cull, resolve, factors, UV, v1/v2
  bundle, bundle stream and resident terrain LOD. Shared authority also passes.
- Headless compute still verifies all 256 values (1..511 odd numbers).
- Renderer tests: 15. Formatting and denied-warning all-target Clippy pass. Both
  default/shared compiler paths compile/SPIR-V validate 31 outputs. Existing
  compiler profile-upgrade warnings remain; no warning-free compiler claim.
- Invalid alpha/wrong mode/conflicting mode/work-item options fail before Vulkan.
  Missing authority mesh shader fails with clean teardown/zero diagnostics.
- Windows/hosted CI remain unexecuted; no cross-vendor success claim.

## Fixed contract and limitations

64-byte mesh push: previous float4 XY, current float4 XY, uint4 low/high ID pairs,
uint2 health, float alpha, uint blend mask. One mesh draw launches three groups:
two actors (250 mm half extent) and one immutable obstacle (100 mm half extent).
Slang emits four vertices/two triangles each; no conventional vertex/index buffer.
Device checks cover 64-byte push, output/count/component/memory and group limits;
mandatory task/mesh device feature checks remain intact despite this mesh-only
pipeline. Vulkan native-byte push is not the network wire schema.

Positions originate in completed authority ticks and pass canonical replica
admission/CPU presentation. GPU owns interpolation, quad generation and colours;
no combat/movement legality is derived from GPU state. Fixed XY projection divides
mm by 4000, with existing clear background. There is no perspective/depth actor
occlusion, animation, UI, resident scalable actor store or performance claim.
Two actor states fit one push; this is not a proposed production storage design.

Alpha is fixed per launch; no keyboard controls, real-time authority loop or live
Quinn feed yet. Captures disable validation and are independent of standalone
synchronization validation. Next add bounded interactive local authority controls,
then live network presentation while preserving prior GPU paths.
