# Isolated nested Wayland qualification (2026-10-11)

Owned Weston 13 runs on the existing X11 desktop, while Theia creates an actual
Wayland Vulkan surface and swapchain. This is additional platform reference
qualification; native Wayland seat/session, Windows and other vendors remain open.
No architecture decision changed and no performance measurement is claimed.

## Executed evidence

Linux Mint 22.3, RX 7900 XTX/RADV Mesa 26.2.4, Rust 1.99/Winit 0.30.13,
Vulkan loader/validation 1.3.275; ignored `build/qualification/wayland-*` artifacts.

- Corrected six-mode runs: clear, mesh-only triangle, direct task, compute-written
  indirect, fixed authority and local demo each pass 120 frames/seven swapchain
  generations, zero Vulkan warnings/errors including teardown. Native backend
  logging confirms Wayland, FIFO uses five images, recovery probes/resize/explicit
  zero-size deferral execute. Demo observes 39 fixed ticks in one run; no native
  Wayland keyboard/combat input qualification is inferred.
- Faulted live encrypted full/delta/repair pair: both pass 120 frames/seven
  generations with zero diagnostics and actual Wayland backend logs. Representative
  first client: 34 readonly complete sets through tick 40, full 2/delta 32,
  queued ACK 32/repair 1/intentionally withheld ACKs 2. Server ticks 40/full 5/
  delta 75/accepted ACKs 59/repairs 2, health [100,100]. Relay actual faults are
  positive and maximum pending <=2. No local gameplay World in viewers.
- X11 regression still passes 120 frames/ten generations and its actual minimize/
  restore requests with zero diagnostics. Headless compute verifies all 256 values.
- Renderer 22 tests, formatting and denied-warning all-target Clippy pass. Only
  backend logging and platform-aware smoke control change in Rust; shaders unchanged.
- Initial clear smoke minimized successfully, then timed out after 30 seconds:
  Winit's Wayland backend ignores programmatic unminimize. This failed attempt is
  preserved in `wayland-initial-minimize`. Corrected smoke explicitly skips that
  unsupported round trip and requests the final resize instead. It does not claim
  native/user-driven Wayland minimize/restore qualification.
- RenderDoc 1.46 portable capture attempt fails instance creation: its layer rejects
  VK_KHR_wayland_surface. Log reports one warning/one error and no capture; replay
  report records failure. No relaxed validation criterion or silent X11 fallback.
  Standalone validation above is separate. Wayland pixel/replay capture remains open.
- Final helper report records native surface Wayland, six checked modes, network
  pair true, replay false and programmatic minimize/restore false. No hosted CI,
  native-seat, Windows/cross-vendor or performance claim.

## Tool provenance and isolation

Ubuntu Noble packages 13.0.0-4build3 downloaded with apt and extracted under ignored
`build/qualification/wayland-tool/root`, not installed system-wide:

| Artifact | SHA-256 |
|---|---|
| weston amd64 deb | bcf6f344c6d5bede9f3f601be1015b0550fa31dd8c7b3e51ac0fe34702dac73c |
| libweston-13-0 amd64 deb | ad6a150d0a21674a797091107617ddc9884a9c8b5a4edd1465636b419b05b88f |
| Weston 13.0.0 upstream source tar.xz | 52ff1d4aa2394a2e416c85a338b627ce97fa71d43eb762fd4aaf145d36fc795a |

Archive source: Ubuntu mirror `pool/universe/w/weston/`; source was read to verify
module/binary mapping semantics, not compiled or shipped as custom runtime code.
Weston's GL compositor uses Radeon/radeonsi; Theia's renderer uses Vulkan/RADV.
These driver roles are separate. Existing system dependencies satisfy this host;
extraction alone is not a general dependency installer.

Helper scopes module/data/library paths only to its compositor child, creates an
owned short mode-700 temporary XDG_RUNTIME_DIR/socket, explicitly selects X11/GL
backend, avoids user configuration and disables idle blanking. Rust clients have
DISPLAY/WAYLAND_SOCKET removed and use only the owned Wayland socket. Each child
is bounded, only owned processes are terminated, and the temporary directory is
cleaned. No system compositor/session/seat configuration or global layer install.
Qt debugger UI uses host X11 while its injected Rust target selects Wayland; the
available debugger rejects that requested surface before any capture.

Primary references: [Weston running/backends documentation](https://wayland.pages.freedesktop.org/weston/toc/running-weston.html),
[Winit 0.30.13 minimize API limitations](https://docs.rs/winit/0.30.13/winit/window/struct.Window.html#method.set_minimized).
Pinned local Weston help/source and Winit source were also inspected. No unsupported
restore API is treated as working. Next investigate an isolated compositor screenshot
oracle or a debugger with proven Wayland support; do not infer replay from execution.
