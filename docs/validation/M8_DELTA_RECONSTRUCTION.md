# M8 — Bounded transport-neutral delta reconstruction (2026-10-11)

`theia-protocol::delta::Delta` names an exact complete baseline and target tick.
Sorted full-actor upserts/removals reconstruct a complete set before readonly
publication. Missing baselines preserve current and previous state; a caller's
explicit full snapshot repairs history. No socket, wire schema, baseline ACK/cache
producer policy or network repair exchange is introduced. THN2 still sends full sets.

## Executed evidence

Linux/Rust 1.99, ignored `build/qualification/m8-delta-*` logs.

- Protocol tests 20 pass. Four additions cover complete lifecycle replacement,
  health/observed respawn/empty sets; exact current/previous baselines, duplicate,
  eviction/missing/full repair and source reset; malformed/conflicting/regressing/
  over-capacity deltas preserve both endpoints; 3000 independent BTreeMap sparse
  membership/value round trips including near-u64-max ticks.
- Synthetic example generates 10000 packets with controlled application-message
  drop/hold/overtake/duplication and old baseline selection. Every publication is
  compared to independent complete-state actor values. Counts: 4259 new, 204
  duplicate, 1714 stale, 2922 missing-baseline full repairs, 1428 dropped, 1714
  reordered. Final explicit full repair verifies tick 10001; source reset rejects
  the old source and does not import the old baseline. These counts are neither
  QUIC packets nor runtime latency/throughput results.
- Renderer tests 22 pass; encrypted headless packet-fault regression preserves
  two hits/health [75,75], reliable sequencing/repair and readonly interpolation.
  Formatting and denied-warning all-target protocol Clippy pass. No GPU/shader
  changes; target-GPU evidence remains the preceding viewer/fault qualifications.
- Linux/Windows CI definitions now execute the pure delta-repair example. Hosted
  CI and Windows are unexecuted locally.

## Bounded contract

Caller supplies ordered unique IDs, positions within +/-1M mm and health <=1M;
these remain prototype values. Limit 1..128 actors. Each upsert/removal list is at
most that limit; removals must exist in the baseline and never overlap upserts.
A reconstructed union exceeding the actor cap rejects. Target tick strictly
exceeds baseline tick; watermarks cannot regress relative to baseline or current
publication. Counter comparison uses integers even near u64 exhaustion.

Replica first checks locally owned connection source. It accepts exact baselines
from only its two retained complete endpoints; no additional baseline cache or
unbounded pending-delta queue. Full reconstruction precedes atomic publication;
retained vectors normalize spare allocation capacity as before. Unknown/evicted
baseline leaves both endpoints intact. Stale envelopes need not reconstruct; a
same-tick delta checks complete equality only when its baseline exists, otherwise
requires full repair. This conservative policy favors explicit repair over guessed
state. Malformed envelopes still reject before stale classification.

The example uses independently generated snapshots, not gameplay authority. It
models full repair by directly requesting a synthetic current complete set; no
network delivery/retry/ACK safety is claimed. Next establish a bounded per-recipient
sender cache and explicitly acknowledged baseline selection before wire integration.
No compression ratio, account identity, persistence, reconnect continuity, cohort
sharing, production component schema or performance claim. M8 remains open.
