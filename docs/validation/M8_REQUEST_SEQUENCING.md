# M8 bounded request sequencing — 2026-10-11

Implemented `theia-protocol`, a transport-neutral Rust authority adapter. The
headless server lab adds `--protocol-test`. No sockets or wire format are present.
This does not close the networking milestone.

## Executed local evidence

Rust 1.99/Linux; no GPU dependencies in this unit. Logs are ignored local files
under `build/qualification/m8-request-*`.

- Seven adapter tests: independent external sequence spaces; pending/completed
  retries and original cached health; conflicting payload reuse; ACK holes and
  tombstones; bounded pressure and admission retry; foreign/disconnected peers,
  target compaction and reconnect freshness; dead-attacker cached rejection;
  internal/request counter exhaustion; invalid limits and peer capacity.
- Typed lab: 10000 simulation ticks, 40000 distinct requests, 110000 duplicate
  replies, 40000 conflicts, 20000 window rejections. Each tick shuffles four
  requests with a seeded permutation and sends a much newer movement sequence.
  An independent one-hit-per-tick/cooldown model predicts health 930000 exactly.
  High request IDs are ACKed before the low ID; tombstones prevent window advance
  until the hole closes. End-of-tick retained count returns to zero.
- Preserved simulation library: 16 tests pass.
- Preserved combat lab: 5000 ticks, 9921 requests, 948 hits, 8973 cooldown
  rejections, final health 93364.
- Preserved movement lab: 100000 ticks, 300000 inputs, 293984 changes, 5998
  blocks, 9375 replay and 9375 malformed-intent rejections; maximum squared step
  22500 mm², matching prior totals.
- Adapter/app formatting, denied-warning all-target Clippy and tests pass.
  Linux/Windows CI definitions include the adapter and typed lab; hosted CI and
  Windows runtime were not executed.

## Scope and limitations

The adapter privately owns the simulation and allocates internal sequence values;
the existing simulation API's shared sequence ordering remains unchanged. External
movement and request IDs are independent. Requests start at zero; IDs must be
allocated contiguously although arrival and acknowledgement may reorder inside
the bounded window. No pending/unacknowledged outcome is silently evicted.

Applied movement watermark means processed input, not successful displacement.
Outcome acknowledgement means application consumption, not QUIC packet receipt
or durable commit. Out-of-order acknowledged entries retain replay tombstones.
Cached results remain immutable after later damage. Resource limits count peer,
input and retained request entries; there is no encoded byte budget yet.

Peer contexts are trusted server-issued handles, not authentication tokens.
Disconnect drops the session/cache and cancels its pending inputs. New contexts
cannot reuse old handles. There is no account identity, reconnect continuity,
persistence, replicated baseline, encryption, 0-RTT policy implementation or
performance claim. Next qualify an explicitly bounded encrypted Quinn exchange
with two local clients and a strict versioned message format.
