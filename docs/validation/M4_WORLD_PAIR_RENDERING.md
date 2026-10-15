# M4 two resident geometry chunks with shared visibility/depth (2026-10-15)

This isolated Render Lab slice draws two admitted THEIAMES geometry pages together.
It is separate from the complete-bundle upload ownership probe and the textured
mixed benchmark. It does not yet connect camera demand to GPU publication.

`--world-pair-test <mesh0> <mesh1>` uploads two independently owned geometry/work
sets. Each compute pass culls at most 32 meshlets into its own 256-byte work arena;
two fixed indirect task/mesh draws write one R32G32_UINT identity image and one
D32 depth target. Resolve binds both geometry pages and validates each local
meshlet/primitive before emitting a diagnostic chunk tint with alternating face
brightness. No conventional vertex buffer, per-meshlet CPU draw or count readback
is introduced. The rig has empty, angled and full views, selected by
`--work-items 0|1|2`; `--world-pair-reference <width height view>` exports CPU rows.

Identity bit 30 selects the second input; remaining low bits retain meshlet+1,
with zero reserved for background. This is a frame-local diagnostic namespace,
not a persistent chunk/entity handle, asset-format change or combined proxy mode.
The fixed mode rejects bundle streaming, benchmark modes, authored materials/UVs,
textures and LOD inputs. The separate mixed benchmark's bit-31 proxy namespace
and resident textured representation remain unchanged.

The replay oracle compares both immutable GPU geometry buffers and both culling
compactions, scans the entire image for valid chunk/local identities, primitives
and depth, and independently ray-tests both pages for the nearer intersection.
Sampled encoded RGB permits two bytes and depth permits 0.00002. Reversed input
order tests that the same spatial chunk works under either frame-local tag.

Limits: resident geometry only, diagnostic tint, two input chunks, one frame in
flight, no complete-bundle ownership integration, world-demand execution,
protected fallback, texture resolve, proxy composition or streaming latency proof.
M4 remains open. Generated captures/logs remain local under ignored
`build/qualification/`.

`world-pair-final-v2/report.json` passes seven native cases: a 120-frame Wayland
WSI recovery route (seven swapchain generations), a second spatial pair, missing
second geometry/shader cleanup, the preserved single-page mode without optional
world shaders, pre-Vulkan benchmark conflict and a native shared-interface pair.
Runtime cases have zero Vulkan warnings/errors through teardown. Five RenderDoc
1.46 captures pass **577 ray/colour probes**, whole-image identity/depth bounds,
both immutable GPU pages and both compactions: full pair, reversed order, angled
right pair, empty view and shared-primary/per-vertex-secondary profile. Across
these probes chunk tags 0/1 win **11/3** two-hit occlusion tests respectively.
Capture validation is disabled; standalone native validation is enabled.

Both default and shared compiler profiles compile/validate **44 SPIR-V modules**.
The second chunk uses its own per-vertex fragment module, avoiding dependence on
the optional shared-primary fragment interface. The existing specifically
qualified GLSL shared-primary fallback stays confined to its previous module.
Renderer tests pass **28**, formatting and warning-denied Clippy pass. Native
workload-isolation checks additionally reject authored-material and camera flags
in either argument order before Vulkan (`world-pair-cli-checks/`).

Logs: `world-pair-{build,clippy,tests,shaders,shared-shaders}.log`,
`world-pair-final-v2/` and `world-pair-preserved-regressions/` under ignored
`build/qualification/`. The last separately reruns the four-chunk ownership probe,
its failure paths, 120-frame streamed cooked publication and the 10000-proxy mixed
route after integration. These are correctness observations, not performance
scores or evidence of asynchronous world streaming.


Windows GNU cross-link/Clippy passes eleven packages and Wine executes the CPU
suite on the final CLI guard revision. Evidence:
`windows-world-pair-cli-final/report.json`. Native Windows GPU remains open.
