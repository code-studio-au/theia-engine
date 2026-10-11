# M8 — Explicitly acknowledged sender baselines (2026-10-11)

The transport-neutral sender now records canonical prepared complete sets per
locally owned recipient. It selects delta baselines only from explicitly
application-acknowledged retained ticks, never transport ACKs or assumptions about
send success. THN2 remains full-snapshot-only in this unit.

## Executed evidence

Linux/Rust 1.99; ignored `build/qualification/m8-baseline-*` logs.

- Protocol tests 23 pass. Three additions verify unknown/future/prepared/duplicate/
  stale ACK selection; exact baseline values and heartbeat behavior; eviction and
  repair fencing; full repair followed by valid readonly delta reconstruction;
  invalid/conflicting/regressing state, foreign/old/disconnected contexts, reset
  and connection-counter exhaustion without partial cache changes.
- Independent complete-state example generates 10000 synthetic state packets plus
  final repair. Prepared packet counts: full 338/delta 9663 (includes final repair).
  Drops 769 state packets and 687 of 7563 ACK attempts; explicitly reorders 1846
  state packets/982 ACKs, observes 945 stale ACKs and 316 missing-baseline repairs.
  Final complete tick 10001 passes. Every selected delta baseline equals a prior
  explicitly advanced ACK; every receiver publication equals independent complete
  actor values. Sender <=8 sets, receiver <=2 sets, <=8 actors each.
- Evicted-ACK rejection is verified in the targeted unit test; this example's
  evicted-ACK count is zero. No claim it exercises every rejection category.
- Preceding 10000-generated delta/full repair example, renderer 22 tests and
  encrypted two-client packet-fault regression pass unchanged. Formatting and
  denied-warning all-target protocol Clippy pass. No shader/GPU path changed.
- Linux/Windows CI definitions execute both protocol examples; hosted CI and
  Windows remain unexecuted locally.

## Policy and limits

History limit 2..16, actor limit 1..128. Sender owns one live locally issued
recipient context; reset/disconnect clear all history/ACKs, connection counters
fail on exhaustion, and foreign or prior contexts fail before mutation. Caller
must bind recipient to the exact trusted connection; handles are not credentials.
Canonical sets reject bad IDs/order/ranges, regressing tick/watermark and conflicting
same-tick values before cache mutation. Retained vectors trim caller spare capacity.

Full state is prepared until a retained tick is acknowledged. Subsequent deltas
name that exact prepared set. Evicting it restores full state; a missing-baseline
repair request clears ACK selection and sets a latest-prepared-tick fence. ACKs
at/below that fence remain stale until a newer complete set is acknowledged. A
same-tick repair sends full state, but cannot reactivate deltas with an old ACK.
This conservative policy assumes subsequent server ticks continue; it is not a
baseline lease or frozen-world liveness guarantee.

Prepared history is bounded, not a queue of every unacknowledged packet. Outbound
packets/caller input allocation and transport buffers are outside retained-state
counts. Receiver retains only two sets, so baseline eviction can still require
explicit full repair. Repair delivery/retry/ACK behavior is simulated application
logic, not a network qualification or compression/latency/throughput benchmark.

Next add versioned bounded delta, snapshot ACK and full-repair messages and qualify
certificate-verified encrypted exchange. No cohort sharing, accounts, persistence,
cross-reconnect continuity, production component schema or performance claim.
