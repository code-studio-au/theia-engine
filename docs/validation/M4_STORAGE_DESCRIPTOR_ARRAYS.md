# M4 optional nonuniform storage-buffer array compute gate (2026-10-15)

This Render Lab probe is a prerequisite to evaluating bounded multi-chunk binding.
It does not draw world chunks or change the production GPU baseline. The original
headless compute app remains separate and unchanged.

`--world-descriptor-test` queries the optional core uniform storage-buffer array
indexing feature and Vulkan 1.2 nonuniform storage-buffer array indexing feature,
checks at least five storage descriptors plus 64-thread compute limits, then
requests those flags only for this test's device. Existing runtime-descriptor-array
support is retained. Other modes keep the prior requested feature set.

The Slang kernel declares an unsized read-only storage buffer descriptor array and
one read/write output. One 64-thread group selects descriptor `lane % 4` and varies
the byte offset within each 16-byte source. Four distinct literal source sequences
produce a fixed transposed 16-word reference pattern repeated four times. All
**64 GPU values** match after sentinel initialization, explicit transfer-to-compute
read/write and shader-write-to-host-read barriers, fence retirement and coherent
readback. The output buffer declares read/write access, so the initialization
barrier covers both. Probe RAII retires commands before destroying dependent
pipelines, layouts, pools and buffers, including partial-construction failure.

`world-descriptor-final/report.json` passes **six plans**: four Vulkan contexts use actual
validation and pass zero warnings/errors through teardown, plus two pre-Vulkan
mode conflicts. Cases cover: default/shared modules,
missing-probe-shader cleanup, normal geometry rendering without the optional
module, and two pre-Vulkan conflicting-mode rejections. Successful runs present
three clear/geometry frames. SPIR-V disassembly verifies `ShaderNonUniform`,
`RuntimeDescriptorArray`, `StorageBufferArrayNonUniformIndexing` and actual
`NonUniform` decorations. No new shader-language fallback is introduced.

Default/shared profiles each compile and validate **46 modules**. Renderer tests
pass 28, formatting and warning-denied Clippy pass. Windows GNU cross-link/Clippy
passes eleven packages and Wine CPU tests pass
(`windows-world-descriptor/report.json`); native Windows GPU is unqualified.

The local RX 7900 XTX/RADV driver reports 8388606 per-stage/set storage-buffer
limits. Only four source descriptors plus one output are exercised; the reported
limit is not a capacity/performance result. Requested Vulkan buffer allocations
total 320 bytes, including the host-visible output, excluding staging, descriptors,
commands, surface/targets and driver overhead. This is not total VRAM usage.

Evidence under ignored `build/qualification/`: `world-descriptor-final/`,
`world-descriptor-{build,clippy,tests,shaders,shared-shaders}.log` and
`world-descriptor-preserved-regressions/`. The latter reruns existing complete
upload/failure ownership, cooked replacement and 10000-proxy mixed paths with the
new optional flags inactive. Generated binaries and logs remain local.

Scope: compute descriptor selection only, one workgroup, tiny buffers. Fragment/
mesh resource arrays, four-chunk composition, allocation scale/performance and
cross-vendor/native Windows execution remain open. This records qualification,
not approval of a production binding policy or a locked ADR change. M4 stays open.


The optimized executable also passes all six plans (`world-descriptor-release/`)
and the release build log is `world-descriptor-release-build.log`.
