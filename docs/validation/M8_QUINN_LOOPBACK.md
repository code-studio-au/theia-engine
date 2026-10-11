# M8 encrypted two-client loopback — 2026-10-11

Separate Rust `apps/theia-network-lab` qualifies bounded transport without changing
the renderer/headless GPU test. This is an M8 reference unit; M8 remains open.

## Dependencies and API sources

Locked Quinn 0.11.11, quinn-proto 0.11.19, rustls 0.23.45, ring 0.17.14,
Tokio 1.53.2 and rcgen 0.14.10. Quinn selects runtime-tokio/rustls-ring with
default features off. rcgen uses crypto/ring; no native gameplay code is added.
Versions are qualified pins, not a claim of latest releases.

Primary API references consulted:
[Quinn](https://docs.rs/quinn/0.11.11/quinn/),
[connection](https://docs.rs/quinn/0.11.11/quinn/struct.Connection.html),
[server configuration](https://docs.rs/quinn/0.11.11/quinn/struct.ServerConfig.html),
[rcgen](https://docs.rs/rcgen/0.14.10/rcgen/fn.generate_simple_self_signed.html),
and the downloaded Quinn 0.11.11 package's official common/client/server examples.

## Executed local evidence

Linux/Rust 1.99; logs ignored under `build/qualification/m8-quinn-*`.

- IPv4 localhost, ephemeral UDP ports: two successful certificate-verified
  handshakes and explicit `theia-lab-1` ALPN. Wrong hostname, untrusted certificate
  and mismatched ALPN each reject. Early data disabled at both endpoints; no
  verifier bypass. Certificate/private key only in memory.
- Movement DATAGRAMs use external sequences starting at 1000000/2000000; reliable
  request IDs start at zero. Range rejection before moving into range, processed
  movement watermark, one authoritative hit from each actor, both snapshots
  showing health [75,75]. Duplicate requests retain the original outcome without
  additional damage. Conflicting targets, ACK holes, full windows, post-ACK stale
  requests and stale movement fail as expected.
- A partially written reliable stream without FIN times out. The other client's
  request completes before that deadline. Oversized reliable data resets its
  stream; malformed DATAGRAMs do not disrupt later snapshots. Typical final run:
  26 fixed ticks, eight admitted movement inputs, two hits, three malformed
  exchanges and one stale move; tick count depends on scheduling, not benchmark.
- Twenty independent complete IPv4 executions pass with the same health/hit,
  admission and rejection assertions. Per-run ticks vary (25/26); no timing or
  performance conclusion is drawn.
- Three codec tests pass: canonical explicit bytes, all valid message truncations
  and extensions, wrong version, enum tags, zero/duplicate IDs, numeric bounds,
  and 20000 arbitrary-byte decoding/canonicalization cases. This is not a full
  coverage-guided fuzz campaign.
- Adapter/simulation regressions pass seven/16 tests. Typed request lab retains
  10000 ticks/40000 requests/110000 duplicates/40000 conflicts/20000 window rejects,
  final health 930000. Formatting and denied-warning all-target Clippy pass.
- IPv6 `--ipv6` actually attempts `[::1]` and fails `AddrNotAvailable`; this host's
  all/loopback `disable_ipv6` settings are both 1. No system setting was changed,
  no IPv4 fallback, no IPv6 runtime success claim. Windows/hosted CI unexecuted.

## Bounds and remaining gates

Two clients/actors, four in-flight stream handlers per connection, 32-entry event
channel, four retained gameplay request IDs per peer, 128-byte messages. Whole
stream exchanges have one-second deadlines. Transport windows: 256 per stream,
4096 connection receive/send, 1024/128 DATAGRAM receive/send queues; pending
handshakes limited to two and 4096/8192 per/total buffered bytes. Movement drops
when the event queue is full, then is recoverable by sequenced retransmission.
This fixture is not a general admission/rate-limit or production resource policy.

Each reliable exchange returns queued/pending/completed/acknowledged or explicit
rejection. Clients retry to obtain completed outcomes. The server's application
ACK floor is distinct from QUIC ACKs. Each tick sends a complete two-actor snapshot,
not a baseline delta. Trusted local fixture connection order binds actors;
TLS server verification is not client account authentication. No persisted key,
account credentials, durable effects, reconnect continuity or public endpoint.

A 50 ms Tokio lab interval advances fixed simulation steps and skips missed host
intervals; this is not a production scheduler. Worker shutdown aborts and joins
pending tasks. There is no packet-loss/reordering proxy yet, general snapshot
version/interpolation/baseline repair, disconnect lifecycle, renderer integration,
persistence or scale/performance claim. Next qualify controlled encrypted packet
loss/reordering before expanding replication or client integration.
