//! Fixed immutable dependency set for the bounded opaque textured mesh profile.
//! Roles are ordered: geometry, material factors, UVs, one shared resident texture.
use crate::{
    AssetError, Mesh, Result, content_id, material::MaterialPage, texture::Texture,
    texture_table::TextureTable, uv::UvPage, word,
};
pub const BYTES: usize = 176;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleProfile {
    SingleTexture,
    TextureTable,
}
#[derive(Clone, Copy, Debug)]
pub struct Dependency {
    pub content_id: [u8; 32],
    pub bytes: u64,
}
#[derive(Debug)]
pub struct Bundle {
    pub content_id: [u8; 32],
    pub dependencies: [Dependency; 4],
    pub profile: BundleProfile,
}
pub struct Validated<'a> {
    pub mesh: Mesh,
    pub materials: MaterialPage,
    pub uvs: UvPage,
    pub texture: Texture<'a>,
}
pub struct ValidatedTable {
    pub mesh: Mesh,
    pub materials: MaterialPage,
    pub uvs: UvPage,
    pub table: TextureTable,
}
impl Bundle {
    pub fn decode(bytes: &[u8], expected: Option<[u8; 32]>) -> Result<Self> {
        if bytes.len() != BYTES
            || &bytes[..8] != b"THEIABND"
            || !matches!(word(bytes, 8)?, 1 | 2)
            || word(bytes, 12)? != 4
        {
            return Err(AssetError("invalid texture bundle header/version/roles"));
        }
        let profile = if word(bytes, 8)? == 1 {
            BundleProfile::SingleTexture
        } else {
            BundleProfile::TextureTable
        };
        let mut dependencies = [Dependency {
            content_id: [0; 32],
            bytes: 0,
        }; 4];
        let minima = [
            crate::HEADER_BYTES,
            crate::material::HEADER_BYTES,
            crate::uv::HEADER_BYTES,
            if profile == BundleProfile::SingleTexture {
                104
            } else {
                crate::texture_table::HEADER_BYTES
            },
        ];
        let maxima = [
            crate::DEFAULT_BUDGET,
            crate::material::DEFAULT_BUDGET,
            crate::uv::DEFAULT_BUDGET,
            if profile == BundleProfile::SingleTexture {
                crate::texture::DEFAULT_BUDGET
            } else {
                crate::texture_table::DEFAULT_BUDGET
            },
        ];
        for (i, dep) in dependencies.iter_mut().enumerate() {
            let start = 16 + i * 40;
            dep.content_id = bytes[start..start + 32].try_into().unwrap();
            dep.bytes = u64::from_le_bytes(bytes[start + 32..start + 40].try_into().unwrap());
            if dep.bytes < minima[i] as u64 || dep.bytes > maxima[i] as u64 {
                return Err(AssetError("texture bundle dependency exceeds role budget"));
            }
        }
        for i in 0..4 {
            if dependencies[..i]
                .iter()
                .any(|v| v.content_id == dependencies[i].content_id)
            {
                return Err(AssetError("duplicate texture bundle content role"));
            }
        }
        let id = content_id(bytes);
        if expected.is_some_and(|v| v != id) {
            return Err(AssetError("texture bundle digest mismatch"));
        }
        Ok(Self {
            content_id: id,
            dependencies,
            profile,
        })
    }
    /// Validate every immutable child and exact geometry binding before publication.
    /// This is admission; it does not publish GPU descriptors or reserve residency.
    pub fn validate<'a>(&self, bytes: [&'a [u8]; 4]) -> Result<Validated<'a>> {
        if self.profile != BundleProfile::SingleTexture {
            return Err(AssetError(
                "indexed bundle requires table/texture-set admission",
            ));
        }
        self.check_children(bytes)?;
        let mesh = Mesh::decode(bytes[0], None, crate::DEFAULT_BUDGET)?;
        let materials =
            MaterialPage::decode(bytes[1], None, &mesh, crate::material::DEFAULT_BUDGET)?;
        let uvs = UvPage::decode(bytes[2], None, &mesh, crate::uv::DEFAULT_BUDGET)?;
        let texture = Texture::decode(bytes[3], None, crate::texture::DEFAULT_BUDGET)?;
        if texture.provenance.is_none() {
            return Err(AssetError("bundle texture requires cooked provenance"));
        }
        Ok(Validated {
            mesh,
            materials,
            uvs,
            texture,
        })
    }
    fn check_children(&self, bytes: [&[u8]; 4]) -> Result<()> {
        for (dep, data) in self.dependencies.iter().zip(bytes) {
            if data.len() as u64 != dep.bytes || content_id(data) != dep.content_id {
                return Err(AssetError("texture bundle child length/hash mismatch"));
            }
        }
        Ok(())
    }
    /// Admit four indexed roots; caller must additionally validate all table textures.
    pub fn validate_table_roots(&self, bytes: [&[u8]; 4]) -> Result<ValidatedTable> {
        if self.profile != BundleProfile::TextureTable {
            return Err(AssetError(
                "single-texture bundle requires single-texture admission",
            ));
        }
        self.check_children(bytes)?;
        let mesh = Mesh::decode(bytes[0], None, crate::DEFAULT_BUDGET)?;
        let materials =
            MaterialPage::decode(bytes[1], None, &mesh, crate::material::DEFAULT_BUDGET)?;
        let uvs = UvPage::decode(bytes[2], None, &mesh, crate::uv::DEFAULT_BUDGET)?;
        let table = TextureTable::decode(
            bytes[3],
            None,
            &materials,
            crate::texture_table::DEFAULT_BUDGET,
        )?;
        Ok(ValidatedTable {
            mesh,
            materials,
            uvs,
            table,
        })
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn fixture() -> (Vec<u8>, [Vec<u8>; 4]) {
        let mesh = crate::tests::fixture();
        let mesh_id = content_id(&mesh);
        let mut material_payload = b"mat\0".to_vec();
        for value in [1f32, 0., 0., 1.] {
            material_payload.extend(value.to_le_bytes());
        }
        for value in [0u32, 1, 0] {
            material_payload.extend(value.to_le_bytes());
        }
        let mut material = b"THEIAMAT".to_vec();
        for value in [1u32, 3, 1, 1, 1, material_payload.len() as u32] {
            material.extend(value.to_le_bytes());
        }
        material.extend(mesh_id);
        material.extend(content_id(&material_payload));
        material.extend(material_payload);
        let mut uv_payload = b"uvs\0".to_vec();
        for value in [0f32, 0., 1., 0., 0., 1.] {
            uv_payload.extend(value.to_le_bytes());
        }
        let mut uv = b"THEIAUVS".to_vec();
        for value in [1u32, 3, 3, uv_payload.len() as u32] {
            uv.extend(value.to_le_bytes());
        }
        uv.extend(mesh_id);
        uv.extend(content_id(&uv_payload));
        uv.extend(uv_payload);
        let source = include_bytes!("../../../fixtures/textures/opaque_2x2.ktx2");
        let key = b"THEIAprovenance\0test\0";
        let mut entry = (key.len() as u32).to_le_bytes().to_vec();
        entry.extend(key);
        entry.resize(crate::aligned(entry.len()).unwrap(), 0);
        let mut tex = source[..268].to_vec();
        tex.extend(&entry);
        tex.extend(&source[268..]);
        tex[60..64].copy_from_slice(&(72 + entry.len() as u32).to_le_bytes());
        tex[80..88].copy_from_slice(&(268 + entry.len() as u64).to_le_bytes());
        let children = [mesh, material, uv, tex];
        let mut encoded = b"THEIABND".to_vec();
        encoded.extend(1u32.to_le_bytes());
        encoded.extend(4u32.to_le_bytes());
        for child in &children {
            encoded.extend(content_id(child));
            encoded.extend((child.len() as u64).to_le_bytes());
        }
        (encoded, children)
    }
    #[test]
    fn complete_set_validates_every_role_and_geometry_binding() {
        let (encoded, children) = fixture();
        let bundle = Bundle::decode(&encoded, Some(content_id(&encoded))).unwrap();
        let checked = bundle
            .validate(std::array::from_fn(|i| children[i].as_slice()))
            .unwrap();
        assert_eq!(checked.mesh.positions.len(), checked.uvs.texcoords.len());
        assert_eq!(checked.texture.width, 2);
        for missing in 0..4 {
            let mut slices = std::array::from_fn(|i| children[i].as_slice());
            slices[missing] = &[];
            assert!(bundle.validate(slices).is_err());
        }
        assert!(
            bundle
                .validate([&children[0], &children[2], &children[1], &children[3]])
                .is_err()
        );
    }
    #[test]
    fn all_truncations_unknown_roles_duplicate_hashes_and_overbudget_lengths_fail() {
        let (encoded, _) = fixture();
        for length in 0..encoded.len() {
            assert!(Bundle::decode(&encoded[..length], None).is_err());
        }
        assert!(Bundle::decode(&encoded, Some([0; 32])).is_err());
        for offset in [0, 8, 12] {
            let mut bad = encoded.clone();
            bad[offset] ^= 0x80;
            assert!(Bundle::decode(&bad, None).is_err());
        }
        for i in 0..4 {
            for count in [0u64, u64::MAX] {
                let mut bad = encoded.clone();
                bad[48 + i * 40..56 + i * 40].copy_from_slice(&count.to_le_bytes());
                assert!(Bundle::decode(&bad, None).is_err());
            }
        }
        let mut bad = encoded.clone();
        bad[56..88].copy_from_slice(&encoded[16..48]);
        assert!(Bundle::decode(&bad, None).is_err());
    }
    #[test]
    fn valid_content_hash_cannot_override_foreign_geometry_or_invalid_child_semantics() {
        let (encoded, mut children) = fixture();
        // A freshly hashed foreign UV page still cannot bind this geometry.
        children[2][24] ^= 1;
        let mut rebound = encoded.clone();
        rebound[96..128].copy_from_slice(&content_id(&children[2]));
        let bundle = Bundle::decode(&rebound, None).unwrap();
        assert!(
            bundle
                .validate(std::array::from_fn(|i| children[i].as_slice()))
                .is_err()
        );
        let (mut encoded, mut children) = fixture();
        // A freshly hashed non-opaque texture is not admitted as an opaque child.
        *children[3].last_mut().unwrap() = 0;
        encoded[136..168].copy_from_slice(&content_id(&children[3]));
        let bundle = Bundle::decode(&encoded, None).unwrap();
        assert!(
            bundle
                .validate(std::array::from_fn(|i| children[i].as_slice()))
                .is_err()
        );
    }
    #[test]
    fn indexed_roots_require_complete_child_admission_and_exact_material_binding() {
        let (_, children) = fixture();
        let mut payload = b"table\0\0\0".to_vec();
        payload.extend(content_id(&children[3]));
        payload.extend((children[3].len() as u64).to_le_bytes());
        payload.extend(0u32.to_le_bytes());
        let mut table = b"THEIATEX".to_vec();
        for v in [1u32, 5, 1, 1] {
            table.extend(v.to_le_bytes());
        }
        table.extend(content_id(&children[1]));
        table.extend(content_id(&payload));
        table.extend(payload);
        let mut roots = children.clone();
        roots[3] = table;
        let mut encoded = b"THEIABND".to_vec();
        encoded.extend(2u32.to_le_bytes());
        encoded.extend(4u32.to_le_bytes());
        for root in &roots {
            encoded.extend(content_id(root));
            encoded.extend((root.len() as u64).to_le_bytes());
        }
        let bundle = Bundle::decode(&encoded, None).unwrap();
        assert_eq!(bundle.profile, BundleProfile::TextureTable);
        assert!(
            bundle
                .validate(std::array::from_fn(|i| roots[i].as_slice()))
                .is_err()
        );
        let validated = bundle
            .validate_table_roots(std::array::from_fn(|i| roots[i].as_slice()))
            .unwrap();
        assert!(validated.table.validate_textures(&[]).is_err());
        assert!(
            validated
                .table
                .validate_textures(&[children[3].as_slice()])
                .is_ok()
        );
        roots[3][24] ^= 1;
        encoded[136..168].copy_from_slice(&content_id(&roots[3]));
        let rebound = Bundle::decode(&encoded, None).unwrap();
        assert!(
            rebound
                .validate_table_roots(std::array::from_fn(|i| roots[i].as_slice()))
                .is_err()
        );
    }
}
