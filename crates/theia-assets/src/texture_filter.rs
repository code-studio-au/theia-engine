//! Bounded CPU oracle for opaque RGBA8 sRGB bilinear repeat within one authored mip.
//! Decode RGB before interpolation. No mip blending/generation or GPU precision claim.
use crate::{AssetError, Result, texture::MAX_DIMENSION, texture_mips::MipLevel};

/// Normalized coordinates, top-left rows, half-texel centers and repeat at edges.
/// The input must be a dense opaque mip, with finite UV magnitudes <= 64.
/// This readonly diagnostic oracle scans alpha; it is not a runtime hot path.
pub fn linear_repeat(level: &MipLevel<'_>, uv: [f64; 2]) -> Result<[f64; 3]> {
    let (first, fraction) = coordinates(level, uv)?;
    Ok(interpolate(level, first, fraction))
}

#[derive(Clone, Copy, Debug)]
pub struct LinearBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

/// Diagnostic envelope for at most one weight-quantization step per axis.
/// Bits must come from the selected device; 4..16 is this oracle's bounded profile.
/// This is not a complete model of GPU coordinate/colour arithmetic.
pub fn linear_repeat_bounds(level: &MipLevel<'_>, uv: [f64; 2], bits: u32) -> Result<LinearBounds> {
    if !(4..=16).contains(&bits) {
        return Err(AssetError("unsupported texture weight precision"));
    }
    let (first, fraction) = coordinates(level, uv)?;
    let step = 1. / f64::from(1u32 << bits);
    let mut bounds = LinearBounds {
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
    };
    // A bilinear function's extrema on a rectangle occur at its corners.
    for y in [-step, step] {
        for x in [-step, step] {
            let value = interpolate(
                level,
                first,
                [
                    (fraction[0] + x).clamp(0., 1.),
                    (fraction[1] + y).clamp(0., 1.),
                ],
            );
            for (axis, v) in value.into_iter().enumerate() {
                bounds.min[axis] = bounds.min[axis].min(v);
                bounds.max[axis] = bounds.max[axis].max(v);
            }
        }
    }
    Ok(bounds)
}
fn coordinates(level: &MipLevel<'_>, uv: [f64; 2]) -> Result<([i64; 2], [f64; 2])> {
    if !(1..=MAX_DIMENSION).contains(&level.width)
        || !(1..=MAX_DIMENSION).contains(&level.height)
        || level.texels.len() != level.width as usize * level.height as usize * 4
        || level.texels.as_chunks::<4>().0.iter().any(|p| p[3] != 255)
        || uv.iter().any(|v| !v.is_finite() || v.abs() > 64.)
    {
        return Err(AssetError("unsupported linear texture sampling profile"));
    }
    let size = [level.width, level.height];
    let position =
        std::array::from_fn::<_, 2, _>(|i| (uv[i] - uv[i].floor()) * f64::from(size[i]) - 0.5);
    let first = position.map(|v| v.floor() as i64);
    let fraction = std::array::from_fn::<_, 2, _>(|i| position[i] - position[i].floor());
    Ok((first, fraction))
}
fn interpolate(level: &MipLevel<'_>, first: [i64; 2], fraction: [f64; 2]) -> [f64; 3] {
    let size = [level.width, level.height];
    let mut result = [0.; 3];
    for y in 0..2 {
        for x in 0..2 {
            let px = (first[0] + x).rem_euclid(i64::from(size[0])) as usize;
            let py = (first[1] + y).rem_euclid(i64::from(size[1])) as usize;
            let weight = if x == 0 {
                1. - fraction[0]
            } else {
                fraction[0]
            } * if y == 0 {
                1. - fraction[1]
            } else {
                fraction[1]
            };
            let offset = (py * level.width as usize + px) * 4;
            for (axis, value) in result.iter_mut().enumerate() {
                let encoded = f64::from(level.texels[offset + axis]) / 255.;
                let linear = if encoded <= 0.04045 {
                    encoded / 12.92
                } else {
                    ((encoded + 0.055) / 1.055).powf(2.4)
                };
                *value += weight * linear;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    const PIXELS: [u8; 16] = [
        0, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 255, 255, 255, 255,
    ];
    fn quad() -> MipLevel<'static> {
        MipLevel {
            width: 2,
            height: 2,
            texels: &PIXELS,
        }
    }
    #[test]
    fn quantized_weight_envelope_contains_rounding_at_dark_edges_and_rejects_bad_bits() {
        let q = quad();
        for bits in [4, 8, 16] {
            let scale = (1u32 << bits) as f64;
            for fraction in [0., 0.0023632, 0.25, 0.5, 0.9999, 1.] {
                let uv = [(fraction + 0.5) / 2., 0.25];
                let bounds = linear_repeat_bounds(&q, uv, bits).unwrap();
                for quantized in [
                    (fraction * scale).floor() / scale,
                    (fraction * scale).ceil() / scale,
                    (fraction * scale).round() / scale,
                ] {
                    assert!(
                        bounds.min[0] <= quantized + 1e-14 && bounds.max[0] + 1e-14 >= quantized
                    );
                }
                let ideal = linear_repeat(&q, uv).unwrap();
                for (i, value) in ideal.into_iter().enumerate() {
                    assert!(bounds.min[i] <= value + 1e-14 && bounds.max[i] + 1e-14 >= value);
                    assert!(
                        (0. ..=1.).contains(&bounds.min[i]) && (0. ..=1.).contains(&bounds.max[i])
                    );
                }
            }
        }
        for bits in [0, 3, 17, u32::MAX] {
            assert!(linear_repeat_bounds(&q, [0.; 2], bits).is_err());
        }
        assert!(linear_repeat_bounds(&q, [f64::NAN, 0.], 8).is_err());
    }
    #[test]
    fn centers_half_texel_edges_and_repeat_decode_before_interpolation() {
        let q = quad();
        for (uv, want) in [
            ([0.25, 0.25], [0., 0., 0.]),
            ([0.75, 0.25], [1., 0., 0.]),
            ([0.25, 0.75], [0., 1., 0.]),
            ([0.75, 0.75], [1., 1., 1.]),
            ([0.5, 0.5], [0.5, 0.5, 0.25]),
            ([0., 0.], [0.5, 0.5, 0.25]),
            ([-1., 2.], [0.5, 0.5, 0.25]),
            ([0., 0.25], [0.5, 0., 0.]),
            ([-0.25, 1.75], [1., 1., 1.]),
        ] {
            assert_eq!(linear_repeat(&q, uv).unwrap(), want);
        }
        // Averaging encoded black/white and then decoding would yield ~0.214,
        // rather than the required 0.5 linear red at this midpoint.
        assert_eq!(linear_repeat(&q, [0.5, 0.25]).unwrap()[0], 0.5);
    }
    #[test]
    fn random_samples_match_independent_periodic_tent_kernel_and_integer_shifts() {
        let bytes: Vec<u8> = (0..15).flat_map(|i| [i * 17, i * 11, i * 7, 255]).collect();
        let level = MipLevel {
            width: 5,
            height: 3,
            texels: &bytes,
        };
        let mut seed = 719u64;
        for _ in 0..1000 {
            let mut coordinate = || {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                (seed >> 32) as f64 / u32::MAX as f64
            };
            let uv = [coordinate(), coordinate()];
            let actual = linear_repeat(&level, uv).unwrap();
            let mut want = [0.; 3];
            // Sum contributions from every texel and its neighboring periodic
            // images, independently of the oracle's floor/four-tap indexing.
            for y in 0..3 {
                for x in 0..5 {
                    let weight = (0..2)
                        .map(|axis| {
                            let (index, size) = if axis == 0 { (x, 5) } else { (y, 3) };
                            (-1..=1)
                                .map(|period| {
                                    let distance = uv[axis] * size as f64
                                        - (index as f64 + 0.5 + period as f64 * size as f64);
                                    (1. - distance.abs()).max(0.)
                                })
                                .sum::<f64>()
                        })
                        .product::<f64>();
                    for axis in 0..3 {
                        let encoded = bytes[(y * 5 + x) * 4 + axis] as f64 / 255.;
                        let linear = if encoded <= 0.04045 {
                            encoded / 12.92
                        } else {
                            ((encoded + 0.055) / 1.055).powf(2.4)
                        };
                        want[axis] += linear * weight;
                    }
                }
            }
            let shifted = linear_repeat(&level, [uv[0] - 7., uv[1] + 9.]).unwrap();
            for axis in 0..3 {
                assert!((actual[axis] - want[axis]).abs() < 1e-12);
                assert!((actual[axis] - shifted[axis]).abs() < 1e-12);
                assert!((0. ..=1.).contains(&actual[axis]));
            }
        }
    }
    #[test]
    fn one_texel_axes_constant_levels_and_srgb_toe_are_supported() {
        let bytes = [10, 11, 255, 255];
        let tail = MipLevel {
            width: 1,
            height: 1,
            texels: &bytes,
        };
        let want = [
            10. / 255. / 12.92,
            ((11. / 255. + 0.055) / 1.055f64).powf(2.4),
            1.,
        ];
        for uv in [[0., 0.], [-64., 64.], [0.25, -0.9]] {
            let actual = linear_repeat(&tail, uv).unwrap();
            for axis in 0..3 {
                assert!((actual[axis] - want[axis]).abs() < 1e-14);
            }
        }
        for (width, height) in [(1, 7), (5, 1), (3, 5)] {
            let bytes = bytes.repeat(width * height);
            let level = MipLevel {
                width: width as u32,
                height: height as u32,
                texels: &bytes,
            };
            let actual = linear_repeat(&level, [-0.37, 2.43]).unwrap();
            for axis in 0..3 {
                assert!((actual[axis] - want[axis]).abs() < 1e-14);
            }
        }
    }
    #[test]
    fn forged_levels_and_nonfinite_or_out_of_profile_uvs_reject_without_panics() {
        for (width, height, texels) in [
            (0, 2, &PIXELS[..]),
            (2, 0, &PIXELS[..]),
            (1025, 1, &PIXELS[..]),
            (u32::MAX, u32::MAX, &PIXELS[..]),
            (2, 2, &PIXELS[..15]),
            (1, 1, &PIXELS[..]),
            (1, 1, &PIXELS[..0]),
        ] {
            assert!(
                linear_repeat(
                    &MipLevel {
                        width,
                        height,
                        texels
                    },
                    [0., 0.]
                )
                .is_err()
            );
        }
        let mut bytes = PIXELS;
        bytes[15] = 0;
        assert!(
            linear_repeat(
                &MipLevel {
                    width: 2,
                    height: 2,
                    texels: &bytes
                },
                [0., 0.]
            )
            .is_err()
        );
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            64f64.next_up(),
        ] {
            for uv in [[value, 0.], [0., value]] {
                assert!(linear_repeat(&quad(), uv).is_err());
            }
        }
    }
}
