//! Complete authored KTX2 mip chains, separate from single-mip runtime admission.
//! Bounded opaque RGBA8 sRGB; no transcoding, filtering or mip-generation policy.
use crate::{AssetError, Result, content_id, texture, word};

#[derive(Debug)]
pub struct MipLevel<'a> {
    pub width: u32,
    pub height: u32,
    pub texels: &'a [u8],
}
#[derive(Debug)]
pub struct TextureMipChain<'a> {
    pub content_id: [u8; 32],
    pub width: u32,
    pub height: u32,
    /// Base level first, independently of smallest-first physical file ordering.
    pub levels: Vec<MipLevel<'a>>,
    pub writer: &'a str,
    pub provenance: Option<&'a str>,
}
impl<'a> TextureMipChain<'a> {
    pub fn decode(bytes: &'a [u8], expected: Option<[u8; 32]>, budget: usize) -> Result<Self> {
        if bytes.len() > budget || bytes.len() < 104 || &bytes[..12] != b"\xabKTX 20\xbb\r\n\x1a\n"
        {
            return Err(AssetError("invalid mip KTX2 header or byte budget"));
        }
        let width = word(bytes, 20)?;
        let height = word(bytes, 24)?;
        let count = word(bytes, 40)?;
        if !(1..=texture::MAX_DIMENSION).contains(&width)
            || !(1..=texture::MAX_DIMENSION).contains(&height)
            || count != u32::BITS - width.max(height).leading_zeros()
            || word(bytes, 12)? != 43
            || word(bytes, 16)? != 1
            || word(bytes, 28)? != 0
            || word(bytes, 32)? != 0
            || word(bytes, 36)? != 1
            || word(bytes, 44)? != 0
            || texture::long(bytes, 64)? != 0
            || texture::long(bytes, 72)? != 0
        {
            return Err(AssetError("unsupported complete mip KTX2 profile"));
        }
        // Count is now bounded to 1..11 before any allocation/index arithmetic.
        let dfd = 80 + count as usize * 24;
        let metadata_start = dfd + 92;
        let metadata_bytes = word(bytes, 60)? as usize;
        if word(bytes, 48)? as usize != dfd
            || word(bytes, 52)? != 92
            || word(bytes, 56)? as usize != metadata_start
            || metadata_bytes == 0
            || metadata_bytes > 16384
            || !metadata_bytes.is_multiple_of(4)
        {
            return Err(AssetError("invalid mip KTX sections"));
        }
        let start = metadata_start
            .checked_add(metadata_bytes)
            .filter(|&v| v <= bytes.len())
            .ok_or(AssetError("truncated mip KTX sections"))?;
        let (writer, provenance) =
            texture::sections(&bytes[dfd..metadata_start], &bytes[metadata_start..start])?;
        let mut cursor = start;
        let mut levels = Vec::with_capacity(count as usize);
        for level in (0..count).rev() {
            let w = (width >> level).max(1);
            let h = (height >> level).max(1);
            let size = (w as usize)
                .checked_mul(h as usize)
                .and_then(|v| v.checked_mul(4))
                .ok_or(AssetError("mip byte size overflow"))?;
            let index = 80 + level as usize * 24;
            if texture::long(bytes, index)? != cursor as u64
                || texture::long(bytes, index + 8)? != size as u64
                || texture::long(bytes, index + 16)? != size as u64
            {
                return Err(AssetError("invalid mip level ordering/ranges/lengths"));
            }
            let end = cursor
                .checked_add(size)
                .filter(|&v| v <= bytes.len())
                .ok_or(AssetError("truncated mip texels"))?;
            let texels = &bytes[cursor..end];
            if texels.as_chunks::<4>().0.iter().any(|v| v[3] != 255) {
                return Err(AssetError("mip KTX profile requires opaque texels"));
            }
            levels.push(MipLevel {
                width: w,
                height: h,
                texels,
            });
            cursor = end;
        }
        if cursor != bytes.len() {
            return Err(AssetError("trailing mip KTX bytes"));
        }
        let id = content_id(bytes);
        if expected.is_some_and(|v| v != id) {
            return Err(AssetError("mip KTX content digest mismatch"));
        }
        levels.reverse();
        Ok(Self {
            content_id: id,
            width,
            height,
            levels,
            writer,
            provenance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = include_bytes!("../../../fixtures/textures/opaque_4x4_mips.ktx2");

    #[test]
    fn official_complete_chain_borrows_exact_level_texels_and_preserves_legacy_gate() {
        let chain =
            TextureMipChain::decode(FIXTURE, Some(content_id(FIXTURE)), texture::DEFAULT_BUDGET)
                .unwrap();
        assert_eq!((chain.width, chain.height), (4, 4));
        assert_eq!(chain.levels.len(), 3);
        for (index, mip) in chain.levels.iter().enumerate() {
            let side = 4 >> index;
            assert_eq!((mip.width, mip.height), (side, side));
            assert_eq!(
                mip.texels,
                [index as u8 * 70 + 10, 50, 120, 255].repeat((side * side) as usize)
            );
            let offset = texture::long(FIXTURE, 80 + index * 24).unwrap() as usize;
            assert_eq!(mip.texels.as_ptr(), FIXTURE[offset..].as_ptr());
        }
        assert_eq!(chain.writer, "ktx create v4.4.2 / libktx v4.4.2");
        assert!(chain.provenance.is_none());
        assert!(texture::Texture::decode(FIXTURE, None, texture::DEFAULT_BUDGET).is_err());
        let legacy = include_bytes!("../../../fixtures/textures/opaque_2x2.ktx2");
        assert!(TextureMipChain::decode(legacy, None, texture::DEFAULT_BUDGET).is_err());
    }
    #[test]
    fn every_truncation_and_invalid_counts_offsets_lengths_hash_alpha_or_metadata_fails() {
        for end in 0..FIXTURE.len() {
            assert!(
                TextureMipChain::decode(&FIXTURE[..end], None, texture::DEFAULT_BUDGET).is_err()
            );
        }
        for offset in [
            0, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 72, 80, 88, 96, 104, 112,
            120, 128, 136, 144, 152, 160, 168, 180, 244, 252, 265, 319, 323, 339,
        ] {
            let mut bad = FIXTURE.to_vec();
            bad[offset] ^= 0x80;
            assert!(
                TextureMipChain::decode(&bad, None, texture::DEFAULT_BUDGET).is_err(),
                "offset={offset}"
            );
        }
        assert!(TextureMipChain::decode(FIXTURE, None, FIXTURE.len() - 1).is_err());
        assert!(TextureMipChain::decode(FIXTURE, Some([0; 32]), texture::DEFAULT_BUDGET).is_err());
        let mut extra = FIXTURE.to_vec();
        extra.push(0);
        assert!(TextureMipChain::decode(&extra, None, texture::DEFAULT_BUDGET).is_err());
        let mut swapped = FIXTURE.to_vec();
        swapped[80..88].copy_from_slice(&320u64.to_le_bytes());
        swapped[104..112].copy_from_slice(&336u64.to_le_bytes());
        assert!(TextureMipChain::decode(&swapped, None, texture::DEFAULT_BUDGET).is_err());
    }
    fn synthetic(width: u32, height: u32) -> Vec<u8> {
        let count = u32::BITS - width.max(height).leading_zeros();
        let dfd = 80 + count as usize * 24;
        let metadata_start = dfd + 92;
        let start = metadata_start + 72;
        let mut bytes = FIXTURE[..80].to_vec();
        bytes.resize(dfd, 0);
        bytes.extend_from_slice(&FIXTURE[152..316]);
        for (offset, value) in [
            (20, width),
            (24, height),
            (40, count),
            (48, dfd as u32),
            (56, metadata_start as u32),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        let mut cursor = start;
        for level in (0..count).rev() {
            let size = (width >> level).max(1) as usize * (height >> level).max(1) as usize * 4;
            for (offset, value) in [
                (80 + level as usize * 24, cursor as u64),
                (88 + level as usize * 24, size as u64),
                (96 + level as usize * 24, size as u64),
            ] {
                bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
            }
            bytes.extend_from_slice(&[10, 50, 120, 255].repeat(size / 4));
            cursor += size;
        }
        bytes
    }
    #[test]
    fn odd_rectangular_dimensions_single_tail_and_maximum_level_count_are_bounded() {
        for (width, height, count) in [(3, 5, 3), (1, 1, 1), (1024, 1, 11), (1, 1024, 11)] {
            let bytes = synthetic(width, height);
            let chain = TextureMipChain::decode(&bytes, None, texture::DEFAULT_BUDGET).unwrap();
            assert_eq!(chain.levels.len(), count);
            assert_eq!(
                (
                    chain.levels.last().unwrap().width,
                    chain.levels.last().unwrap().height
                ),
                (1, 1)
            );
        }
        // The full square chain exceeds 4 MiB even though base dimensions fit.
        let bytes = synthetic(1024, 1024);
        assert!(TextureMipChain::decode(&bytes, None, texture::DEFAULT_BUDGET).is_err());
    }
    #[test]
    fn bounded_arbitrary_inputs_never_panic() {
        let mut seed = 719u64;
        for _ in 0..10000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let mut bytes = FIXTURE.to_vec();
            let offset = ((seed >> 32) as usize) % bytes.len();
            bytes[offset] ^= (seed as u8) | 1;
            let _ = TextureMipChain::decode(&bytes, None, texture::DEFAULT_BUDGET);
        }
    }
}
