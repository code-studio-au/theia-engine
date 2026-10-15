# Protected CPU fallback coverage (2026-10-15)

`world_coverage::ProtectedWorld` encapsulates the existing complete-bundle CPU
world after fallback bootstrap. A bounded catalog pairs each fine chunk ID and
its horizontal demand bounds with one distinct coarse chunk ID. All pairs' IDs
must be unique, every fallback must already be published, and unrelated resident
IDs are rejected. Construction admits 1..2048 regions, within the existing 4096
chunk bound. Asset-pair geometric coverage is a caller qualification requirement;
this API does not infer it from bounds or filenames.

The wrapper exposes staging, immutable snapshots and fine-only atomic publication.
It cannot replace/remove protected fallback IDs. Unknown IDs, foreign tokens,
capacity errors and underlying admission/budget failures preserve the current
selection. Staged reservations consume encoded bytes but never select fine data.
Each retained coverage snapshot owns the complete world snapshot and an ascending
region selection: exactly one committed fine chunk if present, otherwise its
protected coarse chunk. Its owners and selections come from the same epoch.

Distance demand considers only actual committed fine IDs. Coarse chunks do not
consume the demand policy's fine slots or appear in its removals. World slot
capacity, shared encoded bytes, pending reservations and retained readers still
apply independently. A caller can remove an out-of-demand fine chunk while a new
one is deferred, returning that region to coarse coverage. This is an experimental
bounded CPU policy, not a new locked streaming/LOD ADR.

Six new tests bring the asset crate to **73 passing tests**, including 2000
randomized publication attempts with retained epoch selections. They cover
bootstrap rejection, missing/aliased/unknown IDs, invalid bounds, pending cancel,
protected replacement/removal rejection, foreign staging, atomic slot failure,
coarse-only byte capacity, fine demand hysteresis and retained-reader lifetime.
Formatting and warning-denied Clippy pass, including the real-bundle example.

`bundle_residency --covered-route` bootstraps four real coarse graphs, reserves
fine candidates before removing old users, and records demand, published/rejected/
cancelled loads, actual IDs, selected IDs, world epochs and charged bytes. The
independent Python oracle uses closest-point distances and unions of real encoded
resource hashes. Five **687-sample routes** include 240 benchmark camera samples,
spatial excursions, exact central boundaries and a final far-away return to coarse.

| Case | Encoded cap | Transactions | Rejected loads | Cancelled reservations |
|---|---:|---:|---:|---:|
| Complete eight-graph capacity | 237988 | 21 | 0 | 0 |
| One-fine coexistence pressure | 166164 | 24 | 9 | 0 |
| Coarse-only capacity | 142040 | 0 | 1131 | 0 |
| One byte below all graphs | 237987 | 21 | 170 | 0 |
| Simulated pending cancellation every third sample | 237988 | 18 | 0 | 13 |

Every sample's full output matches the oracle. All four protected IDs remain
published and every selected ID names a complete graph in the same snapshot.
The coarse-only run never publishes a fine graph. These byte charges exclude
caller-owned I/O buffers, decoded temporaries, map/Arc overhead and GPU memory.
Cancellation is simulated after synchronous CPU reservation; it is not a tested
background I/O cancellation deadline.

Local ignored evidence: `world-coverage-final/report.json`, its five JSONL traces,
`world-coverage-tests.log`, `world-coverage-clippy.log` and
`world-coverage-example-build.log`. This unit has no GPU publication, descriptors,
worker scheduling or automatic runtime LOD. Existing renderer paths remain separate.
Next retain matching protected GPU owners, retire transfer/graphics users and
publish complete descriptor generations before camera demand changes drawing.

Windows GNU cross-link and denied-warning Clippy pass eleven packages; Wine CPU
tests pass (`windows-world-coverage/report.json`). Native Windows/GPU execution
remains unqualified. Renderer denied-warning Clippy and all 28 CPU tests also pass
with the new library module linked. No new GPU execution claim belongs to this
CPU-only slice.
