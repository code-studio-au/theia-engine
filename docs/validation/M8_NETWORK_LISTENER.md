# M8 separate encrypted listener/bootstrap — 2026-10-11

`--serve` and independent `--client-probe` processes reuse the shared bounded
fixture; no graphics client yet. Prototype protocol explicitly becomes THN2/
`theia-lab-2`, adding a reliable server-issued actor/target/scene welcome.

## Executed Linux evidence

Ignored logs `build/qualification/m8-listener-*`; public certificate generated
under `build/qualification/`, never a signing key.

- Four shared codec tests/one app relay test pass. Welcome truncation/extension/
  old version, unknown profile and duplicate actor/target fail. Formatting and
  denied-warning shared/app Clippy pass.
- Direct and actual encrypted packet-fault regressions retain two hits/health
  [75,75], certificate/hostname/ALPN rejection, cached outcomes, ACK repair,
  malformed/stalled-stream handling and readonly interpolation. The receiver
  also rejects the other client's exact connection before consuming its data.
- Local-demo profile: three independent processes (server + two probes), roles
  10/20, targets 20/10, applied movement sequence 1, positions [-850,-1000] and
  [850,-1000], health 100. Pair closes normally after probe disconnect.
- Laboratory profile: same independent ownership/applied watermarks, positions
  [-850,0]/[850,0], health 100. Shared empty-floor gameplay behavior remains intact.
- A six-second second-peer delay exceeds the configured five-second idle timeout;
  one-second keepalive preserves the first completed handshake. Both probes then
  receive assignments, move correctly and close normally.
- Portable process harness passes default profile; CLI non-loopback listener,
  malformed option and incomplete probe arguments fail before readiness/network
  gameplay. No system IPv6 setting was changed. Windows/hosted CI unexecuted;
  IPv6 remains unqualified on the disabled host.

## Bounds, identity and limitations

Same transport windows/request/input caps as preceding units; welcome is 22 bytes
inside the 128-byte message cap. Successful handshakes supply connection context.
Welcome stream read/write/completion has a two-second server deadline; client
acceptance permits a bounded 30-second pairing wait, then a two-second message
read. Client retains the exact connection object and locally issued replica
context, checks roster agreement and clears both on disconnect. Duplicate server
connection pairs reject before creating authority. No client actor credential is
accepted from packets; movement still carries direction/sequence only.

Default profile 1: central static rectangle +/-100 mm, actor spawns at +/-1000 X,
Y=-1000. Profile 0: empty floor/Y=0. Both retain 50 ms ticks, health 100, 250 mm
half-footprint and +/-4000 mm world. These are fixed prototype settings, not
approved production scene or network precision/timing choices. Profile metadata
will allow matching presentation without inventing world collision.

Public DER input is bounded to 1..65536 bytes; no verifier bypass. Server signing
key remains in its TLS config memory. TLS verifies server identity, not accounts.
The listener accepts one pair, not a reusable zone service; one peer leaving ends
that world, with no reconnect/roster repair, durable results or identity registry.
Source contexts are not authority partition epochs. Full two-actor snapshots need
no delta baseline; general repair/scaling remain absent. Next connect a live GPU
viewer using this separate encrypted authority, keeping gameplay off the client.
