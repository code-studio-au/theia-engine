# Windows GNU build and compatibility evidence (2026-10-11)

Rust 1.99 cross-builds x86_64-pc-windows-gnu using an extracted private MinGW SDK.
Wine 9 compatibility tests execute compiled Windows code on Linux. Neither
cross-building nor Wine qualifies native Windows 11/MSVC/Vulkan execution.

## Executed evidence

Ignored `build/qualification/windows-*` logs/artifacts; existing Linux Mint host.

- All 11 packages from the Rust CI matrix pass denied-warning all-target Clippy
  and cross-link normal/test/example targets. Six normal app executables are
  PE32+ x86-64 Windows console binaries, including the 100 MiB debug renderer.
  Binary size is an artifact fact, not a performance/memory baseline.
- All 129 substantive unit tests pass through Wine: renderer 22, ECS 6, assets 39,
  glTF importer 12, simulation 16, protocol 23, shared network fixture 11. Four
  app test harnesses contain zero tests; they are not counted as substantive tests.
- Normal Windows authority executables through Wine: 10000 movement ticks/30000
  inputs/max squared movement 22500; 5000 combat ticks/9921 requests/948 hits/
  health 93364; 10000 request-adapter ticks/40000 requests/110000 duplicate replies/
  40000 conflicts/20000 window rejections, health 930000. Independent models pass.
- Normal Windows readonly script oracle through Wine reaches 26 ticks/four hits/
  four static blocks/death, actor [-400,-250], without Vulkan or local client
  gameplay authority claims. JSON push/reference values match the existing fixture.
- Initial shared relay oversize test fails because its worker terminates on
  Winsock's truncated receive error. Corrected targeted Wine test logs actual OS
  error 10040 and then successfully forwards both valid directions, drops the
  oversized datagram and retains no pending packets with faults disabled.
- Linux shared 11 tests/formatting/Clippy and encrypted gameplay/replication packet-
  fault regressions pass after the fix. No shader/render/GPU behavior changes.
- Initial SDK setup failures were missing compiler/dlltool paths; the private child
  environment now supplies them. No Cargo.lock, transport dependency or global
  Cargo configuration changes.
- Hosted CI/native Windows/MSVC/GPU remain unexecuted. No performance claim.

## Socket behavior and explicit limitation

Winsock returns WSAEMSGSIZE for UDP larger than the receive buffer and discards the
excess; Linux reports a truncated length. The bounded relay now recognizes exactly
Windows error 10040, counts a drop and never forwards those bytes. Other errors
still terminate. Receive buffer remains 2049 bytes; retained encrypted payload
remains <=2048 bytes/direction. An error supplies no usable source, so its drop
counter is not source-attributed. Unknown complete-source packets remain ignored.
[Microsoft recvfrom contract](https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-recvfrom).

Available Wine 9 rejects the pinned quinn-udp 0.5.16 IPV6_V6ONLY query on an AF_INET
socket with OS error 10045. Winsock trace and pinned source identify the operation,
before any QUIC handshake. Therefore encrypted Windows execution is **unqualified**
in this compatibility environment. Native Windows behavior is unknown locally;
this observation is not a declaration of a native Quinn bug or native success.
No IPv6 host setting change, dependency patch, plaintext/RUDP/crypto bypass or
unsupported-socket fallback is applied. Existing Linux qualification is preserved.

## Tool scope/provenance

Rust target added to the existing Rustup toolchain; compiler remains Rust 1.99.
SDK packages downloaded from Ubuntu Noble with apt and extracted only below
ignored `build/qualification/windows-tool/root`:

| Package | Version | SHA-256 |
|---|---|---|
| gcc-mingw-w64-x86-64-posix | 13.2.0-6ubuntu1+26.1 | 0855fa1d4a618550faac3a3e4352aa3226833b28184ad3c3b496a68a22c4eda2 |
| gcc-mingw-w64-x86-64-posix-runtime | 13.2.0-6ubuntu1+26.1 | aca6b438e0b55b11cedcf1170a1ee26e7979e34192e34f6e6389e560908a5bfc |
| binutils-mingw-w64-x86-64 | 2.41.90.20240122-1ubuntu1+11.4 | 28bfabbf64a68813a56360bca056072af9ff71ef4090a4b8a8a085afcda3e4dd |
| mingw-w64-common | 11.0.1-3build1 | 4a42fd30fbc3838c58c929d8a4882682fc12408bb7a0e6cafa913c0f0a57c187 |
| mingw-w64-x86-64-dev | 11.0.1-3build1 | 018480a62907fcca19c6bf2975ae01f5c92c23dee8c3cf7a8355c10bf7d42c6f |

Compiler tools/headers are third-party offline build tools, not custom game/runtime
C/C++. Child-scoped SDK PATH/compiler/archiver/linker and target directory keep
native Linux builds unchanged. Wine uses only an owned qualification prefix; its
runtime chatter is disabled for CPU tests, assertions/exit codes determine success,
and no Vulkan diagnostic count is claimed from those tests. Targeted network
failure uses explicit Winsock tracing. Prefix-scoped wineserver cleanup never
addresses the user's default prefix. SDK/registry/executables remain ignored.
[Official Rustup cross-compilation setup](https://rust-lang.github.io/rustup/cross-compilation.html).
