# Batched GPU selected-terrain heights (2026-10-15)

`--world-ground-gpu-test <scene.json points.json report.json>` is an isolated
compute/readback qualification followed by three clear presentation frames. It
builds the CPU-qualified terrain cell index from immutable cooked inputs, encodes
all eight fine/coarse surfaces in one readonly GPU buffer, and submits all 16
selection masks over the input points in one batched Slang compute dispatch.
There are no per-proxy host calls, draws or allocations in the dispatched query.
This profile does not render proxies or alter the existing benchmark.

The runtime-generated little-endian table uses eight 32-byte headers, three
float4 vertices per triangle, cell CSR offsets and triangle references. Queries
are 16-byte XY/mask records; results are 16-byte height/region/status/candidate
records. Each invocation chooses the region and selected fine/coarse surface,
reads that cell's candidates and interpolates actual cooked vertex heights.
Missing or conflicting covering triangles return failure status. Host verification
checks every height, region, status and candidate bound. The shader bounds its
last partial workgroup from the query-buffer length.

All compute objects have a shared RAII submission guard with the existing
nonuniform descriptor probe. Submitted work retires before pipelines, layouts,
descriptor pools and referenced buffers release, including failure paths. The
original descriptor probe still checks all 64 GPU values after this guard move.
The ground query uses three ordinary storage bindings and does not require
optional storage-array features. It adds no bundle schema or baseline requirement.

## Native evidence

`world-ground-gpu-final/report.json` passes **10 cases**: full/default and shared
shader profiles, empty/outside queries, missing scene, missing shader after buffer
allocation, the preserved descriptor probe and three pre-Vulkan mode conflicts.
Native runtime cases activate validation and finish with **zero warnings/errors**,
including failed construction. Failed queries leave no completed GPU report.
The existing source-triangle CPU oracle is rerun first and qualifies the same
35605 points and 569680 selected heights before GPU comparisons.

Both profiles verify **569680 GPU heights** over all 16 masks, including 30000
proxy positions at three route frames and exact/near terrain seams. Maximum
CPU/GPU difference is **2.39e-7 m**; adding the independent CPU/source error of
1.26e-7 m bounds GPU/source difference below **3.65e-7 m**. These are interpolation
correctness results, not performance measurements or a continuous error proof.

The table is **40704 bytes**. Queries and readback each use **9114880 bytes**;
RADV reports **18270464 total physical buffer bytes**. That total excludes
transient upload staging, clear targets, descriptors and driver overhead. Inputs
are capped at 50000 points (800000 selected queries), each buffer at 16 MiB, and
storage-range/dispatch limits are checked. The table/reference CPU structures are
not charged to the separate camera world's existing resident graph ledger.

Both default and shared profiles compile/validate **49 SPIR-V modules**, including
the new Slang compute entry point. No shader-language fallback or tolerance
expansion was needed. Renderer formatting, denied-warning Clippy and 30 CPU tests
pass. Ignored artifacts live under `build/qualification/world-ground-gpu-*`.

The optimized executable independently passes all ten cases with the same height
error and allocation ledger (`world-ground-gpu-release/report.json`). Eleven
Windows GNU packages cross-link/pass denied-warning Clippy; CPU suites pass under
Wine (`windows-world-ground-gpu/report.json`). These do not qualify native Windows
GPU execution.

## Remaining gates

This batch readback fixture retains all eight surfaces and uses host-generated
queries/expectations for qualification. The real crowd must instead generate its
positions and selected-mask query on the GPU, avoid readback in the frame path,
and explicitly account for its persistent query table. Actual mixed terrain/crowd
visibility, material resolve, HUD metadata and timed route qualification follow.

This single-layer benchmark heightfield is presentation-only. It does not provide
server collision, caves/roofs, full geometry LOD, texture streaming or gameplay
movement authority. Native Windows GPU execution remains unqualified.
