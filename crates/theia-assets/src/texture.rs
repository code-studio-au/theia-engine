//! Bounded KTX2 profile: opaque, resident 2D RGBA8 sRGB, one mip, top-left origin.
//! This is deliberately not a general KTX decoder or runtime transcoder.
use crate::{AssetError, Result, content_id, word};
pub const DEFAULT_BUDGET: usize = 4 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 1024;
const MAGIC: &[u8; 12] = b"\xabKTX 20\xbb\r\n\x1a\n";
#[derive(Debug)]
pub struct Texture<'a> {
    pub content_id: [u8; 32],
    pub width: u32,
    pub height: u32,
    pub texels: &'a [u8],
    pub writer: &'a str,
    pub provenance: Option<&'a str>,
}
pub(super) fn long(bytes: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        bytes
            .get(offset..offset + 8)
            .ok_or(AssetError("truncated KTX index"))?
            .try_into()
            .unwrap(),
    ))
}
impl<'a> Texture<'a> {
    pub fn decode(bytes: &'a [u8], expected: Option<[u8; 32]>, budget: usize) -> Result<Self> {
        if bytes.len() > budget || bytes.len() < 104 || &bytes[..12] != MAGIC {
            return Err(AssetError("invalid KTX2 header or byte budget"));
        }
        let width = word(bytes, 20)?;
        let height = word(bytes, 24)?;
        if !(1..=MAX_DIMENSION).contains(&width)
            || !(1..=MAX_DIMENSION).contains(&height)
            || word(bytes, 12)? != 43
            || word(bytes, 16)? != 1
            || word(bytes, 28)? != 0
            || word(bytes, 32)? != 0
            || word(bytes, 36)? != 1
            || word(bytes, 40)? != 1
            || word(bytes, 44)? != 0
            || word(bytes, 48)? != 104
            || word(bytes, 52)? != 92
            || word(bytes, 56)? != 196
            || long(bytes, 64)? != 0
            || long(bytes, 72)? != 0
        {
            return Err(AssetError("unsupported KTX2 texture profile"));
        }
        let metadata = word(bytes, 60)? as usize;
        if metadata == 0 || metadata > 16384 || !metadata.is_multiple_of(4) {
            return Err(AssetError("invalid KTX metadata size"));
        }
        let start = 196usize
            .checked_add(metadata)
            .ok_or(AssetError("KTX size overflow"))?;
        let size = (width as usize)
            .checked_mul(height as usize)
            .and_then(|v| v.checked_mul(4))
            .ok_or(AssetError("KTX size overflow"))?;
        if long(bytes, 80)? != start as u64
            || long(bytes, 88)? != size as u64
            || long(bytes, 96)? != size as u64
            || start.checked_add(size) != Some(bytes.len())
        {
            return Err(AssetError("invalid KTX level ranges/lengths"));
        }
        let (writer, provenance) = sections(&bytes[104..196], &bytes[196..start])?;
        let texels = &bytes[start..];
        if texels.as_chunks::<4>().0.iter().any(|v| v[3] != 255) {
            return Err(AssetError("KTX qualification requires opaque texels"));
        }
        let id = content_id(bytes);
        if expected.is_some_and(|v| v != id) {
            return Err(AssetError("KTX content digest mismatch"));
        }
        Ok(Self {
            content_id: id,
            width,
            height,
            texels,
            writer,
            provenance,
        })
    }
}
/// Shared strict sRGB DFD/key-value admission for the two bounded texture profiles.
pub(super) fn sections<'a>(dfd: &[u8], metadata: &'a [u8]) -> Result<(&'a str, Option<&'a str>)> {
    if dfd.len() != 92
        || metadata.is_empty()
        || metadata.len() > 16384
        || !metadata.len().is_multiple_of(4)
    {
        return Err(AssetError("invalid KTX DFD/metadata section size"));
    }
    let start = metadata.len();
    // Canonical basic DFD for R8G8B8A8_SRGB: RGBSDA/BT709/sRGB, straight
    // alpha, 1x1 block, four bytes, samples R/G/B/A at bits 0/8/16/24.
    let prefix = [
        92, 0, 0, 0, 0, 0, 0, 0, 2, 0, 88, 0, 1, 1, 2, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
    ];
    if dfd[..28] != prefix {
        return Err(AssetError("unsupported KTX data format descriptor"));
    }
    for (i, channel) in [0u8, 1, 2, 31].into_iter().enumerate() {
        let sample = &dfd[28 + i * 16..44 + i * 16];
        let required = [
            i as u8 * 8,
            0,
            7,
            channel,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            255,
            0,
            0,
            0,
        ];
        if sample != required {
            return Err(AssetError("unsupported KTX channel descriptor"));
        }
    }
    let mut offset = 0;
    let mut orientation = None;
    let mut writer = None;
    let mut provenance = None;
    while offset < start {
        let length = word(metadata, offset)? as usize;
        let begin = offset
            .checked_add(4)
            .ok_or(AssetError("KTX metadata overflow"))?;
        let end = begin
            .checked_add(length)
            .filter(|&v| v <= start)
            .ok_or(AssetError("KTX metadata range exceeds section"))?;
        let padded = crate::aligned(end)?;
        if length == 0 || padded > start || metadata[end..padded].iter().any(|&v| v != 0) {
            return Err(AssetError("invalid KTX metadata padding/length"));
        }
        let entry = &metadata[begin..end];
        let split = entry
            .iter()
            .position(|&v| v == 0)
            .ok_or(AssetError("missing KTX key terminator"))?;
        let key = std::str::from_utf8(&entry[..split])
            .map_err(|_| AssetError("invalid KTX key UTF-8"))?;
        let value = entry
            .get(split + 1..)
            .and_then(|v| v.strip_suffix(&[0]))
            .ok_or(AssetError("missing KTX value terminator"))?;
        if value.contains(&0) {
            return Err(AssetError("embedded KTX value terminator"));
        }
        let value =
            std::str::from_utf8(value).map_err(|_| AssetError("invalid KTX value UTF-8"))?;
        let slot = match key {
            "KTXorientation" => &mut orientation,
            "KTXwriter" => &mut writer,
            "THEIAprovenance" => &mut provenance,
            _ => return Err(AssetError("unsupported KTX metadata key")),
        };
        if value.is_empty() || slot.replace(value).is_some() {
            return Err(AssetError("empty or duplicate KTX metadata"));
        }
        offset = padded;
    }
    if orientation != Some("rd") || writer.is_none() {
        return Err(AssetError(
            "KTX profile requires top-left orientation and writer",
        ));
    }
    Ok((writer.unwrap(), provenance))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = include_bytes!("../../../fixtures/textures/opaque_2x2.ktx2");
    #[test]
    fn official_ktx_writer_preserves_exact_opaque_srgb_texels_and_orientation() {
        let texture = Texture::decode(FIXTURE, Some(content_id(FIXTURE)), DEFAULT_BUDGET).unwrap();
        assert_eq!((texture.width, texture.height), (2, 2));
        assert_eq!(
            texture.texels,
            &[
                240, 32, 48, 255, 20, 180, 64, 255, 35, 65, 210, 255, 210, 150, 30, 255
            ]
        );
        assert_eq!(texture.writer, "ktx create v4.4.2 / libktx v4.4.2");
        assert!(texture.provenance.is_none());
    }
    #[test]
    fn every_truncation_and_invalid_budget_digest_layout_or_profile_fail_closed() {
        for end in 0..FIXTURE.len() {
            assert!(Texture::decode(&FIXTURE[..end], None, DEFAULT_BUDGET).is_err());
        }
        assert!(Texture::decode(FIXTURE, None, FIXTURE.len() - 1).is_err());
        assert!(Texture::decode(FIXTURE, Some([0; 32]), DEFAULT_BUDGET).is_err());
        for offset in [
            0,
            12,
            16,
            20,
            24,
            28,
            32,
            36,
            40,
            44,
            48,
            52,
            56,
            60,
            64,
            72,
            80,
            88,
            96,
            104,
            108,
            112,
            116,
            120,
            124,
            132,
            135,
            144,
            160,
            176,
            196,
            204,
            216,
            268 + 3,
        ] {
            let mut bad = FIXTURE.to_vec();
            bad[offset] ^= 0x80;
            assert!(
                Texture::decode(&bad, None, DEFAULT_BUDGET).is_err(),
                "offset {offset}"
            );
        }
        let mut extra = FIXTURE.to_vec();
        extra.push(0);
        assert!(Texture::decode(&extra, None, DEFAULT_BUDGET).is_err());
    }
    #[test]
    fn metadata_cannot_reinterpret_orientation_channels_alpha_or_colour_space() {
        for (needle, replacement) in [
            (b"rd\0".as_slice(), b"ru\0".as_slice()),
            (b"KTXwriter".as_slice(), b"KTXswizzl".as_slice()),
        ] {
            let mut bad = FIXTURE.to_vec();
            let start = bad.windows(needle.len()).position(|v| v == needle).unwrap();
            bad[start..start + needle.len()].copy_from_slice(replacement);
            assert!(Texture::decode(&bad, None, DEFAULT_BUDGET).is_err());
        }
        // Duplicate orientation, unterminated/invalid UTF-8 value, padding.
        for offset in [216, 217, 218, 219, 220, 221] {
            let mut bad = FIXTURE.to_vec();
            bad[offset] = 0xff;
            assert!(Texture::decode(&bad, None, DEFAULT_BUDGET).is_err());
        }
        let mut duplicate = FIXTURE[..268].to_vec();
        duplicate.extend_from_slice(&FIXTURE[196..220]);
        duplicate.extend_from_slice(&FIXTURE[268..]);
        duplicate[60..64].copy_from_slice(&96u32.to_le_bytes());
        duplicate[80..88].copy_from_slice(&292u64.to_le_bytes());
        assert!(Texture::decode(&duplicate, None, DEFAULT_BUDGET).is_err());
    }
}
