# Architecture Decision Records (ADR)

The canonical initial register is `../DECISIONS.md`. Create one file per expanded or revised decision when the decision needs richer alternatives, experiments or migration implications. Suggested format:

```
# ADR-NNN — Title
Status: PROPOSED | LOCKED | SUPERSEDED | BENCHMARK REQUIRED
Date:
Context:
Decision:
Alternatives rejected:
Consequences:
Evidence/validation:
Supersedes:
```

Avoid silently turning a recommendation into a locked design or claiming implementation merely because the user approved the architecture. Critical revised decisions: native Rust/declarative scripting over WASM/Zig; QUIC via Quinn over custom UDP; immutable authored world rather than construction/destruction; Linux-first rather than Windows-first; external C++ offline tools permitted.
