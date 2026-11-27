//! Strict cooked-file admission for the optional streaming probe, independent of
//! the JSON visual replay payload. No alternate asset format is loaded at runtime.
use super::{Chunk, Part, Result, TextureData};
use std::{io::Read, path::Path};
use theia_assets::{
    bundle::{Bundle, BundleProfile},
    content_id,
};
pub(super) struct Input {
    pub manifest: Vec<u8>,
    pub roots: [Vec<u8>; 4],
    pub leaves: Vec<Vec<u8>>,
}
fn hex(id: [u8; 32]) -> String {
    id.iter().map(|v| format!("{v:02x}")).collect()
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("Cooked stream file exceeds limit".into());
    }
    Ok(bytes)
}
impl Input {
    pub fn load(path: &Path) -> Result<Self> {
        let manifest = read(path, theia_assets::bundle::BYTES)?;
        if path.file_stem().and_then(|v| v.to_str()) != Some(&hex(content_id(&manifest))) {
            return Err("Cooked manifest filename hash mismatch".into());
        }
        let bundle = Bundle::decode(&manifest, None)?;
        if bundle.profile != BundleProfile::TextureTable {
            return Err("Streaming requires indexed bundles".into());
        }
        let parent = path.parent().ok_or("Missing bundle directory")?;
        let mut roots = Vec::new();
        for (dep, ext) in bundle.dependencies.iter().zip([
            "theia-mesh",
            "theia-material",
            "theia-uv",
            "theia-textures",
        ]) {
            roots.push(read(
                &parent.join(format!("{}.{ext}", hex(dep.content_id))),
                dep.bytes as usize,
            )?);
        }
        let roots: [Vec<u8>; 4] = roots.try_into().map_err(|_| "Wrong root count")?;
        let checked = bundle.validate_table_roots(std::array::from_fn(|i| roots[i].as_slice()))?;
        if checked.mesh.meshlets.len() > 32
            || checked
                .uvs
                .texcoords
                .iter()
                .flatten()
                .any(|v| v.abs() > 64.)
        {
            return Err("Cooked stream exceeds qualified meshlet/UV bounds".into());
        }
        let leaves = checked
            .table
            .textures
            .iter()
            .map(|dep| {
                read(
                    &parent.join(hex(dep.content_id) + ".ktx2"),
                    dep.bytes as usize,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        checked
            .table
            .validate_textures(&leaves.iter().map(Vec::as_slice).collect::<Vec<_>>())?;
        Ok(Self {
            manifest,
            roots,
            leaves,
        })
    }
    pub fn chunk(&self, id: usize) -> Result<Chunk> {
        let checked = Bundle::decode(&self.manifest, None)?
            .validate_table_roots(std::array::from_fn(|i| self.roots[i].as_slice()))?;
        let textures = checked
            .table
            .validate_textures(&self.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>())?;
        let mut parts = checked
            .materials
            .base_colors
            .iter()
            .enumerate()
            .map(|(i, &factor)| {
                let tex = &textures[checked.table.material_textures[i] as usize];
                Part {
                    positions: Vec::new(),
                    uvs: Vec::new(),
                    factor,
                    texture: TextureData {
                        width: tex.width,
                        height: tex.height,
                        rgba: tex.texels.to_vec(),
                    },
                }
            })
            .collect::<Vec<_>>();
        let mut ground = Vec::new();
        for (m, meshlet) in checked.mesh.meshlets.iter().enumerate() {
            for t in 0..meshlet.triangle_count as usize {
                let material = checked.materials.triangle_materials
                    [checked.materials.triangle_offsets[m] as usize + t]
                    as usize;
                let vertices: [usize; 3] = std::array::from_fn(|v| {
                    let local = checked.mesh.triangles[meshlet.triangle_offset as usize + t * 3 + v]
                        as usize;
                    checked.mesh.vertex_refs[meshlet.vertex_offset as usize + local] as usize
                });
                for vertex in vertices {
                    parts[material]
                        .positions
                        .push(checked.mesh.positions[vertex]);
                    parts[material].uvs.push(checked.uvs.texcoords[vertex]);
                }
                if material == 0 {
                    ground.push(vertices.map(|v| checked.mesh.positions[v]));
                }
            }
        }
        parts.retain(|p| !p.positions.is_empty());
        Ok(Chunk {
            id,
            region: id % 4,
            parts,
            ground,
        })
    }
}
pub(super) fn logical_bytes(chunk: &Chunk) -> u64 {
    chunk
        .parts
        .iter()
        .map(|p| p.positions.len() as u64 * 32 + p.texture.rgba.len() as u64 + 32)
        .sum()
}
