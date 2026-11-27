# Native Bevy port of the current visual benchmark — 2026-10-16

An isolated optional Bevy 0.19.1 executable now renders Theia's current cooked
village/terrain/proxy workload. It imports actual triangles, UVs, texels and
linear factors, shares readonly camera/motion reference functions, computes
selected-source ground heights on CPU, and matches proxy detail/face shading/fog
through a bindless-capable unlit material. Bevy's own renderer remains responsible
for extraction, batching, culling and Vulkan rendering. Both overlays are disabled.
No existing engine source/manifests/shaders change; Bevy is not a runtime dependency.

This is comparison tooling, not adoption of Bevy as Theia's engine. It changes
no locked ADR.

## Executed evidence

Ignored paths below are relative to `build/qualification/`.

- `bevy-comparison-final/comparison.json`: three 180-frame population pairs at
  0, 1000 and 10000 proxies, 1280x720. Theia uses actual Vulkan validation; Bevy
  requests wgpu validation and enables Vulkan's validation layer. Corrected runs
  have no Vulkan validation/error diagnostics. Bevy emits a benign compositor
  decoration warning. All frame counts, clocks, selections, device/backend/extent
  and source-report hashes match the declared comparison contract.
- Twelve frozen Bevy captures at frames 0/60/120/179 pass **1462 source ray/colour
  probes**, with maximum **one RGB byte** error against independently intersected
  world/proxy triangles and source textures. Exact texture-boundary probes are
  separately counted/skipped; ordinary colour tolerance remains two bytes.
- `*/pose-proof.json` in that directory checks 7000 and 70000 proxy poses across
  seven sampled frames for the 1000/10000 populations. XY motion exactly matches
  the shared represented CPU reference; independently queried source-ground error
  stays below **1.71e-7 m**. Both fine/coarse terrain and near/far detail are covered.
- `bevy-comparison-timing/comparison.json`: a separate validation-disabled,
  660-frame 10000-proxy pair with 60 warmup frames and a repeating 600-frame route.
  Both record 600 measured samples. All clocks/selections, 70000 pose checks,
  finite metrics, CSV row counts and independent summary arithmetic pass.
  Host-interval means are 24.4271 ms (Theia) / 25.0835 ms (Bevy), in this owned
  Weston Wayland/FIFO context. These are not uncapped capacity measurements or
  evidence that one engine is faster. The extra Bevy pre-roll is 120 frames.
- `bevy-comparison-{build,clippy,tests}.log`: release build, denied-warning Clippy
  and nine shared camera/workload CPU tests pass. Rust formatting and Python 3.8
  syntax checks pass; generated HTML JavaScript passes Node syntax checks.
- `bevy-comparison-final/negative/report.json`: mismatched manifest, enabled
  source overlay and changed actual extent all fail before fixture publication.
- `bevy-comparison-final/comparison.html` and `bevy-comparison-timing/comparison.html`
  provide population-selected paired host-frame traces with explicit scope labels.

The first native attempts (`bevy-comparison-native/`, `bevy-x11-snapshot/`) produced
startup PRESENT-layout/acquire-semaphore validation errors, despite passing pixel
checks. They are superseded by the corrected final gate. The reference now clears
the initial acquired surface before Bevy can present an unrendered image. The
one-time clear precedes render-system execution and is outside recorded samples.
No Bevy/wgpu registry sources are modified. Native X11 after this correction and
native Windows GPU execution remain unqualified.

## What this establishes

Theia's scene is reproducible in a native Bevy renderer, using the actual cooked
content and observed logical view/selection sequence. This is a useful controlled
visual baseline. Bevy preloads all eight graphs and uses CPU ECS proxy updates;
Theia streams and uses GPU presentation work. The source selection trace is replayed,
not independently produced by a second equivalent streaming implementation.

FIFO and one requested Bevy frame of latency constrain host-time interpretation.
Pipelined Bevy rendering is disabled for this baseline, with synchronous pipeline
compilation and explicit pre-roll. Bevy GPU timings are absent. The two recorded
CPU metrics cover different portions of their loops. This does not compare total
VRAM, streaming latency/throughput, production gameplay, animation, lighting, MMO
server scale or maximum renderer throughput. Common uncapped presentation and
comparable GPU timestamp scopes remain separate gates before a performance ranking.

[Build/run instructions](../../tools/theia-bevy-benchmark/README.md).
