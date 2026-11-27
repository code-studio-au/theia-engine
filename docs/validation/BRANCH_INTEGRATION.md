# Branch integration — 2026-11-27

All local and fetched `origin` branch tips were checked for ancestry after fetching
GitHub with pruning. The engine development branches were already ancestors of
`feat/bevy-visual-benchmark` at `63159ea`; merging that tip incorporated their work.
The remaining Chronicle branch at `ef1fb3a` contained five documentation commits.

Integration started from cleaned `main` at `46d5456`. Merge commits `2440ef7` and
`abdaf69` include both histories. Every local and fetched remote branch tip is an
ancestor of the integrated result. Branch references were retained.

## Cleanup and conflict resolution

- Preserved all 19 story-document deletions and removal of both extraction scripts
  and `docs/CHAT_EXPORT.md`.
- Resolved the engine branch's modify/delete conflict by retaining the deletion of
  `CHAT_EXPORT.md`. Excluded its archived conversation and conversation-review
  documents from the resulting tree.
- Resolved the Chronicle branch's modify/delete conflict by retaining the deletion
  of `THEIA_INTERWOVEN_CHRONICLE.md`.
- No engine source conflicts occurred. Removed one trailing blank line from
  `tools/proxy_benchmark_reference.py` to pass `git diff --check`.
- Expanded the root README with implemented capabilities, technical achievements,
  scoped results, evidence links, architecture, package layout and run instructions.

## Checks executed on the integrated tree

| Check | Result |
|---|---|
| Rust formatting | Passed for all 12 packages, including the optional Bevy tool |
| Clippy | All 12 packages passed `--locked --all-targets -- -D warnings` |
| Rust tests | 189 passed across the 12 packages; no failures |
| Python tests | All 39 passed with `python3 -B -m unittest discover -s tools -p 'test_*.py'` |
| Default renderer shaders | 52 compiled and validated for Vulkan 1.3 |
| Shared primitive interface shaders | 52 compiled and validated for Vulkan 1.3 |
| GPU ECS shader compilation | Four compute entry points compiled and validated |
| Preserved headless compute shader | Compiled and passed SPIR-V validation |
| Authoritative movement executable | 100,000 ticks / 300,000 admitted inputs; speed, collision and rejected inputs verified |
| Authoritative combat executable | 5,000 ticks / 9,921 requests; 948 hits and 8,973 cooldown rejections matched the oracle |
| Request sequencing executable | 10,000 ticks / 40,000 distinct requests; duplicate/conflicting/window cases passed |
| Acknowledged baselines example | 10,000 generated packets; bounded histories and final full repair verified |
| Delta reconstruction example | 10,000 synthetic packets; missing-baseline and stale-state preservation verified |
| Encrypted replication with faults | Both clients reached tick 120 with exact reconstructed state and missing-baseline repair |
| Documentation | README local links resolve; previous cleanup deletions remain absent |

The encrypted run reported two server repairs. Its relays dropped 40/42 packets,
delayed 58/62, reordered 11/17 and duplicated 28/29, with at most two/one retained
packets. This is the synthetic replication fixture, not a native viewer or gameplay
execution test.

Rust commands used cached dependencies with `--offline --locked`. Initial sandbox
attempts could not unpack cached Cargo sources; reruns with cache write access
passed. The optional Bevy build used the existing extracted Wayland development
SDK via `PKG_CONFIG_PATH` and `LIBRARY_PATH`, as documented in its README. The
loopback test needed local socket access outside the sandbox.

Shader compilation used Slang 2026.19. The optional GLSL fragment path used the
available RenderDoc-bundled glslang 16.5.0; this differs from CI's pinned glslang
15.1.0. Both profiles passed `spirv-val`; this does not establish GPU execution
equivalence between compiler versions.

Logs and machine-readable results are local ignored artifacts under
`build/qualification/branch-integration/`.

## Limits

No `/dev/dri` GPU device was exposed in this execution environment. GPU execution,
native display/input, pixel replay, performance, Windows and cross-vendor checks
were not rerun during integration. Earlier qualification records remain historical
evidence for their tested revisions. Run the relevant local RX 7900 XTX qualification
profiles before extending those runtime claims to this merge.
