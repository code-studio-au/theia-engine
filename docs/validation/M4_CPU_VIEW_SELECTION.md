# M4 CPU per-view representation selection — 2026-10-15

Protected-world snapshots now resolve explicit per-view coarse/fine requests
against one immutable committed CPU epoch. A distant view can choose protected
coarse data while fine data remains resident for another view. A requested fine
chunk absent from the epoch selects its protected fallback and reports
`refinement_needed`; coverage is retained without claiming that the requested
geometry error was met. Pending reservations do not qualify as resident fine.

`Snapshot::select_view` requires exactly one request per region, rejecting missing,
duplicate and unknown IDs, including fallback IDs used as region IDs. Output is
in stable region order. `ViewSnapshot` retains all owners in its source epoch,
including representations it does not draw. Later publication/eviction does not
change an existing view. Dropping the last retained reader permits encoded-byte
reuse. The immutable region/fallback map is shared across snapshots.

The desired level is supplied by the caller. The API composes with the qualified
`lod::select`/`select_hysteresis` policy; it does not infer geometry error, own
per-view hysteresis history, load/evict data, acknowledge GPU readiness or issue
rendering work. Existing finest-resident selection and demand remain unchanged.
There is no asset-format, shader or locked architecture-decision change.

## Executed evidence

Ignored artifacts under `build/qualification/`:

- `world-view-assets-tests.log`: all **77** asset tests pass. Four new tests cover
  independent near/far views, malformed request rejection, pending/retained epoch
  separation and 5000 generated orthographic error-policy cases. The analytical
  policy oracle checks 10000 region choices, including unavailable refinement.
- `world-view-{native,release}/report.json`: debug and release independently check
  every **16 resident masks × 16 request masks**, or **1024 region choices** each,
  using the eight actual cooked fine/coarse graphs. All selections, missing-fine
  lists, epochs and unique encoded-byte unions match. Branch counts are 256 fine,
  512 requested coarse and 256 unavailable fine. The Rust example also checks
  retained coarse-view pins across fine eviction and exact byte release afterward.
- `world-view-covered-regression/report.json`: the prior five CPU demand/coverage
  routes pass all 687 samples each, preserving full/tight/coarse-only budgets,
  short-cap rejection and pending cancellation.
- `world-view-renderer-tests.log`: all 30 renderer tests pass. Asset and renderer
  denied-warning Clippy and formatting checks pass.
- `world-view-native-regression/`: a 120-frame headless debug streamed benchmark
  with 10000 proxies/HUD at 1280x720 passes actual Vulkan validation with zero
  warnings/errors. This preserves the existing finest-resident renderer; it does
  not exercise the new CPU view choices on the GPU.
- `windows-world-view/report.json`: all eleven packages cross-link and pass Clippy;
  Wine executes CPU tests, including the expanded asset suite. Native Windows
  GPU execution remains unqualified.

Reproduce the real-graph CPU check after building the example:

```sh
cargo build --locked --manifest-path crates/theia-assets/Cargo.toml \
  --example bundle_residency
python3 tools/qualify_world_views.py \
  --scene build/qualification/chunk-fallback-cook/scene.json \
  --output build/qualification/world-view-native
```

## Remaining integration

The renderer still selects its finest committed representation. Next connect
verified per-region geometric bounds and view error policy to these requests,
then separately qualify GPU work/descriptor selection, graphics-reader retirement
and proxy-ground agreement. Per-view demand merging, general hierarchy, morphing,
material/silhouette error and production catalogs remain open. CPU encoded
ownership is distinct from GPU allocation and rendering readiness.
