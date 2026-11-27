//! CPU reference for analytic perspective UV gradients and a bounded isotropic mip policy.
//! This is not a GPU rounding proof, filtering-quality metric or clipped raster oracle.
use crate::{AssetError, Result, texture::MAX_DIMENSION};

#[derive(Clone, Copy, Debug)]
pub struct Differential {
    pub uv: [f64; 2],
    /// UV displacement per framebuffer pixel, before texture wrapping.
    pub dx: [f64; 2],
    pub dy: [f64; 2],
}
#[derive(Clone, Copy, Debug)]
pub struct Footprint {
    pub texels_per_pixel: f64,
    pub mip_level: u32,
}

/// Evaluate one triangle's rational UV interpolation and analytic pixel Jacobian.
/// Clip/UV inputs are represented f32. Pixel coordinates include the center offset.
/// The caller identifies the visible triangle; this function does not test coverage.
/// The bounded profile requires wholly near/far-unclipped, positive-W vertices.
pub fn differential(
    clip: [[f32; 4]; 3],
    uv: [[f32; 2]; 3],
    pixel: [f64; 2],
    extent: [u32; 2],
) -> Result<Differential> {
    if clip
        .iter()
        .flatten()
        .chain(uv.iter().flatten())
        .any(|v| !v.is_finite() || v.abs() > 64.)
        || extent.iter().any(|&v| !(1..=16384).contains(&v))
        || pixel
            .iter()
            .enumerate()
            .any(|(axis, v)| !v.is_finite() || *v < 0. || *v >= f64::from(extent[axis]))
        || clip.iter().any(|v| v[3] < 1e-4 || v[2] < 0. || v[2] > v[3])
    {
        return Err(AssetError("unsupported UV differential profile"));
    }
    let screen = clip.map(|v| {
        [
            f64::from(v[0]) / f64::from(v[3]),
            f64::from(v[1]) / f64::from(v[3]),
        ]
    });
    let [a, b, c] = screen;
    let area = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if !area.is_finite() || area.abs() < 1e-10 {
        return Err(AssetError("degenerate UV differential projection"));
    }
    let point =
        std::array::from_fn::<_, 2, _>(|axis| pixel[axis] / f64::from(extent[axis]) * 2. - 1.);
    let u = ((b[1] - c[1]) * (point[0] - c[0]) + (c[0] - b[0]) * (point[1] - c[1])) / area;
    let v = ((c[1] - a[1]) * (point[0] - c[0]) + (a[0] - c[0]) * (point[1] - c[1])) / area;
    let weights = [u, v, 1. - u - v];
    let dx = [
        (b[1] - c[1]) / area * 2. / f64::from(extent[0]),
        (c[1] - a[1]) / area * 2. / f64::from(extent[0]),
    ];
    let dy = [
        (c[0] - b[0]) / area * 2. / f64::from(extent[1]),
        (a[0] - c[0]) / area * 2. / f64::from(extent[1]),
    ];
    let derivatives = [
        [dx[0], dx[1], -dx[0] - dx[1]],
        [dy[0], dy[1], -dy[0] - dy[1]],
    ];
    let reciprocal = clip.map(|v| 1. / f64::from(v[3]));
    let denominator: f64 = (0..3).map(|i| weights[i] * reciprocal[i]).sum();
    if !denominator.is_finite() || denominator <= 1e-10 {
        return Err(AssetError("unbounded UV differential denominator"));
    }
    let numerator: [f64; 2] = std::array::from_fn(|axis| {
        (0..3)
            .map(|i| weights[i] * reciprocal[i] * f64::from(uv[i][axis]))
            .sum()
    });
    let value = numerator.map(|v| v / denominator);
    let gradients: [[f64; 2]; 2] = derivatives.map(|derivative| {
        let denominator_derivative: f64 = (0..3).map(|i| derivative[i] * reciprocal[i]).sum();
        std::array::from_fn(|axis| {
            let numerator_derivative: f64 = (0..3)
                .map(|i| derivative[i] * reciprocal[i] * f64::from(uv[i][axis]))
                .sum();
            (numerator_derivative - value[axis] * denominator_derivative) / denominator
        })
    });
    if value
        .iter()
        .chain(gradients.iter().flatten())
        .any(|v| !v.is_finite())
    {
        return Err(AssetError("nonfinite UV differential result"));
    }
    Ok(Differential {
        uv: value,
        dx: gradients[0],
        dy: gradients[1],
    })
}

/// Prototype policy: choose ceil(log2(max Jacobian column length in base texels)),
/// clamped to the complete resident chain. Exact powers of two select that level;
/// comparisons avoid a log2 rounding dependency. This estimates an isotropic
/// footprint, not anisotropic filtering or a guarantee of image reconstruction.
pub fn choose_mip(
    gradient: Differential,
    texture_extent: [u32; 2],
    levels: u32,
) -> Result<Footprint> {
    if texture_extent
        .iter()
        .any(|&v| !(1..=MAX_DIMENSION).contains(&v))
        || levels != u32::BITS - texture_extent[0].max(texture_extent[1]).leading_zeros()
        || gradient
            .uv
            .iter()
            .chain(gradient.dx.iter())
            .chain(gradient.dy.iter())
            .any(|v| !v.is_finite())
    {
        return Err(AssetError("invalid texture footprint profile"));
    }
    let rho = [gradient.dx, gradient.dy]
        .map(|d| (d[0] * f64::from(texture_extent[0])).hypot(d[1] * f64::from(texture_extent[1])))
        .into_iter()
        .fold(0., f64::max);
    if !rho.is_finite() {
        return Err(AssetError("texture footprint overflow"));
    }
    let mut mip = 0;
    while mip + 1 < levels && rho > f64::from(1u32 << mip) {
        mip += 1;
    }
    Ok(Footprint {
        texels_per_pixel: rho,
        mip_level: mip,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const CLIP: [[f32; 4]; 3] = [[-1., -1., 0.5, 1.], [1., -1., 0.5, 1.], [-1., 1., 0.5, 1.]];
    const UV: [[f32; 2]; 3] = [[0., 0.], [1., 0.], [0., 1.]];
    #[test]
    fn affine_uv_gradient_is_per_pixel_and_extent_scaling_is_independent() {
        let g = differential(CLIP, UV, [25., 50.], [100, 200]).unwrap();
        assert_eq!(g.uv, [0.25, 0.25]);
        assert_eq!(g.dx, [0.01, 0.]);
        assert_eq!(g.dy, [0., 0.005]);
        let doubled = differential(CLIP, UV, [50., 100.], [200, 400]).unwrap();
        assert_eq!(doubled.uv, g.uv);
        assert_eq!(doubled.dx, g.dx.map(|v| v / 2.));
        assert_eq!(doubled.dy, g.dy.map(|v| v / 2.));
        let footprint = choose_mip(g, [1024, 512], 11).unwrap();
        assert!((footprint.texels_per_pixel - 10.24).abs() < 1e-12);
        assert_eq!(footprint.mip_level, 4);
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let permuted = differential(
                order.map(|i| CLIP[i]),
                order.map(|i| UV[i]),
                [25., 50.],
                [100, 200],
            )
            .unwrap();
            assert_eq!(permuted.uv, g.uv);
            assert_eq!(permuted.dx, g.dx);
            assert_eq!(permuted.dy, g.dy);
        }
    }
    #[test]
    fn perspective_gradients_match_independent_finite_differences_before_wrapping() {
        let mut seed = 719u64;
        let mut sample = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 32) % 1001) as f32 / 1000.
        };
        for _ in 0..1000 {
            let w = [
                0.5 + sample() * 3.,
                0.5 + sample() * 3.,
                0.5 + sample() * 3.,
            ];
            let clip = [
                [-w[0], -w[0], w[0] * 0.5, w[0]],
                [w[1], -w[1], w[1] * 0.5, w[1]],
                [-w[2], w[2], w[2] * 0.5, w[2]],
            ];
            let uv = std::array::from_fn(|_| [sample() * 4. - 2., sample() * 4. - 2.]);
            let p = [
                10. + f64::from(sample()) * 20.,
                20. + f64::from(sample()) * 40.,
            ];
            let g = differential(clip, uv, p, [100, 200]).unwrap();
            // Right-triangle coordinates yield direct affine lambdas; independent
            // interpolation does not use the generic area/Jacobian implementation.
            let evaluate = |p: [f64; 2]| {
                let lambda = [1. - p[0] / 100. - p[1] / 200., p[0] / 100., p[1] / 200.];
                let r: f64 = (0..3).map(|i| lambda[i] / f64::from(w[i])).sum();
                std::array::from_fn::<_, 2, _>(|axis| {
                    (0..3)
                        .map(|i| lambda[i] * f64::from(uv[i][axis]) / f64::from(w[i]))
                        .sum::<f64>()
                        / r
                })
            };
            for axis in 0..2 {
                assert!((g.uv[axis] - evaluate(p)[axis]).abs() < 1e-12);
            }
            for (direction, want) in [g.dx, g.dy].into_iter().enumerate() {
                let mut a = p;
                let mut b = p;
                a[direction] -= 0.001;
                b[direction] += 0.001;
                let a = evaluate(a);
                let b = evaluate(b);
                for axis in 0..2 {
                    assert!((want[axis] - (b[axis] - a[axis]) / 0.002).abs() < 1e-8);
                }
            }
        }
    }
    #[test]
    fn exact_power_thresholds_clamping_and_invalid_inputs_are_explicit() {
        for (rho, want) in [
            (0., 0),
            (1., 0),
            (1f64.next_up(), 1),
            (2., 1),
            (2f64.next_up(), 2),
            (16., 4),
            (2048., 10),
        ] {
            let g = Differential {
                uv: [-1., 2.],
                dx: [rho / 1024., 0.],
                dy: [0.; 2],
            };
            assert_eq!(choose_mip(g, [1024, 1], 11).unwrap().mip_level, want);
        }
        let g = differential(CLIP, UV, [25., 50.], [100, 200]).unwrap();
        for (extent, levels) in [([0, 1], 1), ([1025, 1], 11), ([1024, 1], 10), ([1, 1], 0)] {
            assert!(choose_mip(g, extent, levels).is_err());
        }
        for bad in [f64::NAN, f64::INFINITY, f64::MAX] {
            assert!(choose_mip(Differential { dx: [bad, 0.], ..g }, [1024, 1], 11).is_err());
        }
        let mut bad = CLIP;
        bad[0][3] = 0.;
        assert!(differential(bad, UV, [25., 50.], [100, 200]).is_err());
        bad = CLIP;
        bad[0][2] = -0.1;
        assert!(differential(bad, UV, [25., 50.], [100, 200]).is_err());
        bad = CLIP;
        bad[0] = bad[1];
        assert!(differential(bad, UV, [25., 50.], [100, 200]).is_err());
        assert!(differential(CLIP, UV, [f64::NAN, 0.], [100, 200]).is_err());
        assert!(differential(CLIP, UV, [100., 0.], [100, 200]).is_err());
        assert!(differential(CLIP, UV, [25., 50.], [0, 200]).is_err());
    }
    #[test]
    fn repeat_seams_are_differentiated_before_wrapping_and_constants_choose_base() {
        let uv = [[0.9, -0.1], [1.1, -0.1], [0.9, 0.1]];
        let gradient = differential(CLIP, uv, [50., 50.], [100, 200]).unwrap();
        assert!((gradient.uv[0] - 1.).abs() < 1e-7);
        assert_eq!(choose_mip(gradient, [1024, 1], 11).unwrap().mip_level, 2);
        let wrapped = uv.map(|value| value.map(|v| v - v.floor()));
        let incorrect = differential(CLIP, wrapped, [50., 50.], [100, 200]).unwrap();
        assert_eq!(choose_mip(incorrect, [1024, 1], 11).unwrap().mip_level, 4);
        let constant = differential(CLIP, [[3., -2.]; 3], [50., 50.], [100, 200]).unwrap();
        assert_eq!(choose_mip(constant, [1024, 1], 11).unwrap().mip_level, 0);
    }
}
