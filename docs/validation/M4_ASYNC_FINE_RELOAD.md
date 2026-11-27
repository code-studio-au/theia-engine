# One asynchronous fine reload over protected coarse coverage (2026-10-15)

`--world-covered-test <scene.json> --world-covered-reload-test` includes the finite
fine-retirement sequence, then requests fine region 2 after 100 presented frames.
One named local-file worker and a capacity-one channel return one bounded immutable
complete input. GPU submission stays on the renderer thread. A complete CPU
reservation owns all encoded sources while a pending four-buffer/image upload
set owns every submitted copy. Transfer fences must retire before activation.

The qualification holds uploads until at least presented frame 110 before allowing
publication. Coarse drawing remains active during file reading and pending GPU
work. The renderer retires the old frame/presentation/descriptor users, installs
the ready fine resources while retaining the original coarse GPU set, constructs
a complete new descriptor generation, then publishes the matching fine CPU chunk.
No coarse resource reupload occurs. Region 2 exercises the varied fine fixture's
two-image/material-factor profile returning after four-image coarse selection.

CPU capacity failure cancels that reservation; physical/image capacity failure
retires partial copies before reservation release. Both leave complete coarse
selection active. Other errors, including immutable-source change and target
construction failure, remain fatal. Worker shutdown joins; reads are byte-bounded,
not governed by an I/O/cancellation latency deadline. This is one fixed request,
not repeated scheduling, camera demand or production missing-content recovery.

## Observed ownership states

The base full sequence starts at eight graphs/CPU epoch 5, retires to four coarse
graphs/epoch 9, then publishes region 2 at epoch 10. Protected coarse epochs remain
`[1,1,1,1]`. The final selection is `[4,5,2,7]`, charging **165852 unique CPU bytes**,
**423552 physical graph bytes** and **327680 image bytes**. The varied final
selection charges **200144 CPU**, **390480 physical** and **294912 image bytes**,
with 20 root buffers/18 images (including retained coarse), versus 20/20 for base.

A 110-frame prefix ends with coarse selected and epoch 9, while a **ready pending**
fine set remains unpublished: 142040 bytes of published coarse graphs plus the
fine reservation charge totals 165852 CPU bytes. Active coarse physical/image
charges stay 334368/262144; separate pending charges are 89184/65536. The report
marks `pending_ready=true`, `published=false`. Teardown releases submitted GPU
work before its CPU reservation with zero diagnostics. These counters distinguish
published ownership from pending allocations; they are not total VRAM and exclude
work arenas, targets, staging, descriptors and driver/container overhead.

An exact coexistence cap of **165852 CPU / 423552 physical / 327680 image bytes**
selects only fine region 2 at startup, retires it, then reloads it successfully.
One-byte-short CPU, physical and image caps each defer the later request, preserving
coarse coverage. Coarse-only capacity also survives. History contains at most six
selection rows: initial + four retirements + one reload. Pending metadata is
separate from committed selection history. Normal short runs may complete without
completing the finite schedule; the history/status records actual progress.

## Qualification

`world-reload-final/report.json` passes **16 native cases** with zero validation
diagnostics and **seven replays with 1641 ray/colour probes**. Native cases cover
full, WSI/shared/varied, exact/short/coarse-only caps, shutdown before request,
shutdown with the worker active, shutdown with a ready pending set, immutable
source failure, failed target construction and pre-Vulkan mode conflicts. Combined
WSI/retirement/reload recovery presents 120 frames over 12 swapchain generations.
The two controlled failures modify only private copied inputs/owned shader links
and leave reports incomplete without a false fine CPU publication.

Every committed row matches independent real-resource hash unions, selection,
world/protected epochs and ownership/image charges. Prefix checks include pending
CPU and GPU coexistence charges. Replays verify the actual committed history,
selected payloads, all live allocation inventory, four compactions and nearest-hit
identity/depth/material-factor/sRGB colours. Short-cap captures remain coarse; a
retirement-only regression still passes. Captures disable validation; native plans
supply actual validation evidence. No shader changes or expanded tolerances occur.

Local ignored evidence includes `world-reload-final/`, `world-reload-first/`,
`world-reload-{build,clippy,tests,release-build}.log` and the platform closeout.
Next generalize this bounded operation to camera-demanded region IDs and repeated
fine/coarse transitions, then integrate the actual world path with the timed crowd
benchmark. General world streaming, geometry LOD and virtual textures remain open.

Renderer formatting, denied-warning Clippy and 28 CPU tests pass. Windows GNU
cross-link/Clippy covers eleven packages and Wine CPU suites pass
(`windows-world-reload/report.json`); native Windows GPU remains unqualified.
The optimized executable passes full reload, ready-pending prefix shutdown,
exact coexistence and one-byte-short GPU deferral (`world-reload-closeout/report.json`).
The six preserved upload/cooked-stream/10000-proxy mixed plans pass with zero
native diagnostics (`world-reload-preserved-regressions/report.json`). The prior
48-module default/shared shader profiles are unchanged.
