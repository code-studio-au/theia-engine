# M7 interactive local authority fixture — 2026-10-11

Separate `--authority-demo` preserves fixed `--authority-test` and all preceding
renderer paths. Physical input requests direction/attack; private Rust `World`
decides legality, position, cooldown and damage/health/death. GPU reads completed
readonly state and generates/interpolates the existing fixed actor/obstacle quads.

## Executed local evidence

RX 7900 XTX/RADV, Linux/X11/Rust 1.99. Ignored logs `build/qualification/m7-interactive-*`.

- Renderer tests: 20. Five additions: scripted intents produce four hits/death and
  four swept static blocks with final actor [-400,-250] mm; held/released/opposing
  controls and attack coalescing obey ticks; alias release/focus reset retain/clear
  correct bindings; four-step catchup budget/backward-clock rejection/phase clamp;
  logical-ID matching and observed-respawn flags in the fixed GPU push encoder.
- Scripted 600-frame execution: 100 fixed ticks, four hits at ticks 4/8/12/16,
  target health 0, four static blocks, zero skipped wall intervals, zero Vulkan
  warnings/errors including teardown. The script stops motion after tick 25.
- Native X11/XTest input runs: D held/released followed by four Space press/release
  edges reaches Rust/Winit→authority. Latest run: hits at ticks 8/13/19/24,
  100 ticks/600 frames, four hits/death, zero blocks/skips/diagnostics. Initial
  core XSendEvent attempt was not delivered by this input backend; XTest native
  input replaced it. No Windows/Wayland native-input claim.
- Separate nonscripted 120-frame WSI run: 22 ticks, ten swapchain generations,
  resize/minimize/restore/zero-size handling and labeled recovery injection; zero
  diagnostics. Authority timer continues through redraw deferral. No separate
  thread or host-latency guarantee is inferred.
- Initial 120-frame scripted run reached four hits/death but failed the explicit
  26-tick collision completion gate. Exact frame-limit behavior was preserved;
  the longer complete script above passes. Duration depends on display refresh.
- RenderDoc 1.46 stable post-script replay at frame 300: one direct three-group
  draw; CPU comparison passes 32372 sampled pixels, coverage counts
  [120,120,15,32117] for actor/target/obstacle/background. Actor is blocked left of
  the obstacle; target is dead gray. One-pixel boundary bands excluded. Capture
  disables validation; independent standalone Vulkan validation passes separately.
- Fixed authority/clear/indirect/v2 bundle legacy WSI regressions pass 120 frames
  with zero diagnostics. Invalid script-alone/demo-alpha/demo-work options fail
  before Vulkan. Formatting/denied-warning all-target Clippy pass.
- Windows/hosted CI unexecuted; CI includes the pure completed-script oracle without
  Vulkan. Shader source is unchanged from the 31-output qualified preceding unit.

## Input, clock and resource contract

Physical WASD/arrows map to eight independently held bindings/four direction
axes; opposing directions cancel. Space requires a nonrepeat press edge. One
pending attack/tick coalesces edges, and focus loss clears held keys/pending attack.
Session/target/rules are private, so input cannot choose speed/destination/damage.

A 50 ms Winit deadline advances at most four fixed authority steps per wake.
After excessive delay, remaining wall intervals are counted/logged and discarded;
presentation snaps latest until the next regular step. Backward time rejects.
This is a same-process development authority fixture, not production scheduling
or a network client with gameplay authority. Driver/event-loop stalls can delay
CPU simulation; the independent Quinn server lab has no renderer dependency.

Replica retains readonly completed states; previous positions are matched by
logical ID. New/observed-respawn actors clear blend flags. One 64-byte push/three-
group draw per frame, with no per-actor Vulkan calls/draws/allocations/readback.
Interpolation phase is a separate fractional screen time; health remains latest.
Fixed two actor slots/static rectangle/orthographic XY are qualification profiles,
not a scalable actor/ECS/animation/character system.

Offline Python XTest harness uses third-party Xlib/Xtst, never runtime gameplay
C/C++. It identifies only the process/window it launches, checks focus before
key-down, releases held native key state, and restores prior focus when still
owned. [Xlib API](https://www.x.org/releases/X11R7.6/doc/libX11/specs/libX11/libX11.html)
and [XTest API](https://www.x.org/releases/X11R7.5/doc/Xext/xtestlib.html) were consulted.

No live Quinn feed, account identity, persistence, prediction, network clock
estimator, full collision mesh/navigation, UI/animation or performance claim.
Next connect the separately qualified encrypted authority feed to presentation.
