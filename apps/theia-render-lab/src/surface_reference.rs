//! CPU raster/material oracle for M3's fixed, w=1 two-triangle fixture.
#[derive(Debug)]
pub struct Sample {
    pub identity: [u32; 2],
    pub depth: f32,
    pub color: [f32; 3],
}
fn barycentric(point: [f32; 2], scale: f32) -> [f32; 3] {
    let p = [point[0] / scale, point[1] / scale];
    let top = (0.6 - p[1]) / 1.2;
    let right = (1.0 - top + p[0] / 0.7) * 0.5;
    [1.0 - top - right, right, top]
}
pub fn sample(point: [f32; 2], count: u32) -> Sample {
    assert!(count <= 2);
    let mut result = Sample {
        identity: [0, 0],
        depth: 1.0,
        color: [0.025, 0.07, 0.12],
    };
    for instance in 0..count {
        let scale = if instance == 0 { 0.5 } else { 1.0 };
        let depth = if instance == 0 { 0.25 } else { 0.75 };
        let bary = barycentric(point, scale);
        if bary.iter().all(|weight| *weight >= 0.0) && depth < result.depth {
            let tint = if instance == 0 {
                [1.0; 3]
            } else {
                [89.0 / 255.0, 153.0 / 255.0, 1.0]
            };
            result = Sample {
                identity: [instance + 1, 0],
                depth,
                color: std::array::from_fn(|i| bary[i] * tint[i]),
            };
        }
    }
    result
}
pub fn print_pixel(x: u32, y: u32, width: u32, height: u32, count: u32) {
    let point = [
        (x as f32 + 0.5) / width as f32 * 2.0 - 1.0,
        (y as f32 + 0.5) / height as f32 * 2.0 - 1.0,
    ];
    let result = sample(point, count);
    println!(
        "{{\"identity\":{:?},\"depth\":{},\"color\":{:?}}}",
        result.identity, result.depth, result.color
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlap_selects_nearer_identity_and_reconstructs_barycentrics() {
        let result = sample([0.0, 0.0], 2);
        assert_eq!(result.identity, [1, 0]);
        assert_eq!(result.depth, 0.25);
        assert_eq!(result.color, [0.25, 0.25, 0.5]);
        let outer = sample([0.45, 0.3], 2);
        assert_eq!(outer.identity, [2, 0]);
        assert_eq!(outer.depth, 0.75);
        assert_eq!(sample([0.45, 0.3], 1).identity, [0, 0]);
        assert_eq!(sample([0.0, 0.0], 0).depth, 1.0);
    }
    #[test]
    fn vertex_weights_and_background_are_explicit() {
        for (point, want) in [
            ([-0.7, 0.6], [1.0, 0.0, 0.0]),
            ([0.7, 0.6], [0.0, 1.0, 0.0]),
            ([0.0, -0.6], [0.0, 0.0, 1.0]),
        ] {
            assert_eq!(barycentric(point, 1.0), want);
        }
        assert_eq!(sample([0.95, -0.95], 2).color, [0.025, 0.07, 0.12]);
    }
}
