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
    rig_rows(width, height, EYE, TARGET, 60.)
}
pub fn view_rows(width: u32, height: u32, view: u32) -> Result<[[f32; 4]; 4]> {
    let (shift, fov) = match view {
        0 => (20., 60.),
        1 => (-0.65, 20.),
        2 => (0., 60.),
        _ => return Err("Frustum view must be 0..2".into()),
    };
    let mut eye = EYE;
    eye[0] += shift;
    let mut target = TARGET;
    target[0] += shift;
    rig_rows(width, height, eye, target, fov)
}
fn rig_rows(
    width: u32,
    height: u32,
    eye: [f32; 3],
    target: [f32; 3],
    fov: f32,
) -> Result<[[f32; 4]; 4]> {
    if width == 0 || height == 0 {
        return Err("Camera requires nonzero extent".into());
    }
    let forward = normal(std::array::from_fn(|i| target[i] - eye[i]));
    let right = normal(cross(forward, [0., 0., 1.]));
    let up = cross(right, forward);
    let z = forward.map(|v| -v);
    let s = 1.0 / (fov.to_radians() * 0.5).tan();
    let near = 0.1;
    let far = 10.;
    let a = far / (near - far);
    let b = far * near / (near - far);
    Ok([
        [
            right[0] * s * height as f32 / width as f32,
            right[1] * s * height as f32 / width as f32,
            right[2] * s * height as f32 / width as f32,
            -dot(right, eye) * s * height as f32 / width as f32,
        ],
        [-up[0] * s, -up[1] * s, -up[2] * s, dot(up, eye) * s],
        [z[0] * a, z[1] * a, z[2] * a, -dot(z, eye) * a + b],
        [-z[0], -z[1], -z[2], dot(z, eye)],
    ])
}
fn project(rows: [[f32; 4]; 4], position: [f32; 3]) -> [f32; 4] {
    rows.map(|row| row[0] * position[0] + row[1] * position[1] + row[2] * position[2] + row[3])
}
pub fn visible(mesh: &Mesh, rows: [[f32; 4]; 4]) -> Result<Vec<u32>> {
    if mesh.meshlets.len() > 32 {
        return Err("Frustum reference supports at most 32 meshlets".into());
    }
    let mut ids = Vec::new();
    for (id, meshlet) in mesh.meshlets.iter().enumerate() {
        let mut common = 63u32;
        for index in 0..meshlet.vertex_count as usize {
            let position =
                mesh.positions[mesh.vertex_refs[meshlet.vertex_offset as usize + index] as usize];
            let [x, y, z, w] = project(rows, position);
            let flags = u32::from(x < -w)
                | (u32::from(x > w) << 1)
                | (u32::from(y < -w) << 2)
                | (u32::from(y > w) << 3)
                | (u32::from(z < 0.) << 4)
                | (u32::from(z > w) << 5);
            common &= flags;
        }
        if common == 0 {
            ids.push(id as u32);
        }
    }
    Ok(ids)
}
pub struct Pixel {
    pub color: [f32; 3],
    pub depth: f32,
    pub covered: bool,
    pub identity: [u32; 2],
    pub vertices: [u32; 3],
    pub weights: [f32; 3],
}
pub fn pixel(
    mesh: &Mesh,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    scenario: u32,
    frustum: bool,
) -> Result<Pixel> {
    if x >= width || y >= height || scenario > 2 {
        return Err("Invalid camera reference pixel/scenario".into());
    }
    let rows = if frustum {
        view_rows(width, height, scenario)?
    } else {
        rows(width, height)?
    };
    let point = [
        (x as f32 + 0.5) / width as f32 * 2. - 1.,
        (y as f32 + 0.5) / height as f32 * 2. - 1.,
    ];
    let count = match scenario {
        0 => 0,
        1 => 1,
        _ => mesh.meshlets.len(),
    };
    let ids = if frustum {
        visible(mesh, rows)?
    } else {
        (0..count as u32).collect()
    };
    let mut result = Pixel {
        color: [0.025, 0.07, 0.12],
        depth: 1.,
        covered: false,
        identity: [0, 0],
        vertices: [0; 3],
        weights: [0.; 3],
    };
    for id in ids {
        let meshlet = &mesh.meshlets[id as usize];
        for triangle in 0..meshlet.triangle_count as usize {
            let mut vertices = [0u32; 3];
            let mut positions = [[0.; 3]; 3];
            let mut clip = [[0.; 4]; 3];
            let mut screen = [[0.; 2]; 3];
            for i in 0..3 {
                let local =
                    mesh.triangles[meshlet.triangle_offset as usize + triangle * 3 + i] as usize;
                vertices[i] = mesh.vertex_refs[meshlet.vertex_offset as usize + local];
                positions[i] = mesh.positions[vertices[i] as usize];
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
                identity: [id + 1, triangle as u32],
                vertices,
                weights: std::array::from_fn(|i| weights[i] / clip[i][3] / reciprocal),
            };
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conservative_clip_planes_reject_wholly_outside_but_keep_crossing_meshlets() {
        let mesh = Mesh {
            content_id: [0; 32],
            provenance: String::new(),
            positions: vec![
                [2., -0.5, 0.5],
                [3., -0.5, 0.5],
                [2., 0.5, 0.5],
                [0.5, -0.5, 0.5],
                [2., -0.5, 0.5],
                [0.5, 0.5, 0.5],
            ],
            meshlets: vec![
                theia_assets::Meshlet {
                    vertex_offset: 0,
                    triangle_offset: 0,
                    vertex_count: 3,
                    triangle_count: 1,
                },
                theia_assets::Meshlet {
                    vertex_offset: 3,
                    triangle_offset: 3,
                    vertex_count: 3,
                    triangle_count: 1,
                },
            ],
            vertex_refs: (0..6).collect(),
            triangles: vec![0, 1, 2, 0, 1, 2],
        };
        let identity = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];
        assert_eq!(visible(&mesh, identity).unwrap(), vec![1]);
        assert!(view_rows(960, 540, 3).is_err());
    }
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
        let pixel = pixel(&mesh, 0, 0, 1, 1, 2, false).unwrap();
        // Independent center ray: eye + 0.85*(target-eye) reaches z=0.65 at y=-0.255.
        let distance = 3.89f64.sqrt() * 0.85;
        let expected_depth = 10. / 9.9 - 1. / (9.9 * distance);
        assert!(pixel.covered);
        // Independent center-ray UV on [-.6,.6]^2: (.5, .2875), despite unequal clip W.
        let uv: [f32; 2] = std::array::from_fn(|axis| {
            (0..3)
                .map(|i| {
                    (mesh.positions[pixel.vertices[i] as usize][axis] + 0.6) / 1.2
                        * pixel.weights[i]
                })
                .sum()
        });
        assert!((uv[0] - 0.5).abs() < 1e-6 && (uv[1] - 0.2875).abs() < 1e-6);
        assert!((pixel.color[0] - 0.5).abs() < 1e-6);
        assert!((pixel.color[1] - 0.3725).abs() < 1e-6);
        assert!((pixel.depth as f64 - expected_depth).abs() < 2e-6);
    }
}
