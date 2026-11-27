# M4 independent complete-chunk replacement (2026-10-15)

The two-complete-bundle profile keeps chunk 0 fixed while a single bounded local
I/O worker and one pending GPU set alternate chunk 1's two immutable graphs.
Complete CPU admission precedes reservation; all transfer fences and prior frame/
presentation users retire before replacing GPU owners and rebuilding descriptors.
The CPU world publishes only chunk 1 after the complete new generation exists.
This is a diagnostic schedule every twenty presented frames, not camera demand.

The source variant uses the existing palette fixture cooker with `--images 2`.
Four source images intentionally alias two distinct PNG payloads, so normalized
material-image bindings become `[0,1,0,1]`. Geometry, UVs, triangle materials and
unlit factors remain unchanged. The Rust importer, native meshoptimizer/KTX cook,
exact expected-pixel checks and strict runtime admission are preserved. The replay
oracle now follows the texture table's content hashes/material mapping rather
than assuming one image per material. No shader or binary format changes.

`world-stream-final-v2/report.json` passes **15 native plans** with actual validation:

- Baseline, exact coexistence caps and shared profile each publish five times.
- Wayland recovery publishes five times while exercising the WSI injections.
- One-byte-short CPU, physical and image coexistence caps each defer five times,
  retain both complete initial chunks and keep the world epoch at 1.
- A CPU cap fitting only initial residency also defers without publication.
- Shutdown with a read outstanding and with an admitted GPU pending set retires
  their ownership cleanly. The latter legitimately retains three CPU graphs.
- 1320 frames publish **65** times while fixed chunk 0 remains at epoch 1.
  History retains exactly 64 events and reports two older events dropped.
- A private replacement-manifest copy, corrupted after startup, fails immutable
  identity admission before any publication, with clean GPU teardown. Original
  verified roots/manifests remain untouched.
- Missing pair, orphan report and identical replacement inputs reject pre-Vulkan.

Native runtime cases have zero Vulkan warnings/errors through teardown. The
changed-source case is intentionally fatal; no source-error rollback claim is made.

The tested RADV coexistence observations are **175112 unique encoded CPU bytes**,
**234672 bundle physical bytes**, and **163840 image physical bytes**. Exact caps
permit both publication directions; one-byte-short caps protect the initial sets.
Bundle/image charges exclude staging, targets, work arenas, descriptors, samplers,
queries, host containers and driver overhead. They are not total VRAM. Peak
complete-set fields omit failed partial candidates; individual allocation checks
still apply throughout those failures. CPU live charges include pending reservations.

Seven RenderDoc 1.46 replays pass **903 ray/colour probes** across initial,
replacement, return-to-original, repeated replacement, shared profile and CPU/GPU
pressure-retained content. Each capture verifies both complete root sets, all
active raw image payloads (eight or six), both GPU compactions, whole-image
identity/primitive/depth bounds and sampled material/texture colours. All four
materials in both namespaces are exercised. A bounded CPU history selects the
actual published graph at the captured frame; no assumed worker deadline is used.
The existing depth 0.00002, encoded RGB two-byte and nearest-cell 0.002-texel
boundary tolerances remain unchanged. Replay validation is disabled; standalone
native validation is enabled.

Renderer tests pass 28; formatting and warning-denied Clippy pass. Windows GNU
cross-link/Clippy passes eleven packages and Wine CPU tests pass
(`windows-world-stream/report.json`). Native Windows GPU remains unqualified.

Evidence under ignored `build/qualification/`: `world-stream-final-v2/`,
`world-stream-{build,clippy,tests}.log`, `world-stream-variant/` and
`world-stream-preserved-regressions/`. The latter reruns the isolated four-chunk
ownership probe plus existing cooked replacement and 10000-proxy mixed routes
after extracting pollable complete-upload ownership. Generated assets/captures
stay local.

Limits: two resident complete chunks, one replacement slot, no spatial-region
inference, protected coarse-world coverage, automatic camera demand or crowd-route
composition. Reads have byte caps but no I/O deadline; shutdown joins the worker.
Generation construction failure is fatal without rollback. Device/presentation
retirement remains conservative, so this does not prove production upload/render
overlap, allocation reuse or streaming latency. M4 remains open.


The optimized executable separately completes 120 frames under the exact three
coexistence caps, with five independent publications, fixed epoch 1 and zero
validation diagnostics. Evidence: `world-stream-release/` and
`world-stream-release-build.log`. The shader sources remain the previously
qualified 45-module default/shared profiles; this unit changes upload ownership
and publication, not shader code.
