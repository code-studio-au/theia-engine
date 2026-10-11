# M8 — Packet faults through live GPU viewers (2026-10-11)

The same bounded opaque UDP relay now lives in the shared qualification crate.
Network lab and Render Lab reuse it; `--network-faults` requires live loopback mode
and activates only after certificate-verified bootstrap. Handshake/bootstrap loss
is excluded. The server and renderer remain separate processes.

## Executed evidence

Linux/X11, RX 7900 XTX/RADV Mesa 26.2.4, Rust 1.99, RenderDoc 1.46.
Ignored logs/reports: `build/qualification/m8-live-fault-*`.

- Two faulted GPU viewers pass 180 frames, server-assigned roles 10/20 and zero
  Vulkan diagnostics. Representative first viewer accepts 26 full snapshots through
  tick 31. Its relay reports forwarded 167/drop 15/delay 19/reorder 6/duplicate 12,
  maximum two retained packets. Both reports exercise every fault category.
- Faulted WSI pair passes 120 frames/ten swapchain generations each, labeled
  recovery injection and resize/minimize/restore/zero-size deferral, zero diagnostics.
  Reports: forwarded 121/129, drops 10/9, delays 14/16, reorders 3/6, duplicates
  9/9; maximum pending 2/1. Received snapshots 20/19, latest tick 23.
- Faulted native XTest input passes 600 frames/four acknowledged outcomes/death:
  server ticks 9/14/20/25, health 75/50/25/0. First viewer accepts 88 snapshots
  through tick 100. Server reports ticks 102/movements 113/hits 4/malformed 0/
  rejected moves 1, final health [100,0]. Both viewers have zero diagnostics.
  First/companion relays: forwards 504/358, drops 44/32, delays 64/46, reorders
  21/15, duplicates 32/23, maximum pending two each. This verifies four effects
  and acknowledged results despite actual encrypted packet faults, without claiming
  which application payload was lost. An initial repeat using fixed key timing
  produced three hits and a correctly rejected cooldown request: delayed delivery
  shifted one attack from tick 20 to 22, leaving only three ticks before tick 25.
  The input helper now waits for each acknowledged outcome before spacing the next
  key press; both relay reports and retained bounds are checked explicitly. The
  repaired run passes four hits/death at ticks 9/15/21/28, 90 received snapshots
  through tick 100 and zero diagnostics. First relay drops 42/reorders 21/duplicates
  30 with maximum pending two.
- Faulted RenderDoc replay passes 32387 sampled pixels, actor/target/obstacle/
  background counts [120,120,15,32132], one direct three-group mesh draw. Capture
  validation is disabled; standalone diagnostics above are checked separately.
- Direct live-pair regression passes. Headless encrypted fault lab still passes
  two hits/health [75,75], reliable retry/conflict/ACK-hole/window repair, stalled
  stream isolation and readonly publication/interpolation checks.
- Shared crate six tests pass (four wire/two relay). New public-relay guard rejects
  coincident, unbound, non-loopback or mixed-family routes before socket creation;
  byte-preserving routing/unknown source/oversize checks remain. Renderer 22 tests,
  formatting and denied-warning all-target Clippy pass. No shader changes.
- Windows/Wayland/hosted CI and IPv6 runtime remain unexecuted. Existing build/test
  matrix exercises the moved relay tests; live qualification requires a desktop GPU.

## Lifetime and limits

Each viewer relay knows exact server/client socket addresses, holds one <=2048-byte
packet per direction, and forwards encrypted bytes without decoding/replacing
payloads. Same ordinal fault rules as [preceding evidence](M8_NETWORK_FAULTS.md).
Counts depend on scheduling and are not latency/throughput results.

Worker cancellation is handled outside the connected-session future: it clears
presentation immediately, disables faults, closes its endpoint, retains the
transparent relay for at most one second of QUIC close draining, then signals/joins
it and prints final counters. Dropping a session aborts its request task set;
dropping a relay aborts its worker on error paths. This bounded teardown is exercised
by both normal viewer completion and companion/server closure.

No sustained outage, handshake loss, WAN congestion/MTU, generalized reconnect,
prediction, clock synchronization, deltas/baseline repair, accounts, persistence
or scale claim. Next isolate bounded delta reconstruction and missing-baseline
full-state repair before changing the qualified wire version.
