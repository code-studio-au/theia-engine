//! CPU contract for a contiguous coarse mip tail of an admitted immutable chain.
//! Budgets count dense texels, never Vulkan physical allocation or encoded bytes.
use crate::{
    AssetError, Result,
    texture::MAX_DIMENSION,
    texture_mips::{MipLevel, TextureMipChain},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selection {
    /// Level in the original authored chain.
    pub source_level: u32,
    /// Level relative to a conventional image containing only the resident tail.
    pub view_level: u32,
    pub coarsened: bool,
}

#[derive(Debug)]
pub struct MipTail<'chain, 'texels> {
    chain: &'chain TextureMipChain<'texels>,
    first: usize,
    texel_bytes: usize,
}

impl<'chain, 'texels> MipTail<'chain, 'texels> {
    /// Finest complete tail whose dense texels fit. The 1x1 tail is mandatory;
    /// budgets below four bytes fail. Existing owners/data are never mutated.
    /// The caller still performs encoded hash/provenance admission and actual
    /// device allocation checks; public chain fields are rechecked for safe use.
    pub fn fit(chain: &'chain TextureMipChain<'texels>, budget: usize) -> Result<Self> {
        if !(1..=MAX_DIMENSION).contains(&chain.width)
            || !(1..=MAX_DIMENSION).contains(&chain.height)
            || chain.levels.len()
                != (u32::BITS - chain.width.max(chain.height).leading_zeros()) as usize
        {
            return Err(AssetError("unsupported resident mip chain profile"));
        }
        for (index, level) in chain.levels.iter().enumerate() {
            let width = (chain.width >> index).max(1);
            let height = (chain.height >> index).max(1);
            if (level.width, level.height) != (width, height)
                || level.texels.len() != width as usize * height as usize * 4
                || level.texels.as_chunks::<4>().0.iter().any(|p| p[3] != 255)
            {
                return Err(AssetError(
                    "invalid resident mip level dimensions/texels/alpha",
                ));
            }
        }
        let mut first = chain.levels.len();
        let mut texel_bytes = 0usize;
        for (index, level) in chain.levels.iter().enumerate().rev() {
            let next = texel_bytes
                .checked_add(level.texels.len())
                .ok_or(AssetError("resident mip texel byte overflow"))?;
            if next > budget {
                break;
            }
            first = index;
            texel_bytes = next;
        }
        if first == chain.levels.len() {
            return Err(AssetError(
                "resident mip budget cannot retain mandatory 1x1 tail",
            ));
        }
        Ok(Self {
            chain,
            first,
            texel_bytes,
        })
    }
    pub fn first_level(&self) -> u32 {
        self.first as u32
    }
    pub fn texel_bytes(&self) -> usize {
        self.texel_bytes
    }
    pub fn source_extent(&self) -> [u32; 2] {
        [self.chain.width, self.chain.height]
    }
    pub fn source_level_count(&self) -> u32 {
        self.chain.levels.len() as u32
    }
    pub fn source_id(&self) -> [u8; 32] {
        self.chain.content_id
    }
    pub fn levels(&self) -> &[MipLevel<'texels>] {
        &self.chain.levels[self.first..]
    }
    pub fn resident_extent(&self) -> [u32; 2] {
        [self.levels()[0].width, self.levels()[0].height]
    }
    /// First choose the desired level using original base dimensions, then apply
    /// this fallback. Using the smaller resident extent for footprint evaluation
    /// would under-estimate demand. No arbitrary holes or finer substitution.
    pub fn select(&self, requested: u32) -> Result<Selection> {
        if requested >= self.source_level_count() {
            return Err(AssetError("requested mip outside original complete chain"));
        }
        let source_level = requested.max(self.first_level());
        Ok(Selection {
            source_level,
            view_level: source_level - self.first_level(),
            coarsened: source_level != requested,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_and_short_budgets_keep_protected_tail_and_borrow_original_texels() {
        let bytes = include_bytes!("../../../fixtures/textures/opaque_4x4_mips.ktx2");
        let chain = TextureMipChain::decode(bytes, None, crate::texture::DEFAULT_BUDGET).unwrap();
        for (budget, first, size) in [
            (4, 2, 4),
            (19, 2, 4),
            (20, 1, 20),
            (83, 1, 20),
            (84, 0, 84),
            (usize::MAX, 0, 84),
        ] {
            let tail = MipTail::fit(&chain, budget).unwrap();
            assert_eq!((tail.first_level(), tail.texel_bytes()), (first, size));
            assert_eq!(tail.source_id(), crate::content_id(bytes));
            assert_eq!(tail.source_extent(), [4, 4]);
            assert_eq!(tail.resident_extent(), [4 >> first, 4 >> first]);
            assert_eq!(
                tail.levels()[0].texels.as_ptr(),
                chain.levels[first as usize].texels.as_ptr()
            );
            for requested in 0..3 {
                let selected = tail.select(requested).unwrap();
                assert_eq!(selected.source_level, requested.max(first));
                assert_eq!(selected.view_level, requested.max(first) - first);
                assert_eq!(selected.coarsened, requested < first);
            }
            assert!(tail.select(3).is_err());
            assert!(tail.select(u32::MAX).is_err());
        }
        for budget in 0..4 {
            assert!(MipTail::fit(&chain, budget).is_err());
        }
        assert_eq!(chain.levels.len(), 3);
    }
    fn synthetic(width: u32, height: u32) -> TextureMipChain<'static> {
        // Reuse one immutable opaque array; each represented level borrows its
        // exact prefix. No heap leaks or native texture producer in unit tests.
        static TEXELS: [u8; 4096] = [255; 4096];
        let levels = (0..u32::BITS - width.max(height).leading_zeros())
            .map(|i| {
                let w = (width >> i).max(1);
                let h = (height >> i).max(1);
                MipLevel {
                    width: w,
                    height: h,
                    texels: &TEXELS[..w as usize * h as usize * 4],
                }
            })
            .collect();
        TextureMipChain {
            content_id: [7; 32],
            width,
            height,
            levels,
            writer: "test",
            provenance: None,
        }
    }
    #[test]
    fn all_small_odd_rectangles_and_budgets_match_independent_suffix_search() {
        for width in 1..=9 {
            for height in 1..=9 {
                let chain = synthetic(width, height);
                let full: usize = chain.levels.iter().map(|m| m.texels.len()).sum();
                for budget in 0..=full + 1 {
                    let expected = (0..chain.levels.len()).find(|&i| {
                        chain.levels[i..]
                            .iter()
                            .map(|m| m.texels.len())
                            .sum::<usize>()
                            <= budget
                    });
                    let actual = MipTail::fit(&chain, budget);
                    if let Some(first) = expected {
                        let actual = actual.unwrap();
                        assert_eq!(actual.first_level() as usize, first);
                        assert!(actual.texel_bytes() <= budget);
                        assert_eq!(
                            actual.levels().last().map(|m| (m.width, m.height)),
                            Some((1, 1))
                        );
                        for requested in 0..chain.levels.len() as u32 {
                            let selected = actual.select(requested).unwrap();
                            assert_eq!(selected.source_level, requested.max(first as u32));
                            assert!((selected.view_level as usize) < actual.levels().len());
                        }
                    } else {
                        assert!(actual.is_err());
                    }
                }
            }
        }
    }
    #[test]
    fn maximum_thin_chain_and_footprint_use_original_base_dimensions() {
        for (width, height) in [(1024, 1), (1, 1024)] {
            let chain = synthetic(width, height);
            let tail = MipTail::fit(&chain, 508).unwrap();
            assert_eq!(tail.first_level(), 4);
            assert_eq!(tail.texel_bytes(), 508);
            assert_eq!(tail.resident_extent(), [width.min(64), height.min(64)]);
            let dx = if width == 1024 {
                [1. / 64., 0.]
            } else {
                [0., 1. / 64.]
            };
            let gradient = crate::texture_footprint::Differential {
                uv: [0.; 2],
                dx,
                dy: [0.; 2],
            };
            let wanted = crate::texture_footprint::choose_mip(
                gradient,
                tail.source_extent(),
                tail.source_level_count(),
            )
            .unwrap();
            assert_eq!(wanted.mip_level, 4);
            assert_eq!(tail.select(wanted.mip_level).unwrap().view_level, 0);
            assert_eq!(MipTail::fit(&chain, 507).unwrap().first_level(), 5);
        }
    }
    #[test]
    fn forged_public_chains_reject_before_shifts_or_indexing() {
        let mut chain = synthetic(4, 4);
        chain.width = u32::MAX;
        assert!(MipTail::fit(&chain, usize::MAX).is_err());
        chain = synthetic(4, 4);
        chain.levels.pop();
        assert!(MipTail::fit(&chain, usize::MAX).is_err());
        chain = synthetic(4, 4);
        chain.levels[1].width = 3;
        assert!(MipTail::fit(&chain, usize::MAX).is_err());
        chain = synthetic(4, 4);
        chain.levels[0].texels = &[255, 255, 255, 0];
        assert!(MipTail::fit(&chain, usize::MAX).is_err());
        chain = synthetic(1, 1);
        chain.levels[0].texels = &[255, 255, 255, 0];
        assert!(MipTail::fit(&chain, usize::MAX).is_err());
        chain = synthetic(1, 1);
        chain.width = 0;
        assert!(MipTail::fit(&chain, usize::MAX).is_err());
    }
}
