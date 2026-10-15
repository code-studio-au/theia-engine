# Selected fine/coarse terrain height query — CPU gate (2026-10-15)

Before combining camera-driven residency with moving visual proxies, their feet
must follow the actual selected triangles. The older mixed benchmark interpolates
one regular whole-world 16 m mesh. Quadrant coarse bundles instead split perimeter
edges at 8 m, and fine bundles use 8 m interiors. Reusing that older interpolation
would place proxies above or below the selected rendered terrain.

`--world-ground-reference <scene.json points.json>` now loads/checks complete
immutable fixture bundles and extracts material-zero terrain triangles from actual
cooked meshlet geometry/material bindings. Each 64 m quadrant builds an 8x8 direct
XY cell index with compact candidate ranges. A query tests only candidate
triangles with barycentric interpolation; it performs no allocation per point.
Construction caps each surface at 128 triangles and eight candidates per cell;
the fixture uses two for fine and at most four for coarse. Heights are computed
from cooked f32 vertices, without the procedural analytic height function.

The reference evaluates all 16 fine/coarse selection masks for the same points.
An exact zero coordinate chooses the positive-axis region; outer coordinates
at +/-64 m remain included. Bounds, finite inputs, upward triangle orientation,
empty/capacity-exceeded cells and missing/ambiguous query coverage are checked.
This is a benchmark heightfield contract with one ground layer, not general cave,
roof, navigation or gameplay collision. The constructor does not prove continuous
coverage: a query into a hole fails, while offline terrain coverage qualification
remains the separate existing geometry gate.

## Evidence

`tools/qualify_world_ground.py` independently brute-forces double-precision
source triangles from normalized glTF rather than using the runtime cell index.
It compares 35605 XY points across all 16 masks: **569680 height comparisons**.
Points include grid vertices, quadrant seams, probes on both sides of 8 m lines,
4096 deterministic random positions and actual 10000-proxy positions at three
camera-route frames. The maximum absolute error is **1.26e-7 m**; the sampled
fine/coarse difference is **0.1138763 m**, within the prior 0.12 m offline bound.
Outside-world, empty and nonfinite input cases fail before Vulkan initialization.

The renderer suite has 30 passing CPU tests, including an independent affine
plane checked at 16641 positions and negative hole/conflicting-layer/reversed
winding cases. Formatting and denied-warning Clippy pass. Local ignored evidence
is `world-ground-native/report.json` and `world-ground-{tests,clippy,build}.log`.

The optimized executable passes the same 569680 comparisons and negative cases
(`world-ground-release/report.json`). All eleven Windows GNU packages cross-link
and pass denied-warning Clippy; CPU suites pass under Wine
(`windows-world-ground/report.json`). Native Windows GPU is unqualified; this
particular query gate executes no Vulkan on either platform.

## Remaining work

This gate executes no GPU shader, changes no existing rendered scene or proxy
height, and records no timing. GPU query encoding/submission and independent
readback qualification come next, followed by selected-world crowd rendering and
HUD/timing integration. Any additional GPU query allocations must be explicitly
budgeted; these CPU query structures do not enter the existing resident graph
ledger. Geometry remains presentation-only; server movement legality and world
collision require their own authoritative representation.
