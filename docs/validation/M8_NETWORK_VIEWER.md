# M8 — Separate encrypted GPU viewer qualification (2026-10-11)

Render Lab now connects to the separate encrypted listener using its public DER
certificate and server-owned actor/target/profile bootstrap. Its live path creates
no local `World`: gameplay remains in the server process. The network worker
publishes readonly full snapshots to the existing mesh presentation profile.

## Executed evidence

Linux/X11, RX 7900 XTX/RADV Mesa 26.2.4, Rust 1.99, Slang 2026.19,
Vulkan loader/validation 1.3.275, RenderDoc 1.46. Ignored logs
`build/qualification/m8-viewer-*`; replay reports/images are generated artifacts.

- Renderer tests 22 and protocol tests 16 pass; formatting and all-target Clippy
  with denied warnings pass. New tests cover empty/disconnected identity, absent
  scene geometry, long-gap snapping, held aliases and bounded attack/focus reset.
- Default/shared shader variants each compile and SPIR-V validate 31 outputs.
  Existing compiler profile-upgrade notices remain; no fallback language needed.
- Listener plus two independent GPU viewer processes each pass 180 frames,
  31 readonly snapshots/latest tick 31 and zero Vulkan warnings/errors including
  teardown. Roles 10/20 come from verified server bootstrap, never local fixtures.
- Repeatable `qualify_network_viewer.py --wsi`: both pass 120 frames, ten swapchain
  generations, resize/minimize/restore/zero-size deferral and labeled injected
  acquire/present recovery. Both have zero diagnostics. Laboratory profile also
  passes standalone paired 180-frame execution with zero diagnostics.
- Native X11/XTest D/Space input: first viewer passes 600 frames, 100 snapshots,
  four acknowledged outcomes at server ticks 9/14/20/25, target health 75/50/25/0.
  Listener reports ticks 102/movements 125/hits 4/malformed 0/rejected moves 1,
  final health [100,0]. Both viewers have zero diagnostics. Input helper owns and
  checks only its spawned window/PID, releases keys and restores prior focus when
  still owned. Companion completes 660 frames; no Windows/Wayland input claim.
- RenderDoc pixel replays below pass with one three-group mesh draw. No vertex or
  index buffers, per-actor host draw or CPU position interpolation. Capture uses
  validation disabled; the standalone runs above separately verify Vulkan usage.
- Fixed-authority/indirect WSI regressions pass 120 frames with zero diagnostics;
  preserved headless compute verifies all 256 values. Seven malformed/incompatible/
  missing-certificate startup cases reject before native surface creation.
- Windows/Wayland/hosted CI remain unexecuted. Existing matrix covers the new
  dependencies and Rust tests; the GPU helpers are intentionally desktop-only.

| Replay | Sampled pixels | Actor/target/obstacle/background coverage |
|---|---:|---|
| Live local-demo | 32387 | 120 / 120 / 15 / 32132 |
| Live laboratory | 32370 | 120 / 120 / 0 / 32130 |
| Unpaired | 32400 | 0 / 0 / 0 / 32400 |
| Closed after snapshots | 32400 | 0 / 0 / 0 / 32400 |
| Preserved fixed fixture, alpha .5 | 32371 | 112 / 120 / 15 / 32124 |

Boundary bands excluded; sRGB channels allow two byte values of tolerance.
Laboratory has no invented obstacle. Unpaired capture verifies empty rendering;
its app completion gate intentionally fails because no snapshots were received,
so this is not a successful gameplay run. Closed capture verifies prior actors
are removed after its companion disconnects.

## Contract and limits

Verified bootstrap binds exact retained Quinn connection and readonly replica.
Two historical actor vectors (at most two actors each) cross a mutex-protected
front; stale/duplicate packets do not reset receipt time. One 64-byte push/one
mesh draw feeds two actors and the profile rectangle. Zero logical IDs suppress
actor output; bit 31 in the profile's blend mask suppresses absent geometry.

Worker uses a separate single-thread Tokio runtime. Movement is supersedable;
held aliases track independently, focus loss clears input and dead actors stop
sending movement. One in-flight reliable attack plus one coalesced pending bit
bounds local work. Same-ID pending/budget retries retain semantics; completed
identity is checked before outcome ACK. Exchange/request deadlines are two/three
seconds. Timeout/error ends this fixture, with no reconnect guarantee. Drop
signals worker shutdown, closes endpoint and bounds idle drain to one second.

Presentation phase is elapsed arrival time divided by the prototype 50 ms tick,
clamped to endpoints. For gaps of one through four ticks it covers the final tick
interval; larger gaps snap to latest. Health/membership remain current. There is
no prediction, extrapolation, server-clock estimator, delta baseline, account
identity, persistence or performance claim. Scene profiles, two actors and their
units are qualification settings, not production APIs or locked engine policy.
Next qualify this viewer under controlled post-handshake packet faults.
