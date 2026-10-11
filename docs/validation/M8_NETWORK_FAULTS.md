# M8 encrypted packet fault qualification — 2026-10-11

`theia-network-lab --faults` inserts two bounded localhost UDP relays. Each
connection still uses certificate-verified Quinn/rustls/ring with ALPN and disabled
0-RTT. Relays forward opaque encrypted bytes and activate faults only after the
successful handshakes. No game message is altered or decrypted by the relay.

## Executed evidence

Linux/Rust 1.99/IPv4; ignored logs `build/qualification/m8-fault-*`.

- Faulted complete scenario passes two clients, server-owned movement and range,
  exactly two hits, health [75,75] observed by both clients; cached reliable retries,
  conflict/stale/window rejection and application ACK-hole repair remain correct.
- Ten independent complete faulted executions pass. Each logs actual positive
  drop/reorder/duplicate counts; per-relay ranges: drops 9..16, reorders 1..8,
  duplicates 8..12. Maximum pending packets 1..2 (cap two). Counts are scheduling
  dependent and are not latency/throughput benchmarks.
- Initial representative execution: relays forward 175/118 packets, drop 14/9,
  delay 21/16, reorder 7/3 and duplicate 12/8; maximum pending 2/1. Authority:
  29 fixed ticks/eight admitted movement inputs/two hits, three malformed messages,
  six stale movement retries, final health [75,75].
- Default direct loopback regression passes: 27 ticks/eight movements/two hits,
  three malformed messages/six stale inputs, health [75,75].
- Four unit tests pass. New relay test verifies byte-preserving client→server and
  server→client routes, ignores unknown source sockets, rejects an oversized
  datagram and confirms zero retained packets with faults disabled. Existing
  strict codec tests remain passing. Formatting and denied-warning Clippy pass.
- CI definitions execute direct/faulted IPv4 modes on Linux/Windows; hosted CI and
  Windows remain unexecuted. IPv6 still unqualified because the host disables it.

## Fault and ownership contract

Directional receive ordinals: every eleventh packet drops. Every seventh holds
when its direction has no pending packet; a following eligible packet forwards
first and releases the held one, explicitly reordering it. A 2 ms timer flushes
held packets after their 10 ms deadline, subject to host scheduling. Every
thirteenth eligible directly forwarded packet duplicates. Drop takes priority
and held packets are not duplicated; these are controlled overlapping rules,
not independent probability estimates. Ordinals include handshake packets but
fault injection begins only after the verified gameplay handshakes.

One pending packet per direction, each <=2048 bytes, bounds retained payload to
4096 bytes per relay/8192 combined. Fixed receive buffers are 2049 bytes each to
detect oversized/truncated input. The lab accepts only the fixed connection/server
source addresses. Socket/kernel/QUIC buffering is excluded from these relay
payload numbers; preceding transport limits remain configured. Owners abort tasks
on drop; explicit finish signals and joins them. Faults disable before connection
close draining, so shutdown is not itself a loss-qualification claim.

Client movement retransmits the same external sequence every 50 ms until a full
snapshot carries its processed watermark, under a two-second deadline. Duplicates
cannot reapply input or renew its held-motion expiry. New intent sequences are
independent from reliable request IDs. Repeated full snapshots recover missing
state without deltas; there is no baseline repair protocol yet. Reliable result
retrieval uses the same retained outcome/cache semantics as the preceding unit.

Malformed unreliable data may itself be lost, so the faulted case requires the
two guaranteed malformed reliable-stream rejections; it logs actual DATAGRAM
rejections when observed. Stalled-stream isolation remains checked before its
one-second deadline. Opaque packet statistics cannot prove which application
message a lost packet contained. No WAN path, handshake loss, sustained outage,
MTU/congestion/load, account authentication, persistence, snapshot interpolation
or performance claim. Next qualify client snapshot ordering/session reset before
presentation integration.

UDP API reference: downloaded Tokio 1.53.2 official source documentation for
`net::UdpSocket::recv_from` and `send_to`; receive cancellation is documented safe.
