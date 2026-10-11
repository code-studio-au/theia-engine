# Theia authoritative simulation prototype

Bounded single-writer Rust movement authority, separate from the presentation
ECS and GPU renderer. It uses dense SoA columns with server-supplied logical
world IDs and opaque server connection contexts. There is no identity registry,
authentication, networking, persistence or distributed ownership implementation.

`Scene` defines an immutable flat floor and <=256 static rectangles. `Rules`
bound actors/queued inputs and choose fixed tick, speed, square footprint and
input expiry. Defaults (50 ms, 3000 mm/s, 250 mm half extent, three held ticks)
are prototype test values. Millimetre precision is not an approved global/wire
format decision. Movement uses exact integer normalization and rational swept
rectangle tests; no client destination/time/speed or GPU truth is accepted.

At administrative tick boundaries, spawn logical actors and bind their returned
`Session` to a trusted future transport context. `submit` admits sequenced axes
in [-1000,1000]; it queues input without promising completed movement. `step`
uses latest accepted direction and moves each actor at most once. Queue limits
reject without consuming a new retry sequence. Stale inputs fail; held directions
expire. Frames expose applied input sequences, legal movement changes and blocked
IDs. Static obstacle contact is blocking; world footprint contact at the outer
bound is allowed. Whole blocked moves are rejected; there is no wall sliding.

Ordinary actors overlap and do not participate in movement collision. Compaction
never changes identity. Despawn cancels queued input and retires its context;
respawning the same logical character issues a fresh context. Supplied IDs must
retain their logical meaning; callers must not recycle them for unrelated actors.

Nine tests cover authority/lifetime/counter/input/speed/collision invariants and
independent randomized geometry checks. No complete ECS, combat, navmesh,
mesh/capsule/cave/flight collision or renderer connection is implemented.
