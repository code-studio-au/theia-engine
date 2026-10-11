//! CPU oracle for projected vertical-error selection over a shared world AABB.
//! Both continuous heightfield surfaces must lie inside these bounds; their
//! corresponding points differ only in Z by at most the admitted error.
use crate::{AssetError, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Fine,
    Coarse,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FineReason {
    ClipBoundary,
    UnboundedProjection,
    PixelBudget,
    Hysteresis,
}
#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub level: Level,
    pub bound_pixels: Option<[f64; 2]>,
    pub reason: Option<FineReason>,
}

// Finite f32*f32 products are exactly representable in f64. Outward rounding
// bounds each subsequent sum; no fast-math or shader-rounding guarantee is made.
fn interval(row: [f32; 4], point: [f32; 3]) -> [f64; 2] {
    let mut bounds = [f64::from(row[3]); 2];
    for axis in 0..3 {
        let product = f64::from(row[axis]) * f64::from(point[axis]);
        bounds[0] = (bounds[0] + product).next_down();
        bounds[1] = (bounds[1] + product).next_up();
    }
    bounds
}
fn upper_mul(a: f64, b: f64) -> f64 {
    (a * b).next_up()
}
fn upper_div(a: f64, b: f64) -> f64 {
    (a / b).next_up()
}
fn fine(reason: FineReason, bound_pixels: Option<[f64; 2]>) -> Selection {
    Selection {
        level: Level::Fine,
        bound_pixels,
        reason: Some(reason),
    }
}

/// Vulkan clip Z in [0,W]. Source/target geometry and the supplied camera matrix
/// are represented f32 values. This bounds mathematical projected correspondence,
/// not shader rounding, depth/disocclusion, normals, material or temporal error.
/// AABB must enclose both surfaces. This function does not verify the cooker proof.
pub fn select(
    bounds: [[f32; 3]; 2],
    vertical_error: f32,
    rows: [[f32; 4]; 4],
    extent: [u32; 2],
    pixel_budget: f64,
) -> Result<Selection> {
    if bounds
        .iter()
        .flatten()
        .chain(rows.iter().flatten())
        .any(|v| !v.is_finite())
        || !vertical_error.is_finite()
        || vertical_error < 0.
        || !pixel_budget.is_finite()
        || pixel_budget < 0.
        || extent.contains(&0)
        || (0..3).any(|axis| bounds[0][axis] > bounds[1][axis])
    {
        return Err(AssetError("invalid projected heightfield LOD profile"));
    }
    let mut minimum_w = f64::INFINITY;
    let mut maximum_xy = [0f64; 2];
    let mut clipped = false;
    for corner in 0..8 {
        let point = std::array::from_fn(|axis| bounds[(corner >> axis) & 1][axis]);
        let w = interval(rows[3], point);
        let z = interval(rows[2], point);
        minimum_w = minimum_w.min(w[0]);
        // Guard the whole common box against positive-W and Vulkan near/far
        // boundaries. A conservative box crossing requests the fine baseline.
        clipped |= w[0] <= 0. || z[0] <= 0. || (w[0] - z[1]).next_down() <= 0.;
        for axis in 0..2 {
            let range = interval(rows[axis], point);
            maximum_xy[axis] = maximum_xy[axis].max(range[0].abs()).max(range[1].abs());
        }
    }
    if clipped {
        return Ok(fine(FineReason::ClipBoundary, None));
    }
    if vertical_error == 0. {
        return Ok(Selection {
            level: Level::Coarse,
            bound_pixels: Some([0.; 2]),
            reason: None,
        });
    }
    let error = f64::from(vertical_error);
    let dw = f64::from(rows[3][2]).abs();
    let pixels = std::array::from_fn(|axis| {
        // For P,Q inside the box with Q-P=(0,0,dz), both denominators
        // are >= minimum_w. Bound |(a+da)/(w+dw)-a/w| by
        // E*(|row_axis.z| + max|clip_axis|*|row_w.z|/minW)/minW.
        let term = upper_div(upper_mul(maximum_xy[axis], dw), minimum_w);
        let numerator = (f64::from(rows[axis][2]).abs() + term).next_up();
        let ndc = upper_div(upper_mul(error, numerator), minimum_w);
        upper_mul(ndc, f64::from(extent[axis]) * 0.5)
    });
    if pixels.iter().any(|v| !v.is_finite()) {
        return Ok(fine(FineReason::UnboundedProjection, None));
    }
    Ok(if pixels.iter().all(|v| *v <= pixel_budget) {
        Selection {
            level: Level::Coarse,
            bound_pixels: Some(pixels),
            reason: None,
        }
    } else {
        fine(FineReason::PixelBudget, Some(pixels))
    })
}

/// Delay fine-to-coarse transitions until the projected bound fits a stricter
/// entry budget. Coarse-to-fine uses the original hard budget, so hysteresis
/// never authorizes a previously coarse page above the caller's error limit.
/// The caller owns history for one view/page pair and starts/resets it to Fine
/// when that context changes. This function neither caches nor authenticates it.
pub fn select_hysteresis(
    bounds: [[f32; 3]; 2],
    vertical_error: f32,
    rows: [[f32; 4]; 4],
    extent: [u32; 2],
    pixel_budget: f64,
    previous: Level,
    entry_fraction: f64,
) -> Result<Selection> {
    if !entry_fraction.is_finite() || !(0.0..=1.0).contains(&entry_fraction) {
        return Err(AssetError("invalid LOD hysteresis entry fraction"));
    }
    let selection = select(bounds, vertical_error, rows, extent, pixel_budget)?;
    if previous == Level::Coarse || selection.level == Level::Fine || entry_fraction == 0. {
        return Ok(selection);
    }
    // Round the positive entry threshold inward; the hard-budget oracle still
    // decides safety before history influences a transition.
    let entry_factor = (1. - entry_fraction).next_down().max(0.);
    let entry_budget = (entry_factor * pixel_budget).next_down().max(0.);
    Ok(
        if selection
            .bound_pixels
            .unwrap()
            .iter()
            .all(|v| *v <= entry_budget)
        {
            selection
        } else {
            fine(FineReason::Hysteresis, selection.bound_pixels)
        },
    )
}

/// Convert a finite nonnegative producer upper bound to f32 without rounding down.
pub fn error_f32(upper: f64) -> Result<f32> {
    if !upper.is_finite() || upper < 0. || upper > f64::from(f32::MAX) {
        return Err(AssetError("invalid represented vertical error bound"));
    }
    let value = upper as f32;
    Ok(if f64::from(value) < upper {
        value.next_up()
    } else {
        value
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const BOX: [[f32; 3]; 2] = [[-1., -1., -1.], [1., 1., 1.]];
    fn ortho() -> [[f32; 4]; 4] {
        [
            [1., 0., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 0.5],
            [0., 0., 0., 1.],
        ]
    }
    #[test]
    fn hysteresis_delays_coarsening_but_never_delays_hard_limit_refinement() {
        let mut previous = Level::Fine;
        let expected = [
            Level::Fine,
            Level::Fine,
            Level::Coarse,
            Level::Coarse,
            Level::Coarse,
            Level::Fine,
            Level::Fine,
            Level::Coarse,
        ];
        for (error, want) in [0.9, 0.81, 0.79, 0.9, 0.99, 1.01, 0.9, 0.79]
            .into_iter()
            .zip(expected)
        {
            let selection =
                select_hysteresis(BOX, error, ortho(), [200; 2], 100., previous, 0.2).unwrap();
            assert_eq!(selection.level, want);
            if selection.level == Level::Coarse {
                assert!(selection.bound_pixels.unwrap().iter().all(|v| *v <= 100.));
            }
            previous = selection.level;
        }
        // A new view/page context starts fine, even inside the dead band.
        assert_eq!(
            select_hysteresis(BOX, 0.9, ortho(), [200; 2], 100., Level::Fine, 0.2)
                .unwrap()
                .reason,
            Some(FineReason::Hysteresis)
        );
    }
    #[test]
    fn hysteresis_preserves_guards_zero_error_and_stateless_policy() {
        for previous in [Level::Fine, Level::Coarse] {
            assert_eq!(
                select_hysteresis(BOX, 0., ortho(), [200; 2], 0., previous, 1.)
                    .unwrap()
                    .level,
                Level::Coarse
            );
            let mut rows = ortho();
            rows[2] = [0., 0., 1., 0.];
            assert_eq!(
                select_hysteresis(BOX, 0., rows, [200; 2], 1e9, previous, 0.2)
                    .unwrap()
                    .reason,
                Some(FineReason::ClipBoundary)
            );
            for fraction in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
                assert!(
                    select_hysteresis(BOX, 0.1, ortho(), [200; 2], 100., previous, fraction)
                        .is_err()
                );
            }
            for error in [0., 0.8, 1., 1.1] {
                let ordinary = select(BOX, error, ortho(), [200; 2], 100.).unwrap();
                let disabled =
                    select_hysteresis(BOX, error, ortho(), [200; 2], 100., previous, 0.).unwrap();
                assert_eq!(ordinary.level, disabled.level);
                assert_eq!(ordinary.reason, disabled.reason);
                assert_eq!(ordinary.bound_pixels, disabled.bound_pixels);
            }
            assert!(select_hysteresis(BOX, 0.1, ortho(), [0, 1], 100., previous, 0.2).is_err());
        }
    }
    #[test]
    fn generated_hysteresis_sequence_matches_independent_orthographic_state_machine() {
        let mut previous = Level::Fine;
        let mut coarse = false;
        let mut transitions = 0;
        let mut seed = 719u64;
        for _ in 0..10000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            // Avoid exact entry/exit ties; oracle outward rounding is conservative.
            let error = ((seed >> 32) % 2000) as f32 / 1000. + 0.00025;
            let displacement = f64::from(error) * 100.;
            coarse = if coarse {
                displacement <= 100.
            } else {
                displacement <= 80.
            };
            let selected =
                select_hysteresis(BOX, error, ortho(), [200; 2], 100., previous, 0.2).unwrap();
            assert_eq!(selected.level == Level::Coarse, coarse);
            transitions += usize::from(selected.level != previous);
            previous = selected.level;
        }
        assert!(transitions > 1000);
    }
    #[test]
    fn analytic_orthographic_displacement_and_zero_error() {
        let result = select(BOX, 0.125, ortho(), [100, 200], 12.6).unwrap();
        assert_eq!(result.level, Level::Coarse);
        let pixels = result.bound_pixels.unwrap();
        assert!(pixels[1] >= 12.5 && pixels[1] < 12.500000000001);
        assert_eq!(
            select(BOX, 0.125, ortho(), [100, 200], 12.4)
                .unwrap()
                .reason,
            Some(FineReason::PixelBudget)
        );
        let zero = select(BOX, 0., ortho(), [100, 200], 0.).unwrap();
        assert_eq!(zero.level, Level::Coarse);
        assert_eq!(zero.bound_pixels, Some([0.; 2]));
    }
    #[test]
    fn perspective_denominator_changes_require_error_even_without_axis_z_term() {
        let rows = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 0., 0.5],
            [0., 0., 0.5, 2.],
        ];
        let result = select(BOX, 0.25, rows, [200, 200], 100.).unwrap();
        let bound = result.bound_pixels.unwrap();
        // Independent point projections: y=1,z=-1 -> y=1,z=-.75.
        let actual = (1. / 1.5 - 1. / 1.625) * 100.;
        assert!(bound[1] >= actual && actual > 0.);
        assert_eq!(
            select(BOX, 0.25, rows, [200, 200], actual / 2.)
                .unwrap()
                .level,
            Level::Fine
        );
    }
    #[test]
    fn clip_boundaries_and_invalid_profiles_keep_fine_or_fail_closed() {
        for row in [[0., 0., 1., 0.], [0., 0., 1., 0.5], [0., 0., 0., 1.]] {
            let mut rows = ortho();
            rows[2] = row;
            assert_eq!(
                select(BOX, 0.1, rows, [100, 100], 100.).unwrap().reason,
                Some(FineReason::ClipBoundary)
            );
        }
        let mut rows = ortho();
        rows[3] = [0., 0., 1., 0.];
        assert_eq!(
            select(BOX, 0.1, rows, [100, 100], 100.).unwrap().reason,
            Some(FineReason::ClipBoundary)
        );
        for error in [-1., f32::NAN, f32::INFINITY] {
            assert!(select(BOX, error, ortho(), [1, 1], 1.).is_err());
        }
        assert!(select(BOX, 0.1, ortho(), [0, 1], 1.).is_err());
        assert!(select(BOX, 0.1, ortho(), [1, 1], f64::NAN).is_err());
        let mut bad = BOX;
        bad[0][0] = 2.;
        assert!(select(bad, 0.1, ortho(), [1, 1], 1.).is_err());
    }
    #[test]
    fn finite_extreme_projection_overflow_keeps_the_fine_baseline() {
        let tiny = f32::from_bits(1);
        let bounds = [[tiny, f32::MAX, 0.]; 2];
        let rows = [
            [0., f32::MAX, 0., 0.],
            [0., 0., 0., 0.],
            [tiny, 0., 0., 0.],
            [tiny * 2., 0., f32::MAX, 0.],
        ];
        let result = select(bounds, f32::MAX, rows, [u32::MAX; 2], 1.).unwrap();
        assert_eq!(result.reason, Some(FineReason::UnboundedProjection));
        assert_eq!(result.bound_pixels, None);
    }
    #[test]
    fn producer_bound_conversion_never_rounds_down() {
        for upper in [
            0.,
            f64::from(f32::MIN_POSITIVE) / 100.,
            0.004977348517231262,
            0.012254270974889582,
            f64::from(f32::MAX),
        ] {
            assert!(f64::from(error_f32(upper).unwrap()) >= upper);
        }
        for upper in [-1., f64::NAN, f64::INFINITY, f64::MAX] {
            assert!(error_f32(upper).is_err());
        }
    }
    #[test]
    fn randomized_corresponding_points_stay_within_bound_and_extent_scales() {
        let mut seed = 719u64;
        let mut sample = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 32) % 1001) as f32 / 1000.
        };
        for _ in 0..100 {
            let rows = [
                [sample(), sample(), sample() - 0.5, 0.],
                [sample(), sample(), sample() - 0.5, 0.],
                [0., 0., 0., 0.5],
                [0., 0., (sample() - 0.5) * 0.25, 2.],
            ];
            let error = sample() * 0.25;
            let pixels = select(BOX, error, rows, [960, 540], 1e9)
                .unwrap()
                .bound_pixels
                .unwrap();
            let doubled = select(BOX, error, rows, [1920, 1080], 1e9)
                .unwrap()
                .bound_pixels
                .unwrap();
            assert_eq!(doubled, pixels.map(|v| v * 2.));
            for _ in 0..30 {
                let p = [
                    f64::from(sample() * 2. - 1.),
                    f64::from(sample() * 2. - 1.),
                    f64::from(sample() * 2. - 1.),
                ];
                let mut q = p;
                q[2] = (p[2] + f64::from((sample() * 2. - 1.) * error)).clamp(-1., 1.);
                let project = |point: [f64; 3]| {
                    rows.map(|row| {
                        f64::from(row[3])
                            + (0..3).map(|i| f64::from(row[i]) * point[i]).sum::<f64>()
                    })
                };
                let a = project(p);
                let b = project(q);
                for axis in 0..2 {
                    let actual = (a[axis] / a[3] - b[axis] / b[3]).abs() * [480., 270.][axis];
                    assert!(
                        actual <= pixels[axis],
                        "actual={actual}, upper={}",
                        pixels[axis]
                    );
                }
            }
        }
    }
}
