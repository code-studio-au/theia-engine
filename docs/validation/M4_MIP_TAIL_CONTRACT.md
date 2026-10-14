# M4 coarse resident mip-tail CPU contract (2026-10-14)

`texture_residency::MipTail::fit` borrows an admitted immutable complete authored
chain and chooses the finest contiguous suffix whose dense texel bytes fit a
caller budget. Every suffix includes the protected 1x1 level. Budgets below four
bytes reject; an exact threshold admits that level, one byte below coarsens again.
The source container, content identity and texels remain unchanged. No allocation,
partial file read, publication, feedback, sparse binding or Vulkan adoption is
introduced by this unit.

Original dimensions/count and each public mip's dimensions, exact byte length and
opaque alpha are rechecked before shifts/indexing. The caller still owns original
encoded hash/provenance admission. `source_extent` is deliberately separate from
`resident_extent`: footprint demand must use original dimensions. `select` rejects
requested indices outside the original complete chain, otherwise chooses
`max(requested, first_resident)`, returns an image-relative index and marks whether
residency forced coarsening. There are no arbitrary holes or finer substitutions.
This coarse-resident fallback is separate from unsupported-gradient fallback,
which requests source mip zero before applying residency constraints.

The budget counts only dense source texels in the suffix. It excludes encoded
bytes retained on the CPU, staging alignment, actual Vulkan memory requirements,
descriptors and unrelated resources. It must not be advertised as a physical VRAM
cap; a subsequent GPU image must independently check actual allocation charges.
The example retains/admit-checks the whole encoded file even when planning a tail.

Four tests check exact/short square-chain boundaries, protected tail, borrowed
pointer identity and mapping; exhaustive budgets for all 81 dimension pairs from
1..9 against an independent suffix search; maximum horizontal/vertical thin chains
and original-dimension footprint selection; forged public-chain rejection.
All 58 assets tests pass, as do formatting and denied-warning all-target Clippy.
The readonly content-addressed `texture_tail <file> <texel-budget> <requested-mip>`
example requires complete layout/hash/provenance admission. It executes 156 plan/
selection/rejection checks on the native cooked square, odd and 11-level thin files.
For the thin chain an exact 508-byte dense budget keeps source levels 4..10;
507 bytes keeps 5..10. These are CPU plans, not GPU allocations or rendering.

Logs/report: `build/qualification/m4-mip-tail/report.json`,
`m4-mip-tail-{tests,clippy,build,renderer-tests}.log`. No new GPU, Windows,
performance or milestone-completion claim. Next qualify an explicitly bounded
conventional image containing only the selected suffix, with original source
dimensions/mip offset supplied to the GPU and actual physical caps enforced.
Full tiled virtual textures, feedback/I/O demand and asynchronous tail replacement
remain separate gates. No locked ADR changes.
