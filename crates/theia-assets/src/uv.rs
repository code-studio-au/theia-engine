//! Dense TEXCOORD_0 values bound to the exact immutable geometry vertex stream.
use crate::{AssetError, Mesh, Result, aligned, content_id, word};
pub const HEADER_BYTES: usize = 88;
pub const DEFAULT_BUDGET: usize = 4 * 1024 * 1024;
#[derive(Debug)]
pub struct UvPage {
    pub content_id: [u8; 32],
    pub geometry_id: [u8; 32],
    pub provenance: String,
    pub texcoords: Vec<[f32; 2]>,
}
impl UvPage {
    pub fn decode(
        bytes: &[u8],
        expected: Option<[u8; 32]>,
        mesh: &Mesh,
        budget: usize,
    ) -> Result<Self> {
        if bytes.len() > budget {
            return Err(AssetError("UV page exceeds byte budget"));
        }
        if bytes.len() < HEADER_BYTES || &bytes[..8] != b"THEIAUVS" || word(bytes, 8)? != 1 {
            return Err(AssetError("invalid UV magic/header/version"));
        }
        let meta = word(bytes, 12)? as usize;
        let count = word(bytes, 16)? as usize;
        let length = word(bytes, 20)? as usize;
        if !(1..=65536).contains(&count)
            || count != mesh.positions.len()
            || meta == 0
            || meta > 1024 * 1024
            || bytes[24..56] != mesh.content_id
        {
            return Err(AssetError("invalid UV counts or geometry binding"));
        }
        let expected_length = aligned(meta)?
            .checked_add(count.checked_mul(8).ok_or(AssetError("size overflow"))?)
            .ok_or(AssetError("size overflow"))?;
        if length != expected_length || HEADER_BYTES.checked_add(length) != Some(bytes.len()) {
            return Err(AssetError("UV payload length mismatch"));
        }
        let id = content_id(bytes);
        let payload = &bytes[HEADER_BYTES..];
        if expected.is_some_and(|v| v != id) || content_id(payload).as_slice() != &bytes[56..88] {
            return Err(AssetError("UV content/payload digest mismatch"));
        }
        let provenance = std::str::from_utf8(&payload[..meta])
            .map_err(|_| AssetError("invalid UV provenance UTF-8"))?
            .to_string();
        let start = aligned(meta)?;
        if payload[meta..start].iter().any(|v| *v != 0) {
            return Err(AssetError("noncanonical UV metadata padding"));
        }
        let mut texcoords = Vec::with_capacity(count);
        for vertex in 0..count {
            let offset = start + vertex * 8;
            let value = [
                f32::from_bits(word(payload, offset)?),
                f32::from_bits(word(payload, offset + 4)?),
            ];
            if value.iter().any(|v| !v.is_finite()) {
                return Err(AssetError("nonfinite UV value"));
            }
            texcoords.push(value);
        }
        Ok(Self {
            content_id: id,
            geometry_id: mesh.content_id,
            provenance,
            texcoords,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Mesh, Vec<u8>) {
        let mesh = Mesh::decode(&crate::tests::fixture(), None, crate::DEFAULT_BUDGET).unwrap();
        let mut payload = b"uvs\0".to_vec();
        for v in [-1f32, 0., 2., 0., 0., 1.] {
            payload.extend(v.to_le_bytes());
        }
        let mut bytes = b"THEIAUVS".to_vec();
        for v in [1u32, 3, 3, payload.len() as u32] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend(mesh.content_id);
        bytes.extend(content_id(&payload));
        bytes.extend(payload);
        (mesh, bytes)
    }
    fn rehash(bytes: &mut [u8]) {
        let id = content_id(&bytes[HEADER_BYTES..]);
        bytes[56..88].copy_from_slice(&id);
    }
    #[test]
    fn exact_dense_geometry_binding_preserves_negative_and_repeated_coordinates() {
        let (mesh, bytes) = fixture();
        let page = UvPage::decode(&bytes, Some(content_id(&bytes)), &mesh, DEFAULT_BUDGET).unwrap();
        assert_eq!(page.texcoords, vec![[-1., 0.], [2., 0.], [0., 1.]]);
        assert_eq!(page.geometry_id, mesh.content_id);
    }
    #[test]
    fn every_truncation_counts_geometry_digest_version_and_budget_fail_closed() {
        let (mesh, bytes) = fixture();
        for length in 0..bytes.len() {
            assert!(UvPage::decode(&bytes[..length], None, &mesh, DEFAULT_BUDGET).is_err());
        }
        assert!(UvPage::decode(&bytes, None, &mesh, bytes.len() - 1).is_err());
        assert!(UvPage::decode(&bytes, Some([0; 32]), &mesh, DEFAULT_BUDGET).is_err());
        for offset in [0, 8, 12, 16, 20, 24, 56, HEADER_BYTES] {
            let mut bad = bytes.clone();
            bad[offset] ^= 0x80;
            assert!(UvPage::decode(&bad, None, &mesh, DEFAULT_BUDGET).is_err());
        }
    }
    #[test]
    fn rehashed_nonfinite_padding_and_utf8_are_rejected() {
        let (mesh, bytes) = fixture();
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut bad = bytes.clone();
            bad[HEADER_BYTES + 4..HEADER_BYTES + 8].copy_from_slice(&value.to_le_bytes());
            rehash(&mut bad);
            assert!(UvPage::decode(&bad, None, &mesh, DEFAULT_BUDGET).is_err());
        }
        for (offset, value) in [(HEADER_BYTES, 0xff), (HEADER_BYTES + 3, 1)] {
            let mut bad = bytes.clone();
            bad[offset] = value;
            rehash(&mut bad);
            assert!(UvPage::decode(&bad, None, &mesh, DEFAULT_BUDGET).is_err());
        }
    }
}
