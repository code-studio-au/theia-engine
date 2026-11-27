# M4 CPU bilinear texture reference (2026-10-14)

`theia_assets::texture_filter::linear_repeat` is a readonly f64 oracle for one
dense opaque RGBA8 sRGB mip, dimensions 1..1024 and finite UV magnitudes <=64.
It uses normalized top-left coordinates, half-texel centres, periodic edge taps
and four bilinear weights. RGB is decoded to linear values before interpolation;
alpha is required opaque. Forged extents/lengths/alpha and invalid coordinates
return errors before index arithmetic. No allocation or native dependency is
introduced. The diagnostic alpha scan is not intended as a runtime sampling path.

The model follows the Vulkan specification's
[sampling coordinate and linear filtering rules](https://raw.githubusercontent.com/KhronosGroup/Vulkan-Docs/main/chapters/textures.adoc)
and [sRGB image-read conversion](https://docs.vulkan.org/spec/latest/chapters/images.html).
The reference computes ideal weights; it does not model hardware weight
quantization or assert exact cross-vendor GPU arithmetic.

Four tests check known centres, midpoints, seams/negative repeat, decoding before
interpolation, 1000 samples against an independent all-texel periodic tent-kernel
sum, integer-shift invariance, thin/odd/constant levels, the sRGB transfer-function
toe and invalid/forged input rejection. All 54 asset tests pass, as do formatting
and denied-warning all-target Clippy. A bounded content-addressed example
`texture_filter <sha256.ktx2> <mip> <u> <v>` independently admits filename/hash,
complete-chain layout and cooked provenance before sampling.

The example executes 102 readonly probes over 17 levels in the existing square,
odd and thin native-KTX cooked chains. Results match a separate Python calculation
from raw indexed KTX texels within 1e-12. Logs/report:
`build/qualification/m4-linear-cpu/report.json`,
`m4-linear-cpu-{tests,clippy,build}.log`.

This is CPU/reference qualification only. No renderer/shader change or new GPU,
Windows, filtering-quality, mip-generation or performance claim. Next qualify an
opt-in bilinear sampler with explicit automatic integer mip selection against this
oracle. Nearest sampling stays preserved; trilinear/anisotropic filtering and
tiled/partial residency remain separate gates. No locked decision changes.
