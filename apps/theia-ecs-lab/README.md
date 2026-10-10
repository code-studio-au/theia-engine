# M1 headless GPU presentation ECS qualification

Separate executable; neither M0-C headless compute nor M0-D Render Lab is changed.
Slang owns a device-local component arena and processes bounded structural queues.
The host compares batched diagnostics against `theia-ecs` after each publication.
This is correctness scaffolding, not a production allocator/scheduler or gameplay
simulation. It performs synchronous readback for testing, not a runtime data path.

```bash
python3 tools/compile_ecs_shaders.py
cargo run --locked --manifest-path apps/theia-ecs-lab/Cargo.toml
```

Optional Linux reference isolation:
`VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json`.
`--gpu <substring>` defaults to `7900`; `--shader-dir <directory>` overrides the
checkout build/shaders directory. No desktop session is required. Vulkan 1.3,
Sync2, mesh/task capability, sufficient descriptor/storage limits and Khronos
validation are checked. Only compute and Sync2 are enabled/used here; this is not
an execution test of the full renderer feature profile.

128 generational slots and two 32-row chunks per archetype. Position-only and
position+velocity archetypes each hold at most 64 entities. A single-lane mutation
kernel establishes deterministic command ordering, reserve-before-migrate,
swap-removal and epoch publication. Integration uses 64 parallel lanes. Final
production mutation parallelism is deliberately unimplemented.

World ID is 64-bit; local handle slot/generation and qualification publication epoch
are 32-bit. Generation wrap retires the slot permanently. Epoch wrap is rejected
by the qualification driver. Chunk sizes and bit budgets are not engine decisions.

The arena is 1,541 little-endian 32-bit words, all offsets in words:

| Offset | Column / length |
|---|---|
| 0 | Publication epoch / 1 |
| 1 | Chunk row counts / 4 |
| 5 | Generation / 128 |
| 133, 261 | World ID low/high / 128 each |
| 389, 517 | Archetype (0=dead, 1=stationary, 2=moving), absolute row / 128 each |
| 645, 773 | Retirement epoch, permanently exhausted / 128 each |
| 901 | Dense row-to-slot / 128 |
| 1029, 1157 | Position X/Y / 128 each |
| 1285, 1413 | Velocity X/Y / 128 each |

Command records have ten words: opcode, slot, generation, world low/high, position
X/Y, velocity X/Y, has-velocity. Status records have three: status, slot, generation.
Status 0 succeeds; 1 stale, 2 duplicate world ID, 3 slots exhausted, 4 chunks exhausted,
5 invalid/nonfinite component. Queues are bounded at 64 commands. Reclaim occurs
at batch boundaries; conflicts are resolved by recorded queue order. Failures do
not roll back earlier successful commands.

One additional device-local arena retains an old reader version. The GPU copies
this bounded test arena once, mutates current storage through several epochs and
runs a reader kernel against the retained version; every word must remain intact.
The host's oldest-reader epoch prevents handle reuse until release. This is a
selected-component version correctness test, not proof of asynchronous multi-queue
reader overlap or a production full-ECS-copy scheme.

Sync2 buffer barriers order mutation, integration, transfer and retained-reader
access; GPU-to-host barriers plus completion fences protect coherent diagnostic
mappings. RAII handles partial construction and destroys children before the device.
Synchronization validation is enabled; a deliberate callback-delivery check is
excluded from diagnostic totals. Real validation errors fail after teardown.

The qualification compares all live identities, locations and component values,
all mutation statuses and successful returned handles. It includes deterministic
capacity/conflict cases, 512 randomized 16-command batches, retained epochs and a
reset with generation `u32::MAX` to verify permanent retirement without wrap.
Arithmetic inputs remain bounded finite binary fractions; general overflow and
production time policies are not qualified. No performance claim is made.

Local evidence: [M1 qualification](../../docs/validation/M1_GPU_ECS.md).
