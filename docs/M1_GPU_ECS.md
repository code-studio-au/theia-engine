# M1 — bounded GPU presentation ECS qualification

Approved target: ADR-006 and SUBSYSTEMS section 1. This document describes a
qualification slice, not a complete ECS or a new locked architecture decision.

## Reference contract

Two fixed archetypes: position-only and position+velocity. Columns are dense SoA
inside bounded chunks. Stable 64-bit world identity, `(slot, generation)` handle
and physical archetype/chunk/row are distinct. Spawn, despawn and velocity add/remove
commands execute in recorded queue order at a publication epoch. A failing command
records a reason and preserves that entity; earlier successful commands remain
committed. Destination capacity is reserved before migration. Swap-removal repairs
the moved entity's metadata. Retired handles cannot be reused until older reader
leases complete; generation overflow permanently retires the slot.

The CPU oracle in `crates/theia-ecs` owns reference allocation/columns and copies
retained reader data for correctness comparison. Those copies are test artifacts,
not the production epoch/versioning strategy. It provides no gameplay authority.

## GPU qualification sequence

1. CPU oracle and independent randomized map comparison.
2. Separate headless `apps/theia-ecs-lab`, explicit Vulkan RAII, Slang initialization,
   bounded structural mutation, and parallel bulk position integration. Compare
   status, identity/location bijection and live components after each batch.
3. Retain device-local reader storage across later mutations, prohibit early handle
   reuse and verify retained data through GPU work; compare with the oracle. Test
   capacity overflow, stale/conflicting handles, generation reuse and wrap.
4. Record executed shader/compiler/driver evidence, validation and limitations.
   Only then move into M2's bounded 2.5D candidate/culling slice.

Initial mutation dispatch may use one lane to establish ordered correctness on a
small queue. It is not the production parallel sort/group/reserve allocator, a
scale benchmark, or a claim that serialized mutation will meet MMO throughput.
Bulk coherent component processing must run in parallel. CPU uploads/readbacks are
batched qualification plumbing; production scene state remains GPU resident.
GPU epoch width, chunk sizes and arena counts are explicit test parameters, not
approved final bit budgets. Retained versions protect data as well as handle reuse.

## Current evidence

CPU oracle local tests pass: relocation/migration, capacity/conflict results,
generation exhaustion, retained readers, invalid components, and 10,000 randomized
operations against a separate map representation. The separate GPU slice now passes 526 batches / 8,717 commands against this oracle
on RX 7900 XTX with zero validation diagnostics. See [execution evidence](validation/M1_GPU_ECS.md).
This qualifies the bounded reference gate; production parallel mutation, concurrent
reader overlap and scale remain open.
