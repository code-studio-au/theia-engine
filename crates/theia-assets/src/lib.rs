//! Bounded immutable meshlet container reader. Offline native cookers are not runtime dependencies.
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
pub mod arena;
pub mod bundle;
pub mod bundle_residency;
pub mod lod;
pub mod material;
pub mod residency;
pub mod texture;
pub mod texture_filter;
pub mod texture_footprint;
pub mod texture_mips;
pub mod texture_residency;
pub mod texture_table;
pub mod uv;
pub mod world_coverage;
pub mod world_demand;
pub const HEADER_BYTES: usize = 76;
pub const DEFAULT_BUDGET: usize = 64 * 1024 * 1024;
#[derive(Debug)]
pub struct AssetError(pub &'static str);
impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl Error for AssetError {}
type Result<T> = std::result::Result<T, AssetError>;
#[derive(Clone, Copy, Debug)]
pub struct Meshlet {
    pub vertex_offset: u32,
    pub triangle_offset: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
}
#[derive(Debug)]
pub struct Mesh {
    pub content_id: [u8; 32],
    pub provenance: String,
    pub positions: Vec<[f32; 3]>,
    pub meshlets: Vec<Meshlet>,
    pub vertex_refs: Vec<u32>,
    pub triangles: Vec<u8>,
}
pub fn content_id(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn word(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or(AssetError("truncated word"))?
            .try_into()
            .unwrap(),
    ))
}
fn aligned(length: usize) -> Result<usize> {
    length
        .checked_add(3)
        .map(|v| v & !3)
        .ok_or(AssetError("size overflow"))
}
impl Mesh {
    pub fn decode(bytes: &[u8], expected: Option<[u8; 32]>, budget: usize) -> Result<Self> {
        if bytes.len() > budget {
            return Err(AssetError("asset exceeds byte budget"));
        }
        if bytes.len() < HEADER_BYTES || &bytes[..8] != b"THEIAMES" {
            return Err(AssetError("invalid mesh magic/header"));
        }
        if word(bytes, 8)? != 1 {
            return Err(AssetError("unsupported mesh schema version"));
        }
        let meta = word(bytes, 12)? as usize;
        let vertices = word(bytes, 16)? as usize;
        let clusters = word(bytes, 20)? as usize;
        let refs = word(bytes, 24)? as usize;
        let triangles = word(bytes, 28)? as usize;
        let max_vertices = word(bytes, 32)?;
        let max_triangles = word(bytes, 36)?;
        let payload_len = word(bytes, 40)? as usize;
        if vertices == 0
            || clusters == 0
            || meta == 0
            || max_vertices == 0
            || max_vertices > 64
            || max_triangles == 0
            || max_triangles > 64
        {
            return Err(AssetError("invalid qualification mesh counts/limits"));
        }
        let expected_len = aligned(meta)?
            .checked_add(
                vertices
                    .checked_mul(12)
                    .ok_or(AssetError("size overflow"))?,
            )
            .and_then(|v| v.checked_add(clusters.checked_mul(16)?))
            .and_then(|v| v.checked_add(refs.checked_mul(4)?))
            .and_then(|v| v.checked_add(aligned(triangles).ok()?))
            .ok_or(AssetError("size overflow"))?;
        if payload_len != expected_len
            || bytes.len()
                != HEADER_BYTES
                    .checked_add(payload_len)
                    .ok_or(AssetError("size overflow"))?
        {
            return Err(AssetError("mesh payload length mismatch"));
        }
        let id = content_id(bytes);
        if expected.is_some_and(|expected| expected != id) {
            return Err(AssetError("content address mismatch"));
        }
        let payload = &bytes[HEADER_BYTES..];
        if content_id(payload).as_slice() != &bytes[44..76] {
            return Err(AssetError("mesh payload digest mismatch"));
        }
        let provenance = std::str::from_utf8(&payload[..meta])
            .map_err(|_| AssetError("invalid provenance UTF-8"))?
            .to_string();
        let mut cursor = aligned(meta)?;
        if payload[meta..cursor].iter().any(|v| *v != 0) {
            return Err(AssetError("noncanonical metadata padding"));
        }
        let mut positions = Vec::with_capacity(vertices);
        for _ in 0..vertices {
            let value = [
                f32::from_bits(word(payload, cursor)?),
                f32::from_bits(word(payload, cursor + 4)?),
                f32::from_bits(word(payload, cursor + 8)?),
            ];
            if !value.iter().all(|v| v.is_finite()) {
                return Err(AssetError("nonfinite mesh position"));
            }
            positions.push(value);
            cursor += 12;
        }
        let mut meshlets = Vec::with_capacity(clusters);
        for _ in 0..clusters {
            let cluster = Meshlet {
                vertex_offset: word(payload, cursor)?,
                triangle_offset: word(payload, cursor + 4)?,
                vertex_count: word(payload, cursor + 8)?,
                triangle_count: word(payload, cursor + 12)?,
            };
            let vertex_end = (cluster.vertex_offset as usize)
                .checked_add(cluster.vertex_count as usize)
                .ok_or(AssetError("meshlet vertex range overflow"))?;
            let triangle_end = (cluster.triangle_offset as usize)
                .checked_add(
                    (cluster.triangle_count as usize)
                        .checked_mul(3)
                        .ok_or(AssetError("meshlet triangle size overflow"))?,
                )
                .ok_or(AssetError("meshlet triangle range overflow"))?;
            if cluster.vertex_count == 0
                || cluster.vertex_count > max_vertices
                || cluster.triangle_count == 0
                || cluster.triangle_count > max_triangles
                || vertex_end > refs
                || triangle_end > triangles
            {
                return Err(AssetError("meshlet range/count invalid"));
            }
            meshlets.push(cluster);
            cursor += 16;
        }
        let mut vertex_refs = Vec::with_capacity(refs);
        for _ in 0..refs {
            let reference = word(payload, cursor)?;
            if reference as usize >= vertices {
                return Err(AssetError("global vertex reference invalid"));
            }
            vertex_refs.push(reference);
            cursor += 4;
        }
        let triangle_bytes = payload[cursor..cursor + triangles].to_vec();
        if payload[cursor + triangles..].iter().any(|v| *v != 0) {
            return Err(AssetError("noncanonical triangle padding"));
        }
        for cluster in &meshlets {
            let begin = cluster.triangle_offset as usize;
            let end = begin + cluster.triangle_count as usize * 3;
            if triangle_bytes[begin..end]
                .iter()
                .any(|index| *index as u32 >= cluster.vertex_count)
            {
                return Err(AssetError("local triangle index invalid"));
            }
        }
        Ok(Self {
            content_id: id,
            provenance,
            positions,
            meshlets,
            vertex_refs,
            triangles: triangle_bytes,
        })
    }
    pub fn triangle_count(&self) -> usize {
        self.meshlets
            .iter()
            .map(|m| m.triangle_count as usize)
            .sum()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture() -> Vec<u8> {
        let mut payload = b"test".to_vec();
        for coordinate in [0f32, 0., 0., 1., 0., 0., 0., 1., 0.] {
            payload.extend(coordinate.to_bits().to_le_bytes());
        }
        for field in [0u32, 0, 3, 1, 0, 1, 2] {
            payload.extend(field.to_le_bytes());
        }
        payload.extend([0, 1, 2, 0]);
        let mut bytes = b"THEIAMES".to_vec();
        for field in [1u32, 4, 3, 1, 3, 3, 64, 64, payload.len() as u32] {
            bytes.extend(field.to_le_bytes());
        }
        bytes.extend(content_id(&payload));
        bytes.extend(payload);
        bytes
    }
    fn rehash(bytes: &mut [u8]) {
        let hash = content_id(&bytes[HEADER_BYTES..]);
        bytes[44..76].copy_from_slice(&hash);
    }
    #[test]
    fn valid_container_roundtrip_and_content_address() {
        let bytes = fixture();
        let mesh = Mesh::decode(&bytes, Some(content_id(&bytes)), DEFAULT_BUDGET).unwrap();
        assert_eq!(mesh.positions.len(), 3);
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.vertex_refs, vec![0, 1, 2]);
    }
    #[test]
    fn corruption_version_length_and_budget_are_rejected() {
        let original = fixture();
        assert!(Mesh::decode(&original, Some([0; 32]), DEFAULT_BUDGET).is_err());
        assert!(Mesh::decode(&original, None, original.len() - 1).is_err());
        for offset in [0, 8, 12, 40, 44, HEADER_BYTES + 4] {
            let mut bytes = original.clone();
            bytes[offset] ^= 0x80;
            assert!(
                Mesh::decode(&bytes, None, DEFAULT_BUDGET).is_err(),
                "offset {offset}"
            );
        }
        for length in 0..original.len() {
            assert!(Mesh::decode(&original[..length], None, DEFAULT_BUDGET).is_err());
        }
    }
    #[test]
    fn hashed_but_invalid_geometry_and_padding_are_rejected() {
        let original = fixture();
        for (offset, value) in [
            (HEADER_BYTES + 4, f32::NAN.to_bits()),
            (HEADER_BYTES + 4 + 36 + 8, 65),
            (HEADER_BYTES + 4 + 36 + 16, 999),
        ] {
            let mut bytes = original.clone();
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            rehash(&mut bytes);
            assert!(Mesh::decode(&bytes, None, DEFAULT_BUDGET).is_err());
        }
        let mut bytes = original.clone();
        let length = bytes.len();
        bytes[length - 2] = 3;
        rehash(&mut bytes);
        assert!(Mesh::decode(&bytes, None, DEFAULT_BUDGET).is_err());
        let mut bytes = original.clone();
        bytes[length - 1] = 1;
        rehash(&mut bytes);
        assert!(Mesh::decode(&bytes, None, DEFAULT_BUDGET).is_err());
    }
}
