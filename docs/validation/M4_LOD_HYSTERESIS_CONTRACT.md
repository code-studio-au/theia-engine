# M4 bounded heightfield LOD hysteresis contract (2026-10-11)

The transport-independent CPU oracle now exposes `lod::select_hysteresis` for the
qualified two-page continuous-heightfield profile. This extends the prototype
without changing a locked ADR or the existing stateless selection API. GPU adoption
is outstanding at this checkpoint; no runtime/performance claim.

A caller supplies the prior level and an entry fraction in [0,1]. Fine-to-coarse
requires both projected bounds to fit the inward-rounded budget
`hard_budget * (1 - entry_fraction)`. A previously coarse page stays coarse only
while the original hard-budget oracle permits it. Therefore hysteresis never
extends the allowed error above that limit. Inside the dead band an initially fine
view stays fine and an initially coarse view stays coarse. Clip/unbounded guards
always force fine; zero error remains eligible at zero budget. Fraction zero
preserves stateless selection exactly. Invalid fractions/profiles return errors.

The caller owns history for exactly one view/page pair and resets to Fine on context
change. This pure function has no cache, identity authentication, streamed page
demand, morphing or temporal disocclusion model. It consumes the existing producer
bound/profile preconditions and does not recompute geometry proof.

Three new tests cover entry/exit trajectories and context reset, guards/zero error/
disabled equivalence/invalid settings, and 10000 generated values checked against
an independent analytical orthographic state machine. All 42 asset tests pass;
renderer regression tests remain 22/22. Asset formatting and denied-warning Clippy
pass. No GPU execution is introduced in this unit.

Observed logs: `build/qualification/m4-lod-hysteresis-assets-{tests,clippy}.log`
and `m4-lod-hysteresis-renderer-regression.log`. Changes are local only.
Next: retain prior selection on GPU with explicit first-dispatch initialization,
read/write synchronization, hard-limit guards and replay trajectory qualification.
M4's broader hierarchy/streaming/virtual-texture gates remain open.
