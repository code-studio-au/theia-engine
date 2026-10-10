//! Bounded perspective/depth CPU oracle. Fixed camera rig, static mesh positions.
use crate::Result;
use theia_assets::Mesh;
pub const EYE: [f32; 3] = [0., -1.7, 1.5];
pub const TARGET: [f32; 3] = [0., 0., 0.5];
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn normal(v: [f32; 3]) -> [f32; 3] {
    let size = dot(v, v).sqrt();
    v.map(|v| v / size)
}
pub fn rows(width: u32, height: u32) -> Result<[[f32; 4]; 4]> {
    if width == 0 || height == 0 {
        return Err("Camera requires nonzero extent".into());
    }
    let forward = normal(std::array::from_fn(|i| TARGET[i] - EYE[i]));
    let right = normal(cross(forward, [0., 0., 1.]));
    let up = cross(right, forward);
    let z = forward.map(|v| -v);
    let s = 1.0 / (60f32.to_radians() * 0.5).tan();
    let near = 0.1;
    let far = 10.;
    let a = far / (near - far);
    let b = far * near / (near - far);
    Ok([
        [
            right[0] * s * height as f32 / width as f32,
            right[1] * s * height as f32 / width as f32,
            right[2] * s * height as f32 / width as f32,
            -dot(right, EYE) * s * height as f32 / width as f32,
        ],
        [-up[0] * s, -up[1] * s, -up[2] * s, dot(up, EYE) * s],
        [z[0] * a, z[1] * a, z[2] * a, -dot(z, EYE) * a + b],
        [-z[0], -z[1], -z[2], dot(z, EYE)],
    ])
}
fn project(rows: [[f32; 4]; 4], position: [f32; 3]) -> [f32; 4] {
    rows.map(|row| row[0] * position[0] + row[1] * position[1] + row[2] * position[2] + row[3])
}
pub struct Pixel {
    pub color: [f32; 3],
    pub depth: f32,
    pub covered: bool,
}
pub fn pixel(mesh: &Mesh, x: u32, y: u32, width: u32, height: u32, scenario: u32) -> Result<Pixel> {
    if x >= width || y >= height || scenario > 2 {
        return Err("Invalid camera reference pixel/scenario".into());
    }
    let rows = rows(width, height)?;
    let point = [
        (x as f32 + 0.5) / width as f32 * 2. - 1.,
        (y as f32 + 0.5) / height as f32 * 2. - 1.,
    ];
    let count = match scenario {
        0 => 0,
        1 => 1,
        _ => mesh.meshlets.len(),
    };
    let mut result = Pixel {
        color: [0.025, 0.07, 0.12],
        depth: 1.,
        covered: false,
    };
    for meshlet in mesh.meshlets.iter().take(count) {
        for triangle in 0..meshlet.triangle_count as usize {
            let mut positions = [[0.; 3]; 3];
            let mut clip = [[0.; 4]; 3];
            let mut screen = [[0.; 2]; 3];
            for i in 0..3 {
                let local =
                    mesh.triangles[meshlet.triangle_offset as usize + triangle * 3 + i] as usize;
                positions[i] = mesh.positions
                    [mesh.vertex_refs[meshlet.vertex_offset as usize + local] as usize];
                clip[i] = project(rows, positions[i]);
                // This oracle is deliberately bounded to wholly unclipped positive-w triangles.
                if clip[i][3] <= 0. || clip[i][2] < 0. || clip[i][2] > clip[i][3] {
                    return Err(
                        "Camera reference does not implement near/far triangle clipping".into(),
                    );
                }
                screen[i] = [clip[i][0] / clip[i][3], clip[i][1] / clip[i][3]];
            }
            let [a, b, c] = screen;
            let denominator = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if denominator.abs() < 1e-10 {
                continue;
            }
            let u = ((b[1] - c[1]) * (point[0] - c[0]) + (c[0] - b[0]) * (point[1] - c[1]))
                / denominator;
            let v = ((c[1] - a[1]) * (point[0] - c[0]) + (a[0] - c[0]) * (point[1] - c[1]))
                / denominator;
            let weights = [u, v, 1. - u - v];
            if weights.iter().any(|v| *v < 0.) {
                continue;
            }
            let depth = (0..3)
                .map(|i| weights[i] * clip[i][2] / clip[i][3])
                .sum::<f32>();
            if depth >= result.depth {
                continue;
            }
            let reciprocal = (0..3).map(|i| weights[i] / clip[i][3]).sum::<f32>();
            let world: [f32; 3] = std::array::from_fn(|axis| {
                (0..3)
                    .map(|i| weights[i] * positions[i][axis] / clip[i][3])
                    .sum::<f32>()
                    / reciprocal
            });
            result = Pixel {
                color: [world[0] * 0.5 + 0.5, world[1] * 0.5 + 0.5, 0.5],
                depth,
                covered: true,
            };
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn camera_maps_target_and_vulkan_near_far_depth() {
        let rows = rows(960, 540).unwrap();
        let target = project(rows, TARGET);
        assert!(target[0].abs() < 1e-6 && target[1].abs() < 1e-6);
        let forward = normal(std::array::from_fn(|i| TARGET[i] - EYE[i]));
        for (distance, depth) in [(0.1, 0.), (10., 1.)] {
            let clip = project(
                rows,
                std::array::from_fn(|i| EYE[i] + forward[i] * distance),
            );
            assert!(clip[3] > 0.);
            // f32 view translation cancellation is amplified by division at the 0.1 near plane.
            assert!(
                (clip[2] / clip[3] - depth).abs() < 4e-6,
                "distance {distance}, clip {clip:?}, depth {depth}"
            );
        }
    }
    #[test]
    fn aspect_and_y_orientation_are_explicit() {
        let narrow = project(rows(540, 540).unwrap(), [0.5, 0., 0.5]);
        let wide = project(rows(1080, 540).unwrap(), [0.5, 0., 0.5]);
        assert!(narrow[0] > 0.);
        assert!((wide[0] * 2. - narrow[0]).abs() < 1e-6);
        assert!(project(rows(960, 540).unwrap(), [0., 0., 1.])[1] < 0.);
        assert!(rows(0, 540).is_err());
        assert!(rows(960, 0).is_err());
    }
    #[test]
    fn perspective_center_ray_selects_near_plane_before_later_far_triangles() {
        let positions = [0.65, 0.35]
            .into_iter()
            .flat_map(|z| {
                [
                    [-0.6, -0.6, z],
                    [0.6, -0.6, z],
                    [0.6, 0.6, z],
                    [-0.6, 0.6, z],
                ]
            })
            .collect();
        let mesh = Mesh {
            content_id: [0; 32],
            provenance: String::new(),
            positions,
            meshlets: vec![theia_assets::Meshlet {
                vertex_offset: 0,
                triangle_offset: 0,
                vertex_count: 8,
                triangle_count: 4,
            }],
            vertex_refs: (0..8).collect(),
            triangles: vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7],
        };
        let pixel = pixel(&mesh, 0, 0, 1, 1, 2).unwrap();
        // Independent center ray: eye + 0.85*(target-eye) reaches z=0.65 at y=-0.255.
        let distance = 3.89f64.sqrt() * 0.85;
        let expected_depth = 10. / 9.9 - 1. / (9.9 * distance);
        assert!(pixel.covered);
        assert!((pixel.color[0] - 0.5).abs() < 1e-6);
        assert!((pixel.color[1] - 0.3725).abs() < 1e-6);
        assert!((pixel.depth as f64 - expected_depth).abs() < 2e-6);
    }
}
