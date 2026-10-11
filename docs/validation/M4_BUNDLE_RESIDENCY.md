# M4 complete-bundle CPU residency evidence (2026-10-11)

This is CPU ownership qualification; the existing fixed renderer and geometry
streaming path are unchanged. No asynchronous GPU set publication is claimed.

The pure Rust store semantically admits complete THEIABND v1/v2 sets, including
exact table leaves, before reserving the whole unique encoded-byte set. Stages,
current snapshots and retained readers share content owners. Manifest bytes are
included; I/O buffers, decoded temporaries, allocator/map overhead and VRAM are
excluded. Publication changes the whole CPU snapshot and checked epoch at once.

All 33 asset tests pass. Five new tests cover profile-independent sharing and
stage cancellation, missing/corrupt leaves, freshly hashed foreign UV binding,
wrong manifest digest, unexpected v1 leaves, one-byte-short budget rejection,
retained-reader lifetime, foreign store tokens and epoch exhaustion. A 3,000-step
randomized test compares pending stages/current/readers to an independent union
of resource IDs and encoded sizes after every operation. The previous geometry
and arena oracles remain in the test suite. Formatting and denied-warning
Clippy for all asset targets pass; renderer/importer regression tests pass.

The example loads the actual preceding two-image and alias bundles (identities
in [texture evidence](M4_TEXTURES.md)). Individual charges are 6528 and 5416 bytes;
the union is 11012, sharing the 932-byte primary KTX2. Snapshots contain seven
and six resources. A 6528-byte budget rejects staging the alias while the first
set is current or retained, preserving the current epoch. Clearing current and
releasing the last reader permits the alias. The exact union budget admits both
and releases old resources only after readers drop.

A second execution uses the actual v2 two-image bundle and preceding v1 shared
bundle: 6528 and 4912 bytes, union 10508, sharing the same 932-byte texture.
Snapshots contain seven/five resources. The same budget/retention checks pass.

Artifacts under ignored `build/qualification/`:

- `m4-bundle-residency-tests.log` (33 tests).
- `m4-bundle-residency-real.log` (v2/v2 real files).
- `m4-bundle-residency-profiles.log` (v2/v1 real files).
- `m4-bundle-residency-{renderer,import}-tests.log` (14/12 tests).

No hosted CI, Windows runtime, GPU publication or performance measurement was
performed for this unit. Next qualify complete uploads before frame-safe GPU
publication and retirement under separately specified allocation budgets.
