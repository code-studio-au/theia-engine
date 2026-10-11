# M7 authoritative movement evidence (2026-10-11)

This begins the roadmap's parallel server track. M4 and M7 remain open; advanced
GI is deferred. The new Rust library/lab do not modify GPU or asset code.

The single-writer authority owns rules, immutable static collision and connection
context bindings. Input contains only directional axes and sequence. Movement
occurs once per server fixed tick after applying the latest admitted direction,
with conservative integer speed normalization, exact swept static rectangle
collision, outer world bounds and missing-input expiry. Ordinary actors overlap.
Logical server-supplied IDs remain independent of dense SoA row placement.

Default prototype values are 50 ms/tick, 3000 mm/s, 250 mm square half extent,
three held ticks, 128 actors, 256 queued inputs and four inputs/session/tick.
These are qualification settings, not locked engine/wire constants. Positions
use integer millimetres on Z=0. World coordinates are bounded +/-1000000 mm;
scene contains <=256 rectangles. No mesh/capsule/slope/cave/flight collision,
sliding or navigation is claimed. Collider definitions are independent of
renderer LOD/GI. Session context binding is not authentication or a credential.

All nine tests pass:

- Fixed tick/latest intent, once-per-tick speed bound, diagonal normalization,
  held-input expiry and replay rejection.
- Non-blocking overlapping actors and immutable snapshot values.
- Swept obstacle tunneling and world-boundary rejection.
- Logical identity through row compaction, respawn with fresh context, cancelled
  stale queued input and foreign-world context rejection.
- Bounded global/session input pressure without consuming retry sequence;
  maximum sequence cannot wrap into replay.
- Invalid scene/rules/ID, actor capacity/duplicate IDs and atomic tick/expiry
  exhaustion preserve current state and pending input.
- 5000 integer segments compared to an independent orientation/edge oracle.
- 5000 three-actor authoritative ticks preserve speed, collision and keyed
  change-stream consistency (15000 actor updates/replay checks).

The local headless lab executes:

| Ticks | Admitted inputs | Movement changes | Static/world blocks | Replay / malformed rejections | Max squared movement |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 10000 | 30000 | 29131 | 867 | 939 / 939 | 22500 mm² |
| 100000 | 300000 | 293984 | 5998 | 9375 / 9375 | 22500 mm² |

The speed cap is 150 mm/tick, including multiple admitted inputs/diagonals. The
loop advances fixed simulation time without realtime pacing; these are
correctness runs, not latency/throughput benchmarks. Tick counts 0/100001 fail.
Formatting and denied-warning Clippy pass for library/app. App builds/tests
pass; its actual deterministic executions supply integration evidence.

Artifacts under ignored `build/qualification/`:

- `m7-authority-tests.log` (nine tests).
- `m7-authority-app-tests.log` (app build/test).
- `m7-authority-{lab,long}.log` (10000/100000 ticks).
- `m7-authority-invalid-{0,100001}.log`.

Linux execution is observed. Linux/Windows CI matrices and lab runs are added,
but hosted CI/native Windows execution was not performed. No Quinn, external
clients, combat, inventory, persistence, worker handoff, general ECS, GPU-authority
or renderer integration claim. Next qualify server-owned interactions/combat.
