//! Bounded opaque unlit factors bound to one immutable geometry page.
use crate::{AssetError, Mesh, Result, aligned, content_id, word};

pub const HEADER_BYTES: usize = 96;
pub const DEFAULT_BUDGET: usize = 4 * 1024 * 1024;
const MAX_MATERIALS: usize = 256;
const MAX_MESHLETS: usize = 4096;

#[derive(Debug)]
pub struct MaterialPage {
    pub content_id: [u8; 32],
    pub geometry_id: [u8; 32],
    pub provenance: String,
    pub base_colors: Vec<[f32; 4]>,
    pub triangle_offsets: Vec<u32>,
    pub triangle_materials: Vec<u32>,
}
impl MaterialPage {
    pub fn decode(
        bytes: &[u8],
        expected: Option<[u8; 32]>,
        mesh: &Mesh,
        budget: usize,
    ) -> Result<Self> {
        if bytes.len() > budget {
            return Err(AssetError("material page exceeds byte budget"));
        }
        if bytes.len() < HEADER_BYTES || &bytes[..8] != b"THEIAMAT" || word(bytes, 8)? != 1 {
            return Err(AssetError("invalid material magic/header/version"));
        }
        let meta = word(bytes, 12)? as usize;
        let materials = word(bytes, 16)? as usize;
        let meshlets = word(bytes, 20)? as usize;
        let triangles = word(bytes, 24)? as usize;
        let payload_len = word(bytes, 28)? as usize;
        let geometry_triangles = mesh
            .meshlets
            .iter()
            .try_fold(0usize, |sum, meshlet| {
                sum.checked_add(meshlet.triangle_count as usize)
            })
            .ok_or(AssetError("geometry triangle count overflow"))?;
        if !(1..=MAX_MATERIALS).contains(&materials)
            || !(1..=MAX_MESHLETS).contains(&meshlets)
            || meta == 0
            || meta > 1024 * 1024
            || meshlets != mesh.meshlets.len()
            || triangles != geometry_triangles
            || bytes[32..64] != mesh.content_id
        {
            return Err(AssetError("invalid material counts or geometry binding"));
        }
        let length = aligned(meta)?
            .checked_add(
                materials
                    .checked_mul(16)
                    .ok_or(AssetError("size overflow"))?,
            )
            .and_then(|v| v.checked_add(meshlets.checked_add(1)?.checked_mul(4)?))
            .and_then(|v| v.checked_add(triangles.checked_mul(4)?))
            .ok_or(AssetError("size overflow"))?;
        if length != payload_len || HEADER_BYTES.checked_add(length) != Some(bytes.len()) {
            return Err(AssetError("material payload length mismatch"));
        }
        let id = content_id(bytes);
        let payload = &bytes[HEADER_BYTES..];
        if expected.is_some_and(|expected| expected != id)
            || content_id(payload).as_slice() != &bytes[64..96]
        {
            return Err(AssetError("material content/payload digest mismatch"));
        }
        let provenance = std::str::from_utf8(&payload[..meta])
            .map_err(|_| AssetError("invalid material provenance UTF-8"))?
            .to_string();
        let mut cursor = aligned(meta)?;
        if payload[meta..cursor].iter().any(|v| *v != 0) {
            return Err(AssetError("noncanonical material metadata padding"));
        }
        let mut base_colors = Vec::with_capacity(materials);
        for _ in 0..materials {
            let color = std::array::from_fn(|axis| {
                f32::from_bits(word(payload, cursor + axis * 4).unwrap())
            });
            if color
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                || color[3] != 1.0
            {
                return Err(AssetError(
                    "material requires finite opaque linear RGBA factor",
                ));
            }
            base_colors.push(color);
            cursor += 16;
        }
        let mut triangle_offsets = Vec::with_capacity(meshlets + 1);
        let mut total = 0u32;
        for index in 0..=meshlets {
            let offset = word(payload, cursor)?;
            if offset != total {
                return Err(AssetError(
                    "material triangle offsets do not match geometry",
                ));
            }
            triangle_offsets.push(offset);
            cursor += 4;
            if let Some(meshlet) = mesh.meshlets.get(index) {
                total = total
                    .checked_add(meshlet.triangle_count)
                    .ok_or(AssetError("triangle count overflow"))?;
            }
        }
        let mut triangle_materials = Vec::with_capacity(triangles);
        for _ in 0..triangles {
            let material = word(payload, cursor)?;
            if material as usize >= materials {
                return Err(AssetError("triangle material reference invalid"));
            }
            triangle_materials.push(material);
            cursor += 4;
        }
        Ok(Self {
            content_id: id,
            geometry_id: mesh.content_id,
            provenance,
            base_colors,
            triangle_offsets,
            triangle_materials,
        })
    }
    pub fn color(&self, meshlet: u32, triangle: u32) -> Option<[f32; 4]> {
        let index = usize::try_from(meshlet).ok()?;
        let start = *self.triangle_offsets.get(index)?;
        let end = *self.triangle_offsets.get(index.checked_add(1)?)?;
        if triangle >= end.checked_sub(start)? {
            return None;
        }
        let material = *self
            .triangle_materials
            .get(start.checked_add(triangle)? as usize)?;
        self.base_colors.get(material as usize).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Mesh, Vec<u8>) {
        let mesh = Mesh::decode(&crate::tests::fixture(), None, crate::DEFAULT_BUDGET).unwrap();
        let mut payload = b"mat\0".to_vec();
        for v in [0.25f32, 0.5, 0.75, 1.0] {
            payload.extend(v.to_le_bytes());
        }
        for v in [0u32, 1, 0] {
            payload.extend(v.to_le_bytes());
        }
        let mut bytes = b"THEIAMAT".to_vec();
        for v in [1u32, 3, 1, 1, 1, payload.len() as u32] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend(mesh.content_id);
        bytes.extend(content_id(&payload));
        bytes.extend(payload);
        (mesh, bytes)
    }
    fn rehash(bytes: &mut [u8]) {
        let hash = content_id(&bytes[HEADER_BYTES..]);
        bytes[64..96].copy_from_slice(&hash);
    }
    #[test]
    fn immutable_binding_and_local_triangle_lookup() {
        let (mesh, bytes) = fixture();
        let page =
            MaterialPage::decode(&bytes, Some(content_id(&bytes)), &mesh, DEFAULT_BUDGET).unwrap();
        assert_eq!(page.color(0, 0), Some([0.25, 0.5, 0.75, 1.0]));
        assert_eq!(page.color(0, 1), None);
        assert_eq!(page.color(u32::MAX, 0), None);
        assert_eq!(page.color(0, u32::MAX), None);
        assert_eq!(page.geometry_id, mesh.content_id);
    }
    #[test]
    fn multiple_meshlets_use_local_triangle_ids_and_independent_material_indices() {
        let (original, _) = fixture();
        let mut payload = b"test".to_vec();
        for v in original.positions.iter().flatten() {
            payload.extend(v.to_le_bytes());
        }
        for v in [0u32, 0, 3, 1, 3, 3, 3, 1, 0, 1, 2, 0, 1, 2] {
            payload.extend(v.to_le_bytes());
        }
        payload.extend([0, 1, 2, 0, 2, 1, 0, 0]);
        let mut geometry = b"THEIAMES".to_vec();
        for v in [1u32, 4, 3, 2, 6, 6, 64, 64, payload.len() as u32] {
            geometry.extend(v.to_le_bytes());
        }
        geometry.extend(content_id(&payload));
        geometry.extend(payload);
        let mesh = Mesh::decode(&geometry, None, crate::DEFAULT_BUDGET).unwrap();
        let mut payload = b"mat\0".to_vec();
        for v in [1f32, 0., 0., 1., 0., 0., 1., 1.] {
            payload.extend(v.to_le_bytes());
        }
        for v in [0u32, 1, 2, 1, 0] {
            payload.extend(v.to_le_bytes());
        }
        let mut bytes = b"THEIAMAT".to_vec();
        for v in [1u32, 3, 2, 2, 2, payload.len() as u32] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend(mesh.content_id);
        bytes.extend(content_id(&payload));
        bytes.extend(payload);
        let page = MaterialPage::decode(&bytes, None, &mesh, DEFAULT_BUDGET).unwrap();
        assert_eq!(page.color(0, 0), Some([0., 0., 1., 1.]));
        assert_eq!(page.color(1, 0), Some([1., 0., 0., 1.]));
        assert_eq!(page.color(1, 1), None);
    }
    #[test]
    fn budgets_truncation_counts_digests_and_geometry_mismatch() {
        let (mesh, bytes) = fixture();
        for length in 0..bytes.len() {
            assert!(MaterialPage::decode(&bytes[..length], None, &mesh, DEFAULT_BUDGET).is_err());
        }
        assert!(MaterialPage::decode(&bytes, None, &mesh, bytes.len() - 1).is_err());
        assert!(MaterialPage::decode(&bytes, Some([0; 32]), &mesh, DEFAULT_BUDGET).is_err());
        for offset in [0, 8, 12, 16, 20, 24, 28, 32, 64, HEADER_BYTES] {
            let mut invalid = bytes.clone();
            invalid[offset] ^= 0x80;
            assert!(
                MaterialPage::decode(&invalid, None, &mesh, DEFAULT_BUDGET).is_err(),
                "offset {offset}"
            );
        }
    }
    #[test]
    fn hashed_but_invalid_factors_padding_offsets_and_references() {
        let (mesh, bytes) = fixture();
        for (offset, value) in [
            (HEADER_BYTES + 4, f32::NAN.to_bits()),
            (HEADER_BYTES + 4, (-0.1f32).to_bits()),
            (HEADER_BYTES + 8, 1.1f32.to_bits()),
            (HEADER_BYTES + 16, 0.5f32.to_bits()),
            (HEADER_BYTES + 20, 1),
            (HEADER_BYTES + 24, 2),
            (HEADER_BYTES + 28, 1),
        ] {
            let mut invalid = bytes.clone();
            invalid[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            rehash(&mut invalid);
            assert!(
                MaterialPage::decode(&invalid, None, &mesh, DEFAULT_BUDGET).is_err(),
                "offset {offset}"
            );
        }
        for (offset, value) in [(HEADER_BYTES, 0xff), (HEADER_BYTES + 3, 1)] {
            let mut invalid = bytes.clone();
            invalid[offset] = value;
            rehash(&mut invalid);
            assert!(MaterialPage::decode(&invalid, None, &mesh, DEFAULT_BUDGET).is_err());
        }
    }
}
