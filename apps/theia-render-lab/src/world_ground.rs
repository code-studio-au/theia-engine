//! Presentation-only ground query for the bounded four-quadrant benchmark fixture.
//! Uses actual cooked material-zero triangles, including split coarse perimeter
//! edges. This CPU qualification path does not change renderer/gameplay heights.
use crate::{Result, bundle_streaming::Input};
use std::{io::Read, path::Path};
use theia_assets::{DEFAULT_BUDGET, Mesh, material::MaterialPage};
const SIDE: usize = 8;
const STEP: f32 = 8.;
const MAX_TRIANGLES: usize = 128;
const MAX_CANDIDATES: usize = 8;
type Triangle = [[f32; 3]; 3];
struct Surface {
    origin: [f32; 2],
    triangles: Vec<Triangle>,
    offsets: Vec<usize>,
    candidates: Vec<usize>,
    max_candidates: usize,
}
impl Surface {
    fn new(origin: [f32; 2], triangles: Vec<Triangle>) -> Result<Self> {
        if triangles.is_empty() || triangles.len() > MAX_TRIANGLES {
            return Err("Ground query requires 1..128 terrain triangles".into());
        }
        for &[a, b, c] in &triangles {
            if [a, b, c].iter().flatten().any(|v| !v.is_finite())
                || cross([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]]) <= 0.
                || [a, b, c]
                    .iter()
                    .any(|p| (0..2).any(|i| p[i] < origin[i] || p[i] > origin[i] + 64.))
            {
                return Err(
                    "Ground query requires finite upward triangles inside its region".into(),
                );
            }
        }
        let mut offsets = vec![0];
        let mut candidates = Vec::new();
        let mut max_candidates = 0;
        for y in 0..SIDE {
            for x in 0..SIDE {
                let lo = [origin[0] + x as f32 * STEP, origin[1] + y as f32 * STEP];
                let begin = candidates.len();
                for (id, triangle) in triangles.iter().enumerate() {
                    // Strict overlap excludes triangles merely touching another cell.
                    // Queries on shared edges use triangles from the chosen cell.
                    if (0..2).all(|axis| {
                        triangle.iter().any(|p| p[axis] > lo[axis])
                            && triangle.iter().any(|p| p[axis] < lo[axis] + STEP)
                    }) {
                        candidates.push(id);
                    }
                }
                let count = candidates.len() - begin;
                if count == 0 || count > MAX_CANDIDATES {
                    return Err("Ground query cell is empty or exceeds candidate cap".into());
                }
                max_candidates = max_candidates.max(count);
                offsets.push(candidates.len());
            }
        }
        Ok(Self {
            origin,
            triangles,
            offsets,
            candidates,
            max_candidates,
        })
    }
    fn height(&self, p: [f32; 2]) -> Result<f32> {
        if p.iter().any(|v| !v.is_finite())
            || (0..2).any(|i| p[i] < self.origin[i] || p[i] > self.origin[i] + 64.)
        {
            return Err("Ground query outside region".into());
        }
        let cell: [usize; 2] = std::array::from_fn(|i| {
            (((p[i] - self.origin[i]) / STEP).floor() as usize).min(SIDE - 1)
        });
        let id = cell[1] * SIDE + cell[0];
        let mut height: Option<f32> = None;
        for &triangle in &self.candidates[self.offsets[id]..self.offsets[id + 1]] {
            let [a, b, c] = self.triangles[triangle];
            let ab = [b[0] - a[0], b[1] - a[1]];
            let ac = [c[0] - a[0], c[1] - a[1]];
            let ap = [p[0] - a[0], p[1] - a[1]];
            let denominator = cross(ab, ac);
            let u = cross(ap, ac) / denominator;
            let v = cross(ab, ap) / denominator;
            if u >= -0.000001 && v >= -0.000001 && u + v <= 1.000001 {
                let value = a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2]);
                if height.is_some_and(|old| (old - value).abs() > 0.00001) {
                    return Err("Ground query has conflicting overlapping heights".into());
                }
                height = Some(value);
            }
        }
        height.ok_or_else(|| "Ground query has no covering triangle".into())
    }
}
fn cross(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}
fn surface(entry: &serde_json::Value, slot: usize) -> Result<Surface> {
    let input = Input::load(Path::new(
        entry["cook"]["bundle"]["path"]
            .as_str()
            .ok_or("Missing ground bundle")?,
    ))?;
    let mesh = Mesh::decode(&input.roots[0], None, DEFAULT_BUDGET)?;
    let materials = MaterialPage::decode(
        &input.roots[1],
        None,
        &mesh,
        theia_assets::material::DEFAULT_BUDGET,
    )?;
    let mut triangles = Vec::new();
    for (id, meshlet) in mesh.meshlets.iter().enumerate() {
        for triangle in 0..meshlet.triangle_count as usize {
            if materials.triangle_materials[materials.triangle_offsets[id] as usize + triangle] != 0
            {
                continue;
            }
            triangles.push(std::array::from_fn(|corner| {
                let local = mesh.triangles[meshlet.triangle_offset as usize + triangle * 3 + corner]
                    as usize;
                mesh.positions[mesh.vertex_refs[meshlet.vertex_offset as usize + local] as usize]
            }));
        }
    }
    Surface::new(
        [(slot % 2) as f32 * 64. - 64., (slot / 2) as f32 * 64. - 64.],
        triangles,
    )
}
fn read_json(path: &Path, limit: u64) -> Result<serde_json::Value> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("Ground reference JSON exceeds byte cap".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}
pub fn reference(scene: &Path, queries: &Path) -> Result<serde_json::Value> {
    let scene = read_json(scene, 1024 * 1024)?;
    if scene["passed"] != true {
        return Err("Ground reference requires checked fixture".into());
    }
    let mut surfaces = Vec::new();
    for group in ["quadrants", "fallbacks"] {
        let entries = scene[group].as_array().ok_or("Missing ground regions")?;
        if entries.len() != 4 {
            return Err("Ground reference requires four regions per level".into());
        }
        for (slot, entry) in entries.iter().enumerate() {
            surfaces.push(surface(entry, slot)?);
        }
    }
    let points: Vec<[f32; 2]> = serde_json::from_value(read_json(queries, 4 * 1024 * 1024)?)?;
    if points.is_empty() || points.len() > 50000 {
        return Err("Ground reference requires 1..50000 points".into());
    }
    let mut fine = Vec::new();
    let mut coarse = Vec::new();
    let mut regions = Vec::new();
    for &p in &points {
        if p.iter().any(|v| !v.is_finite() || v.abs() > 64.) {
            return Err("Ground point outside benchmark world".into());
        }
        let slot = usize::from(p[0] >= 0.) + 2 * usize::from(p[1] >= 0.);
        regions.push(slot);
        fine.push(surfaces[slot].height(p)?);
        coarse.push(surfaces[slot + 4].height(p)?);
    }
    let selections = (0..16)
        .map(|mask| {
            regions
                .iter()
                .enumerate()
                .map(|(i, &slot)| {
                    if mask & (1 << slot) != 0 {
                        fine[i]
                    } else {
                        coarse[i]
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    Ok(
        serde_json::json!({"points":points.len(),"regions":regions,"heights":selections,
        "ground_triangles":surfaces.iter().map(|s|s.triangles.len()).collect::<Vec<_>>(),
        "candidate_references":surfaces.iter().map(|s|s.candidates.len()).collect::<Vec<_>>(),
        "max_cell_candidates":surfaces.iter().map(|s|s.max_candidates).collect::<Vec<_>>(),
        "presentation_only":true,"gpu_execution":false}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Vec<Triangle> {
        vec![
            [[0., 0., 0.], [64., 0., 1.], [0., 64., 2.]],
            [[64., 0., 1.], [64., 64., 3.], [0., 64., 2.]],
        ]
    }
    #[test]
    fn affine_surface_and_shared_edges_cover_every_cell() {
        let surface = Surface::new([0., 0.], plane()).unwrap();
        for y in 0..=128 {
            for x in 0..=128 {
                let p = [x as f32 / 2., y as f32 / 2.];
                assert!((surface.height(p).unwrap() - (p[0] + 2. * p[1]) / 64.).abs() < 0.000001);
            }
        }
        assert!(surface.height([64.001, 0.]).is_err());
        assert!(surface.height([f32::NAN, 0.]).is_err());
    }
    #[test]
    fn holes_and_conflicting_height_layers_fail_queries() {
        let surface = Surface::new([0., 0.], vec![[[0., 0., 0.], [64., 0., 0.], [0., 64., 0.]]]);
        assert!(surface.unwrap().height([63., 63.]).is_err());
        let mut triangles = plane();
        let mut upper = plane();
        for triangle in &mut upper {
            for p in triangle {
                p[2] += 1.;
            }
        }
        triangles.extend(upper);
        assert!(
            Surface::new([0., 0.], triangles)
                .unwrap()
                .height([7., 13.])
                .is_err()
        );
        let mut reversed = plane();
        reversed[0].swap(1, 2);
        assert!(Surface::new([0., 0.], reversed).is_err());
    }
}
