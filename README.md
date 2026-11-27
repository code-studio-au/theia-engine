# Theia Engine

An experimental Rust engine test repository for GPU-first MMO technology, using
Vulkan through `ash` and Slang shaders. Linux is the primary qualification platform;
Windows is a first-class target.

This repository is self-contained for engine development and qualification. Game
story documents, worldbuilding sources and narrative extraction tools belong in a
separate project. Engine tests should use small, synthetic fixtures without depending
on game-specific content or another Theia repository.

## Current applications

- `apps/theia-gpu-test`: the preserved headless Vulkan compute regression test.
- [Render Lab](apps/theia-render-lab/README.md): native window and swapchain tests,
  Slang mesh-generated triangles, task amplification and compute-written indirect
  work counts.

[Current status](docs/CURRENT_STATUS.md) records existing RX 7900 XTX / RADV
qualification evidence and its limits. M0-D qualification closeout remains open on
this branch. The wider engine architecture is a development target, not a claim of
implemented or benchmarked capabilities.

## Development

Start with the [developer workflow](docs/DEVELOPMENT.md) and
[platform and toolchain requirements](docs/PLATFORMS_AND_BUILD.md).
The [Render Lab README](apps/theia-render-lab/README.md) contains shader compilation,
run and validation commands. GPU execution requires suitable local hardware;
build and unit-test results alone do not qualify GPU behavior.

- [Architecture](docs/ARCHITECTURE.md) and [decisions](docs/DECISIONS.md)
- [Subsystem specifications](docs/SUBSYSTEMS.md)
- [Roadmap](docs/ROADMAP.md) and [risks and validation](docs/RISKS_AND_VALIDATION.md)
- [Content tooling](docs/CONTENT_AND_TOOLS.md) and
  [networking design](docs/NETWORKING_AND_SERVER.md)

Runtime/source languages are Rust, Go and Slang under the existing architecture
decisions; Python scripts support offline development. Keep private exports and
credentials outside the repository.
