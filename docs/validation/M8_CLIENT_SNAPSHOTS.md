# M8 readonly client snapshot publication — 2026-10-11

`theia-protocol::replica` supplies bounded readonly client state, independently
of authority/transport. `theia-network-lab::client` now routes both clients'
consumed DATAGRAM snapshots through it. No shader/renderer changes.

## Executed local evidence

Linux/Rust 1.99; ignored logs `build/qualification/m8-snapshot-*`.

- Protocol suite: 12 tests, including five new replica tests. Newest full-set
  replacement, identical duplicates, stale ticks, same-tick conflicts, monotonic
  processed movement watermark, empty/removal and changed logical IDs pass.
- Foreign/old connection contexts fail; new contexts clear old state and accept
  a restarted tick domain. Disconnect clears state. Counter exhaustion leaves
  current context/state intact; maximum tick never wraps.
- Budget/numeric/sorted-ID/duplicate-ID failures preserve state. Mutating a
  caller clone cannot change the replica. Caller vector capacity reserved for
  10000 extra records is discarded before retaining the single live actor.
- Seeded 10000-delivery model drops/reorders/duplicates complete snapshots in a
  bounded queue; the replica always equals the independently tracked maximum
  delivered tick's entire state. A later full set repairs all omitted intervals.
- Direct/faulted encrypted two-client executions pass authoritative gameplay,
  outcome/ACK repair, malformed/stalled-stream checks and readonly publication;
  both observe health [75,75]. Actual example counts: direct 27 ticks/eight
  movements/two hits/seven stale inputs; faulted 27 ticks/seven movements/two hits/
  seven stale inputs. Movement counts can vary because loss permits a held intent
  to advance between snapshot receipts; speed and gameplay remain server-owned.
- Four app tests pass. Protocol/app formatting and denied-warning Clippy pass.
  Preserved request lab: 10000 ticks/40000 requests, 110000 duplicate replies,
  40000 conflicts, 20000 window rejections, final health 930000.
- Windows/hosted CI and IPv6 remain unexecuted/unqualified as preceding units.

## Contract and remaining work

One current complete actor set, configured 1..128 actors, sorted unique logical
IDs, integer positions within +/-1000000 mm and health <=1000000. The local
prototype profile is not an approved general network schema. Caller input/decode
allocation is outside retained-state budget; accepted spare capacity is removed
with boxed-slice conversion. Allocator overhead/transport buffering are excluded.

Only newer server ticks publish. Same tick must carry the identical complete
state; no within-tick revision semantics exist. Processed input watermark cannot
regress on a newer tick; stale ticks are discarded. It does not certify successful
displacement or durable effects. Lifecycle is represented by complete membership,
not speculative local authority. Snapshots remain presentation-only copied values.

Locally issued opaque connection context prevents earlier connection deliveries
from republishing after reset. It is not authentication, wire authority epoch or
persistent reconnect identity. New connection begins a fresh tick/watermark domain.
No client mutation of authority, interpolation/extrapolation/prediction, network
clock synchronization, delta baseline/repair, renderer integration or performance
claim. Next add a bounded presentation interpolation contract with explicit clocks.
