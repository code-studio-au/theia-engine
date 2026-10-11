# M8 — Bounded replication-envelope codec (2026-10-11)

Separate qualification TDR1 envelopes carry full states, full-actor deltas,
snapshot application ACKs and full-repair requests. Existing THN2 movement/combat/
bootstrap codec, ALPN and live listener/viewer behavior remain unchanged.
This unit qualifies encoding only; no new message has executed over Quinn yet.

## Executed evidence

Linux/Rust 1.99; ignored `build/qualification/m8-replication-codec-*` logs.

- Shared crate nine tests pass. Three additions round-trip empty/full/maximal
  delta/ACK/repair messages including u64-max fields and explicit fixed-byte
  little-endian examples; reject every truncation and extension; reject unknown
  kind/version/optional tags, excessive counts, zero/duplicate/unordered IDs,
  numeric limits, delta overlap/duplicate removal, invalid baseline tick order and
  oversized messages. Invalid caller encode input returns errors without panics.
- 20000 arbitrary-byte cases exercise bounded decoding; every accepted message
  re-encodes to exactly the original bytes. This is deterministic adversarial
  sampling, not exhaustive fuzz coverage.
- Protocol 23 tests and renderer 22 tests pass; original encrypted THN2 fault
  regression preserves two authoritative hits/health [75,75] and reliable/readonly
  checks. Formatting and denied-warning all-target shared Clippy pass.
- Existing Linux/Windows shared-crate CI matrix includes these tests; hosted CI
  and Windows remain unexecuted locally. No shader/GPU changes.

## Binary limits and remaining gates

Magic TDR1, explicit u8 tags: full 0, delta 1, snapshot ACK 2, repair 3. All numeric
fields little endian. Optional movement watermark uses strict 0/1 tag. Full sets
carry target tick, watermark, actor count and complete actor values. Delta carries
baseline/target ticks, watermark, upsert count/actors, removal count/logical IDs.
ACK carries only the complete target tick; repair has no payload. Exact lengths
are required; no trailing data, fallback version or unknown-field acceptance.

Decoder rejects >128 bytes first, then each count >2 before allocating. Largest
valid delta with two upserts/two removals and watermark is 88 bytes; full is 63,
ACK 13, repair 5. Empty full sets are legal. Actor IDs/positions/health use the same
prototype canonical validation as readonly replicas. Envelope validation does not
prove removal membership or baseline/current watermark consistency; those remain
mandatory in `Delta::reconstruct` and readonly publication.

No production component format, bootstrap negotiation, encrypted delivery,
transport classification, repair scheduling or baseline ACK handling is qualified
by this codec. Next qualify isolated certificate-verified Quinn exchange before
live viewer adoption. No account/persistence, compression ratio or performance claim.
