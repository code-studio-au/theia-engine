# M7 bounded authoritative combat evidence (2026-10-11)

Pure Rust single-writer authority on the analytic flat/static-rectangle fixture.
No GPU code changes, external clients, networking or persistence are involved.
M7 remains open; this supplements the movement slice.

Attacker identity comes from trusted server context; input carries only target
and sequence. Health/damage/range/cooldown are private server-owned state/rules.
Movement precedes combat, then attack requests execute in admitted order against
current state. Only hits consume cooldown. Death clears motion and rejects new
actions; queued attacks by an actor killed earlier in the tick fail. Ordinary
actors do not block this prototype's LOS. Collision footprint and point combat
range are distinct; no general 3D hit-volume/ability/projectile system is claimed.

Sixteen library tests pass, retaining all nine movement/geometry tests. Seven
combat tests cover:

- Server-owned damage, once-per-cooldown hits, replay rejection, death/stopped input.
- Self/unknown/out-of-range/static-LOS/dead targets and no cooldown on rejection.
- Post-movement range and deterministic same-tick attacker death/order.
- Multiple attackers coalescing one final health change; actors do not occlude LOS.
- Inclusive exact range and target despawn/row compaction without wrong-row damage.
- Invalid health/range/cooldown rules and checked timing exhaustion before mutation.
- 5000 seeded ticks compared to an independent health/cooldown schedule.

The actual headless combat lab executes 5000 ticks, 9921 attack requests, 948 hits
and 8973 cooldown rejections, leaving 93364 of 100000 test health. Every result
and coalesced health change matches the independent model. Additional executable
cases qualify static LOS and post-movement/same-tick lethal ordering. Its test
profile uses damage 7/cooldown five ticks; library defaults remain local prototype
100 health/25 damage/1500 mm range/four ticks. These are not production constants.

The existing 10000/100000-tick movement executions keep their preceding movement
changes/blocks/rejections and 150 mm/tick speed bound. Formatting, denied-warning
Clippy and app tests/build pass. Conflicting lab modes reject. Linux actual
execution is observed; Windows CI is configured but not executed here.

`Frame.applied` acknowledges authoritative input processing, not successful
movement/combat, a component baseline ACK or durable commit. Attack outcomes
carry request sequence; health changes coalesce once per target/tick. The local
store still has a shared movement/attack sequence space. Independent transport
ordering/idempotent bounded outcomes must be qualified before mixed DATAGRAM/
reliable delivery. No Quinn/authentication/persistence implementation is claimed.

Artifacts under ignored `build/qualification/`:

- `m7-combat-tests.log` (16 library tests).
- `m7-combat-app-tests.log`.
- `m7-combat-lab.log` (actual combat oracle execution).
- `m7-combat-movement-{regression,long-regression}.log`.
- `m7-combat-invalid-modes.log`.

Navigation/mesh collision, larger gameplay interactions, hit volumes, inventory,
renderer integration, networking and persistence remain open.
