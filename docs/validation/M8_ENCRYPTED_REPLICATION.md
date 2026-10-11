# M8 — Isolated encrypted delta/ACK/full-repair exchange (2026-10-11)

Separate `--replication-test` runs TDR1 application messages through verified
Quinn connections. It uses synthetic complete two-actor states and independent
client value checks; no gameplay rules/authority or live viewer behavior changes.
Existing THN2 encrypted gameplay/listener/viewer modes are preserved.

## Executed evidence

Linux/IPv4/Rust 1.99, Quinn 0.11.11/rustls/ring; ignored
`build/qualification/m8-encrypted-replication-*` logs.

- Direct run: final tick 120 verified by both clients. Server prepares eight full
  states/232 deltas, consumes 230 application ACKs/two repair requests. Each client
  receives four full states/116 deltas, one missing baseline, 119 new complete sets,
  zero duplicate/stale publications. Every published set matches independent
  complete positions/health/IDs/watermark; disconnect clears both endpoints.
- Three independent faulted encrypted runs pass. Each client observes at least
  one missing baseline/full repair and exact final tick 120. Representative run 0:
  server full 13/delta 231/ACKs 187/repairs 2; clients full 4/6, delta 101/106,
  missing 1/1, new 103/109, duplicate 1/2. Some complete ticks are lost; no guessed
  state or partial reconstruction is published.
- Run 0 relays: forwarded 436/469, drops 39/42, delays 56/63, reorders 14/19,
  duplicates 28/30, maximum two retained packets each. All three executions record
  positive actual drop/reorder/duplicate counts. Scheduling-dependent counters are
  not latency/throughput measurements or proof of the lost application payload.
- Shared nine tests pass; app formatting and denied-warning all-target Clippy pass.
  Original encrypted gameplay fault regression still verifies two hits/health
  [75,75], reliable request/ACK repair and readonly interpolation. No shader/GPU
  changes; live viewer remains on the preceding qualified full-snapshot mode.
- Linux/Windows CI definitions include direct/faulted replication tests. Hosted
  CI, Windows and local IPv6 runtime remain unexecuted (IPv6 disabled on this host).

## Protocol and ownership

TLS certificate verification/ALPN and disabled early data use the qualified shared
configuration. This explicitly selected test mode uses existing theia-lab-2 ALPN;
there is no production feature negotiation or old-peer compatibility claim. TDR1
is separate from THN2 gameplay messages. All encoded messages remain <=128 bytes.
Server consumes ACK/repair only from its two exact connection futures and binds
locally owned sender recipient contexts. No packet-supplied actor context.

Snapshots/deltas and snapshot ACK/repair requests are supersedable DATAGRAMs.
Clients ACK only complete published state, never partial/missing reconstruction.
Missing baseline sends repair and leaves presentation endpoints intact. Repeated
states retry missing control delivery; repairs fence delayed old ACKs, and sender
cache eviction forces full state. ACKs of unknown/unprepared ticks cannot create
baselines. All controls share connection congestion and bounded transport queues.

Clients deliberately withhold ACKs for ticks 2..6 after the initial acknowledged
complete set. This makes two-set receiver eviction outrun the eight-set sender
baseline and deterministically exercises full repair in direct qualification.
Faulted qualification checks observed repair rather than assuming packet counts
alone prove it. Integer source ticks advance at the test 50 ms cadence; no latency
or clock synchronization guarantee. Synthetic positions/health are not gameplay.

Server waits for both final complete-tick ACKs before closure. A same-tick ACK under
a repair fence can confirm terminal delivery without enabling deltas; this does
not change the sender's conservative frozen-world baseline policy. Clients reject
closure without exact final tick 120. Entire mode times out after 20 seconds.
Endpoint close draining is bounded to one second with faults disabled and relays
alive; relays signal/join on completion and abort on drop. Joined session futures
cancel together on error. No unbounded task queue or baseline history.

Next explicitly revise live protocol/ALPN and integrate actual authority snapshots
with the GPU viewers. No account authentication, persistence, cross-reconnect retry,
cohort sharing, general component precision/NetLOD, WAN/load, production feature
negotiation or performance claim. M8 remains open.
