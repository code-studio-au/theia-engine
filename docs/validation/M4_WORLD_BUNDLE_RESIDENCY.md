# M4 independent complete-bundle CPU world residency (2026-10-15)

`WorldResidency` adds named chunk transactions over the existing complete-graph
admission and encoded ownership pool. Existing single-bundle publication and
renderer code are unchanged. There are no new dependencies or binary formats.

Native `theia-assets` tests pass **63**, including four new world tests covering
shared allocation identity, different chunks referencing one bundle, independent
replacement/removal, retained whole-world and per-chunk readers, exact/one-byte-
short budgets, foreign staging, duplicate/conflicting/unknown IDs, slot/epoch
exhaustion and 4000 deterministic randomized operations against an independent
union of current/staged/retained resource IDs and lengths. Formatting and
warning-denied all-target Clippy pass.

The existing real-file example accepts `--world <bundle0> <bundle1> ...` in
addition to its preserved two-bundle mode. On the checked four-quadrant fixture:

- Four complete v2 bundles, 24 unique encoded resources, **165148 unique bytes**.
- Separate bundle charges total 372748 bytes; shared owners save 207600 bytes.
- All resources are addressable by chunk, matching admitted IDs/lengths.
- Independent removal and re-addition preserve unchanged chunk epochs.
- An exact-byte pool admits all chunks; a one-byte-short pool rejects the first
  reservation exceeding the independently calculated resource union.
- Removing all chunks retains bytes until the last world reader drops, then
  releases all charged bytes. Rejected staging leaves publication unchanged.

Local logs under ignored `build/qualification/` are `world-residency-tests.log`,
`world-residency-clippy.log` and `world-residency-real-bundles.log`. Inputs are the
four bundle paths in `resident-world-cook/scene.json`. No generated assets are
committed. CPU bytes exclude caller I/O, decoded temporaries, container overhead
and Vulkan allocations; these figures are not VRAM measurements.

Windows GNU cross-link/Clippy passes eleven packages and Wine runs the CPU
test executables, including all 63 asset tests. Evidence is
`windows-world-residency/report.json`; native Windows/GPU is not qualified.

This qualifies CPU ownership only. Camera-driven demand, background multi-chunk
I/O, GPU upload/publication/retirement, simultaneous quadrant rendering and native
Windows GPU execution are still open. The displayed mixed benchmark remains the
single resident whole-world bundle plus GPU visual proxies. M4 is not complete.
