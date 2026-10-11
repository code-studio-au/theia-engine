# M8 bounded presentation interpolation — 2026-10-11

Separate CPU reference consumes readonly replicated state. It does not advance
server simulation and is not a production GPU smoothing pipeline.

## Executed evidence

Linux/Rust 1.99; ignored logs `build/qualification/m8-presentation-*`.

- Protocol suite: 16 tests, four new presentation tests. Endpoints, fractional
  midpoint, independent screen times, no extrapolation, latest health, stable
  logical IDs, new/removed actors, observed dead→alive, gap fallback and reset pass.
- 3000 seeded position/time comparisons match independent convex-weight sums to
  <1e-8 mm. Every result stays within the bounded source profile; current health
  remains 75 and authority positions remain unchanged.
- Server ticks near u64::MAX retain an exact local interval: samples at MAX-1+.5
  between MAX-2/MAX produce alpha .75, positions [500000,-500000] mm. Integer
  tick is never first converted to f64. NaN/infinite/out-of-range fraction and
  invalid gap policy fail.
- Direct encrypted two-client run: 27 ticks/eight movements/two hits, health
  [75,75], 88/16 explicitly supplied screen-time samples. Faulted run: 27 ticks/
  eight movements/two hits, health [75,75], 92/16 samples; encrypted relays actually
  drop 14/9, reorder 9/2 and duplicate 12/8 packets. Counts depend on scheduling.
- Every consumed snapshot now samples the presentation oracle at before/history/
  current/far-ahead test times. IDs/count/current health agree with readonly
  replica; positions are finite and within +/-1000000 mm. Test clocks are not a
  synchronization or visual-quality claim.
- Four app tests, formatting, denied-warning protocol/app Clippy pass. Existing
  monotonic snapshot/request tests remain passing. Windows/hosted CI unexecuted;
  IPv6 still unqualified on the disabled host. No GPU shader changed.

## Contract and remaining work

At most two full snapshots, each capped by configured 1..128 actors, normalized
retained vector capacity. Rejected/stale/duplicate publications preserve both
endpoints; reset/disconnect releases both. Input/decoded caller allocation and
allocator overhead remain outside retained payload counts.

Explicit `Time {integer tick, fraction}` is validated. A caller-selected max gap
1..120 ticks is a prototype test policy. Samples clamp to retained endpoints;
long gaps snap latest, without extrapolation. Matching logical IDs blend positions
in f64 using bounded local tick differences. Latest membership and health remain
exact. New actors snap, removed actors disappear, observed dead→alive snaps.

There is no incarnation/teleport flag: a respawn entirely hidden by dropped
snapshots cannot be detected. Interpolated visual paths may cut corners between
legal server positions and are not collision/movement truth. No prediction,
clock synchronization, lag compensation, GPU smoothing/renderer integration,
baseline delta protocol, account authentication, persistence or benchmark claim.
Next inspect the renderer and qualify a small presentation connection while
preserving prior GPU paths.
