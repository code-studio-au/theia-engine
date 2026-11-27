# M4 complete authored texture mip contract (2026-10-11)

A separate pure-Rust `texture_mips::TextureMipChain` reader admits complete authored
KTX2 mip chains. Existing `texture::Texture` and renderer/bundle admission retain the
single-mip profile. Both share the strict colour descriptor/key-value validator;
no native runtime dependency or filtering/transcoding was added.

The new profile accepts opaque RGBA8_SRGB/BT709, top-left 2D, one face, no arrays,
depth or supercompression. Dimensions are 1..1024; level count must be exactly
`floor(log2(max(width,height))) + 1`, bounded to 1..11 before allocation. Each axis
halves with floor and clamps to one. The final level is 1x1. Index entries remain
base-first, while physical texel blocks are smallest-first, contiguous and four-byte
aligned. Exact offsets/lengths, no trailing bytes, canonical DFD and metadata,
opaque alpha at every level and optional whole-file digest are checked. This matches
the applicable [Khronos KTX2 layout](https://registry.khronos.org/KTX/specs/2.0/ktxspec.v2.html).
Other valid KTX layouts are outside this deliberately bounded profile.

The default encoded budget is 4 MiB; metadata retains the 16 KiB cap. A full 1024x1024
RGBA8 chain exceeds that default budget and rejects even though its dimensions fit.
Level descriptors occupy one bounded vector; texels borrow immutable encoded bytes.
No mip-generation request/partial chain, sparse page table, demand feedback, residency
publication, GPU upload or sampler behavior is established by this reader.

`tools/cook_texture_mips.py` accepts schema-1 JSON with explicit base-first `levels`
(width/height/opaque integer `rgba`). It invokes the preceding producer to reuse the
pinned source-build/archive/version/license gate, keeps its single-level output
private/temporary, then calls KTX 4.4.2 raw creation with all authored levels. Every
native texel is compared with its supplied source bytes. Canonical provenance
packaging relocates every level index while preserving DFD and texels. Official KTX
validation passes. Inputs intentionally have authored colors per level; no claim
that they approximate a filtered base image. Native tool behavior was checked against
its pinned local help/source; [current tool documentation](https://github.khronos.org/KTX-Software/ktxtools/ktx_create.html)
is background, not evidence for the pinned version.

| Native cooked case | Levels | Encoded bytes | SHA-256 |
|---|---:|---:|---|
| 4x4 → 2x2 → 1x1 | 3 | 1060 | d8b82f12b2559fcb4cd4e07b14fe7b40013220e72090b99f8ba9351625f734e5 |
| 5x3 → 2x1 → 1x1 | 3 | 1048 | 855485c6f3c727142c9fe4f01b85ab117cfc57db415896cfb1e50a5ca55716c1 |
| 1024x1 → … → 1x1 | 11 | 9356 | d630966ec3ebef171ac2fb486c5df2817155a25bb72db59a0a50dc9825120dc7 |

The independent Rust `texture_mips` example verifies hash filenames and admits each
complete native file. Repeated square cooking produces an identical report/bytes.
A 400-byte native fixture, with the writer/orientation keys but no cooked provenance,
is committed alongside its authored source JSON. Native fixture SHA-256:
`3b0746e585cbc0eea9259475e0db9197a67d9a056d9848086bc72ed1818678b6`.
The fixture was written by the qualified KTX tool and directly validates with it.

Four new Rust tests cover exact source texels/borrowed storage and preserved legacy
rejection, every truncation plus count/section/index/hash/alpha/metadata errors,
odd/rectangular/1x1/max-level boundaries and 10000 mutated inputs without panics.
All 46 asset tests and 22 renderer regression tests pass. Three new producer tests
bring texture producer tests to eight; formatting, denied-warning asset Clippy,
Python syntax, workflow YAML parsing and whitespace checks pass. Windows GNU assets/
examples cross-link/lint and all 46 asset tests pass through owned Wine. This does
not establish native Windows or GPU mip execution.

Legacy 16-image complete-set WSI streaming still presents 120 frames/five publications
with zero Vulkan diagnostics. A multi-mip file supplied to the old single-image path
rejects during renderer initialization with zero diagnostics and no presentation.
That path creates Vulkan context before texture admission; an initial test incorrectly
expected pre-Vulkan rejection. The actual ordering is recorded, not changed here.
CI now includes native mip production and Rust admission; hosted execution is untested.

```bash
PYTHONPYCACHEPREFIX=build/qualification/python-cache \
python3 tools/cook_texture_mips.py \
  --ktx build/qualification/ktx-tool/source-build/Release/ktx \
  --build-dir build/qualification/ktx-tool/source-build \
  --source-archive build/qualification/ktx-tool/ktx-source-v4.4.2.tar.gz \
  --source fixtures/textures/opaque_4x4_mips.json \
  --output-dir build/qualification/m4-mip-contract/assets
# Pass the emitted path to:
# cargo run --manifest-path crates/theia-assets/Cargo.toml --example texture_mips -- <path>
```

Observed artifacts: `build/qualification/m4-mip-contract/` producer reports, native
logs, Rust reader/test/lint logs, legacy GPU/rejection logs and Windows/Wine logs.
Next qualify full-chain Vulkan upload and fixed mip-view sampling before derivatives,
filtering, automatic mip choice or tiled fallback. M4 stays open; local changes only.
