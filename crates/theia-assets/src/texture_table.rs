//! Dense material-to-resident-texture bindings for a bounded opaque profile.
use crate::{
    AssetError, Result, aligned, bundle::Dependency, content_id, material::MaterialPage,
    texture::Texture, word,
};
pub const HEADER_BYTES: usize = 88;
pub const DEFAULT_BUDGET: usize = 1024 * 1024;
pub const MAX_TEXTURES: usize = 16;
#[derive(Debug)]
pub struct TextureTable {
    pub content_id: [u8; 32],
    pub material_id: [u8; 32],
    pub provenance: String,
    pub textures: Vec<Dependency>,
    pub material_textures: Vec<u32>,
}
impl TextureTable {
    pub fn decode(
        bytes: &[u8],
        expected: Option<[u8; 32]>,
        materials: &MaterialPage,
        budget: usize,
    ) -> Result<Self> {
        if bytes.len() > budget
            || bytes.len() < HEADER_BYTES
            || &bytes[..8] != b"THEIATEX"
            || word(bytes, 8)? != 1
        {
            return Err(AssetError("invalid texture table header/version/budget"));
        }
        let meta = word(bytes, 12)? as usize;
        let count = word(bytes, 16)? as usize;
        let textures = word(bytes, 20)? as usize;
        if !(1..=256).contains(&count)
            || count != materials.base_colors.len()
            || !(1..=MAX_TEXTURES).contains(&textures)
            || textures > count
            || meta == 0
            || meta > 65536
            || bytes[24..56] != materials.content_id
        {
            return Err(AssetError("invalid texture table counts/material binding"));
        }
        let start = aligned(meta)?;
        let length = start
            .checked_add(textures * 40)
            .and_then(|v| v.checked_add(count * 4))
            .ok_or(AssetError("texture table size overflow"))?;
        if HEADER_BYTES.checked_add(length) != Some(bytes.len()) {
            return Err(AssetError("texture table payload length mismatch"));
        }
        let payload = &bytes[HEADER_BYTES..];
        let id = content_id(bytes);
        if expected.is_some_and(|v| v != id) || content_id(payload).as_slice() != &bytes[56..88] {
            return Err(AssetError("texture table digest mismatch"));
        }
        let provenance = std::str::from_utf8(&payload[..meta])
            .map_err(|_| AssetError("invalid texture table metadata UTF-8"))?
            .to_string();
        if payload[meta..start].iter().any(|&v| v != 0) {
            return Err(AssetError("noncanonical texture table padding"));
        }
        let mut dependencies: Vec<Dependency> = Vec::with_capacity(textures);
        for i in 0..textures {
            let offset = start + i * 40;
            let dep = Dependency {
                content_id: payload[offset..offset + 32].try_into().unwrap(),
                bytes: u64::from_le_bytes(payload[offset + 32..offset + 40].try_into().unwrap()),
            };
            if dep.bytes < 104
                || dep.bytes > crate::texture::DEFAULT_BUDGET as u64
                || dependencies
                    .last()
                    .is_some_and(|v| v.content_id >= dep.content_id)
            {
                return Err(AssetError(
                    "texture dependencies must be bounded and hash-sorted unique",
                ));
            }
            dependencies.push(dep);
        }
        let mut mappings = Vec::with_capacity(count);
        let mut used = 0u32;
        for material in 0..count {
            let index = word(payload, start + textures * 40 + material * 4)?;
            if index >= textures as u32 {
                return Err(AssetError("texture table material index out of range"));
            }
            used |= 1 << index;
            mappings.push(index);
        }
        if used != (1u32 << textures) - 1 {
            return Err(AssetError("texture table contains unreferenced dependency"));
        }
        Ok(Self {
            content_id: id,
            material_id: materials.content_id,
            provenance,
            textures: dependencies,
            material_textures: mappings,
        })
    }
    pub fn texture_for(&self, material: u32) -> Option<u32> {
        self.material_textures.get(material as usize).copied()
    }
    /// Validate the entire immutable texture set before any GPU publication.
    pub fn validate_textures<'a>(&self, bytes: &[&'a [u8]]) -> Result<Vec<Texture<'a>>> {
        if bytes.len() != self.textures.len() {
            return Err(AssetError("texture table requires every dependency"));
        }
        self.textures
            .iter()
            .zip(bytes)
            .map(|(dep, data)| {
                if data.len() as u64 != dep.bytes || content_id(data) != dep.content_id {
                    return Err(AssetError("texture table child length/hash mismatch"));
                }
                let texture = Texture::decode(data, None, crate::texture::DEFAULT_BUDGET)?;
                if texture.provenance.is_none() {
                    return Err(AssetError(
                        "texture table requires cooked texture provenance",
                    ));
                }
                Ok(texture)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (MaterialPage, Vec<u8>, Vec<Vec<u8>>) {
        let materials = MaterialPage {
            content_id: [77; 32],
            geometry_id: [42; 32],
            provenance: "fixture".into(),
            base_colors: vec![[1.; 4]; 3],
            triangle_offsets: vec![0, 1],
            triangle_materials: vec![0],
        };
        let source = include_bytes!("../../../fixtures/textures/opaque_2x2.ktx2");
        let key = b"THEIAprovenance\0test\0";
        let mut entry = (key.len() as u32).to_le_bytes().to_vec();
        entry.extend(key);
        entry.resize(aligned(entry.len()).unwrap(), 0);
        let mut first = source[..268].to_vec();
        first.extend(&entry);
        first.extend(&source[268..]);
        first[60..64].copy_from_slice(&(72 + entry.len() as u32).to_le_bytes());
        first[80..88].copy_from_slice(&(268 + entry.len() as u64).to_le_bytes());
        let mut second = first.clone();
        second[268 + entry.len()] = 128;
        let mut textures = vec![first, second];
        textures.sort_by_key(|v| content_id(v));
        let encoded = encode(&materials, &textures, [1, 0, 1]);
        (materials, encoded, textures)
    }
    fn encode(materials: &MaterialPage, textures: &[Vec<u8>], mapping: [u32; 3]) -> Vec<u8> {
        let mut payload = b"table\0\0\0".to_vec();
        for bytes in textures {
            payload.extend(content_id(bytes));
            payload.extend((bytes.len() as u64).to_le_bytes());
        }
        for index in mapping {
            payload.extend(index.to_le_bytes());
        }
        let mut data = b"THEIATEX".to_vec();
        for count in [1u32, 5, 3, 2] {
            data.extend(count.to_le_bytes());
        }
        data.extend(materials.content_id);
        data.extend(content_id(&payload));
        data.extend(payload);
        data
    }
    fn rehash(bytes: &mut [u8]) {
        let hash = content_id(&bytes[HEADER_BYTES..]);
        bytes[56..88].copy_from_slice(&hash);
    }
    #[test]
    fn dense_mapping_preserves_reuse_and_hash_order_independent_of_material_order() {
        let (materials, bytes, textures) = fixture();
        let table =
            TextureTable::decode(&bytes, Some(content_id(&bytes)), &materials, DEFAULT_BUDGET)
                .unwrap();
        assert_eq!(table.material_textures, vec![1, 0, 1]);
        assert_eq!(table.texture_for(0), table.texture_for(2));
        assert_eq!(table.texture_for(3), None);
        let checked = table
            .validate_textures(&textures.iter().map(Vec::as_slice).collect::<Vec<_>>())
            .unwrap();
        assert_eq!(checked.len(), 2);
        assert_ne!(checked[0].texels, checked[1].texels);
        assert!(table.validate_textures(&[textures[0].as_slice()]).is_err());
        assert!(
            table
                .validate_textures(&[textures[1].as_slice(), textures[0].as_slice()])
                .is_err()
        );
    }
    #[test]
    fn truncation_lengths_counts_material_binding_and_hashes_fail_closed() {
        let (materials, bytes, _) = fixture();
        for length in 0..bytes.len() {
            assert!(
                TextureTable::decode(&bytes[..length], None, &materials, DEFAULT_BUDGET).is_err()
            );
        }
        assert!(TextureTable::decode(&bytes, None, &materials, bytes.len() - 1).is_err());
        assert!(TextureTable::decode(&bytes, Some([0; 32]), &materials, DEFAULT_BUDGET).is_err());
        for offset in [0, 8, 12, 16, 20, 24, 56, HEADER_BYTES] {
            let mut bad = bytes.clone();
            bad[offset] ^= 0x80;
            assert!(TextureTable::decode(&bad, None, &materials, DEFAULT_BUDGET).is_err());
        }
        let mut extra = bytes.clone();
        extra.push(0);
        assert!(TextureTable::decode(&extra, None, &materials, DEFAULT_BUDGET).is_err());
    }
    #[test]
    fn fresh_hashes_do_not_admit_bad_mappings_duplicate_ids_metadata_or_alpha() {
        let (materials, bytes, mut textures) = fixture();
        let start = HEADER_BYTES + 8;
        for offset in [HEADER_BYTES, HEADER_BYTES + 5] {
            let mut bad = bytes.clone();
            bad[offset] = 0xff;
            rehash(&mut bad);
            assert!(TextureTable::decode(&bad, None, &materials, DEFAULT_BUDGET).is_err());
        }
        let mut oversized = bytes.clone();
        oversized[start + 32..start + 40].copy_from_slice(&u64::MAX.to_le_bytes());
        rehash(&mut oversized);
        assert!(TextureTable::decode(&oversized, None, &materials, DEFAULT_BUDGET).is_err());
        let mut wrong_length = bytes.clone();
        wrong_length[start + 32..start + 40]
            .copy_from_slice(&(textures[0].len() as u64 + 1).to_le_bytes());
        rehash(&mut wrong_length);
        let admitted =
            TextureTable::decode(&wrong_length, None, &materials, DEFAULT_BUDGET).unwrap();
        assert!(
            admitted
                .validate_textures(&textures.iter().map(Vec::as_slice).collect::<Vec<_>>())
                .is_err()
        );
        let mut duplicate = bytes.clone();
        duplicate[start + 40..start + 72].copy_from_slice(&bytes[start..start + 32]);
        rehash(&mut duplicate);
        assert!(TextureTable::decode(&duplicate, None, &materials, DEFAULT_BUDGET).is_err());
        for mapping in [[0, 0, 0], [0, 1, 2], [u32::MAX, 0, 1]] {
            let bad = encode(&materials, &textures, mapping);
            assert!(TextureTable::decode(&bad, None, &materials, DEFAULT_BUDGET).is_err());
        }
        *textures[0].last_mut().unwrap() = 254;
        textures.sort_by_key(|v| content_id(v));
        let rebound = encode(&materials, &textures, [0, 1, 0]);
        let table = TextureTable::decode(&rebound, None, &materials, DEFAULT_BUDGET).unwrap();
        assert!(
            table
                .validate_textures(&textures.iter().map(Vec::as_slice).collect::<Vec<_>>())
                .is_err()
        );
    }
}
