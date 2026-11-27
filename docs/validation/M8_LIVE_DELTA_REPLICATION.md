# M8 — Actual authority to GPU full/delta replication (2026-10-11)

Actual server snapshots now use per-recipient prepared histories and explicit
application ACK baselines. Live GPU clients reconstruct TDR1 full/delta state before
readonly publication. THN3/ALPN theia-lab-3 explicitly revises the previous profile;
THN1/THN2 and the prior ALPN reject. No architecture decision or gameplay rules change.

## Executed evidence

Linux/X11, RX 7900 XTX/RADV Mesa 26.2.4, Rust 1.99, RenderDoc 1.46.
Ignored `build/qualification/m8-live-delta-*` logs/reports.

- Faulted independent GPU pair: both pass 180 frames/30 readonly complete snapshots
  through tick 31, one full/29 delta/30 queued ACKs, zero diagnostics. Actual server
  full 3/delta 59/accepted snapshot ACKs 53, health [100,100]. Relay faults are
  positive and retained packets <=2. No full repair occurs in this ordinary run.
- Faulted native input: 600 frames/88 complete snapshots through tick 100,
  one full/87 deltas/88 queued snapshot ACKs; four acknowledged combat hits/death.
  Server ticks 101/movements 114/hits 4/health [100,0], full 2/delta 199/
  accepted snapshot ACKs 161. Both viewers have zero diagnostics. This ordinary
  run has no missing-baseline repair; it does not alone qualify repair handling.
- Explicit faulted ACK-withholding WSI pair: both pass 120 frames/ten swapchain
  generations with zero diagnostics. Clients publish 18/19 complete sets through
  tick 23, full 2/2, accepted delta 16/17, queued ACK 16/17, repair requests 2/1,
  intentionally withheld ACKs 2/2. Server consumes two full-repair requests and
  prepares full 5/delta 41. One queued repair is not observed by the server; repeated
  state/control repairs both replicas. Actual packet faults remain enabled.
- RenderDoc repair capture passes 32387 sampled pixels, counts [120,120,15,32132],
  one three-group mesh draw; companion reports two full/27 deltas/one repair/
  two withheld ACKs. Closed capture passes 32400 background pixels and no actor/
  obstacle output. Validation disabled in captures; standalone diagnostics above
  are qualified separately. Shaders unchanged from the 31-output qualified slice.
- Laboratory pair passes 180 frames with zero diagnostics; server full 2/delta 60/
  ACKs 62 and no invented obstacle. Separate process probes still qualify roles
  10/20/movement watermark/health 100 and normal shutdown.
- Direct/faulted headless gameplay retains two hits/health [75,75], reliable retry/
  conflict/window/ACK-hole repair, foreign connection rejection, stalled stream
  isolation and readonly interpolation. Direct run also observes actual full repair.
  Old ALPN is now explicitly included in negative handshake configurations.
- Isolated encrypted replication/fault regression still verifies both final tick
  120 values and missing-baseline repair. Shared 11/renderer 22 tests pass;
  formatting and denied-warning all-target Clippy pass. New client tests prove
  full/delta roster failure cannot partly publish, missing baselines preserve both
  endpoints, full repair restores state and controls/empty/partial roster reject.
- Windows/Wayland/hosted CI remain unexecuted. Existing matrix qualifies the updated
  libraries/apps; target-GPU/input helpers require the qualified desktop session.

## Source, sequencing and publication

Server binds two sender recipient contexts to its exact authority peers. Eight
prepared sets per peer; only explicit snapshot ACK enables a retained baseline.
Unknown ACK rejects without inventing state. Repair clears baseline and fences
old ACKs; eviction forces full state. ACK/repair are supersedable DATAGRAMs admitted
through the bounded 32-entry event channel, independently of reliable combat ACKs
and external movement sequences. Retained data counts remain bounded; no cohort
sharing/compression or bandwidth/latency benchmark is claimed.

Client checks exact retained connection before reading. Full/delta reconstruction
must produce the two bootstrap IDs before atomic publication. Stale data never
resets GPU receipt time. Missing baseline queues repair, preserves both endpoints,
and keeps previous verified GPU state; no partial delta reaches rendering. Latest
complete tick is ACKed after publication; queued ACK != transport delivery or durable
commit. Counters distinguish accepted state and queued controls.

Explicit repair qualification withholds delta ACKs only until a missing baseline
is observed. It is never default behavior. Full states remain ACKed, allowing loss
of an initial ACK to recover. Once eviction occurs the injection ends; normal
repair/ACK retries continue. This tests actual authority/cache/connection/client/GPU
integration without altering gameplay legality or locally fabricating state.

No general roster/incarnation protocol, frozen-world baseline lease, account
identity, reconnect continuity, persistence, WAN congestion/MTU/load, clock
synchronization/prediction or performance claim. M7/M8 remain open. Broader platform
qualification and remaining asset/GI/character milestones are still outstanding.
