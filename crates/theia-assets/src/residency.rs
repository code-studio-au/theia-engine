//! CPU reference for immutable page publication. No Vulkan completion is implied.
use crate::{AssetError, Mesh, Result};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

pub type PageId = [u8; 32];
const MAX_PAGES: usize = 4096;
static NEXT_STORE: AtomicU64 = AtomicU64::new(1);

/// Version 1: THEIACEL, u32 version/count, u64 cell identity, then hash/u64 byte length.
#[derive(Debug)]
pub struct Manifest {
    cell: u64,
    pages: BTreeMap<PageId, usize>,
}
impl Manifest {
    pub fn decode(bytes: &[u8], expected: PageId) -> Result<Self> {
        if bytes.len() < 24 || &bytes[..8] != b"THEIACEL" || crate::content_id(bytes) != expected {
            return Err(AssetError("invalid cell manifest header/content address"));
        }
        if crate::word(bytes, 8)? != 1 {
            return Err(AssetError("unsupported cell manifest version"));
        }
        let count = crate::word(bytes, 12)? as usize;
        if count == 0 || count > MAX_PAGES || bytes.len() != 24 + count * 40 {
            return Err(AssetError("invalid cell manifest length/count"));
        }
        let cell = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let mut pages = BTreeMap::new();
        let mut previous = None;
        for record in bytes[24..].as_chunks::<40>().0 {
            let id: PageId = record[..32].try_into().unwrap();
            let size = usize::try_from(u64::from_le_bytes(record[32..].try_into().unwrap()))
                .map_err(|_| AssetError("page length exceeds platform range"))?;
            if !(crate::HEADER_BYTES..=crate::DEFAULT_BUDGET).contains(&size)
                || previous.is_some_and(|p| p >= id)
            {
                return Err(AssetError("invalid page length or noncanonical page order"));
            }
            pages.insert(id, size);
            previous = Some(id);
        }
        Ok(Self { cell, pages })
    }
    pub fn pages(&self) -> &BTreeMap<PageId, usize> {
        &self.pages
    }
    pub fn cell(&self) -> u64 {
        self.cell
    }
}

#[derive(Clone, Default)]
pub struct Snapshot {
    epoch: u64,
    pages: BTreeMap<PageId, Arc<[u8]>>,
}
impl Snapshot {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn page(&self, id: &PageId) -> Option<&[u8]> {
        self.pages.get(id).map(AsRef::as_ref)
    }
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}
/// Staging is validation only. A GPU backend must separately prove upload completion.
pub struct StagedPage {
    owner: u64,
    id: PageId,
    bytes: Arc<[u8]>,
}
pub struct Residency {
    owner: u64,
    budget: usize,
    allocations: BTreeMap<PageId, Weak<[u8]>>,
    current: Snapshot,
}
impl Residency {
    pub fn new(budget: usize) -> Result<Self> {
        let owner = NEXT_STORE
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| AssetError("residency identity exhausted"))?;
        Ok(Self {
            owner,
            budget,
            allocations: BTreeMap::new(),
            current: Snapshot::default(),
        })
    }
    /// Unique live encoded bytes, including staged pages and retained snapshots.
    /// This excludes map/Arc/decoder overhead and is not a physical GPU memory budget.
    pub fn live_bytes(&self) -> usize {
        self.allocations
            .values()
            .filter_map(Weak::upgrade)
            .map(|v| v.len())
            .sum()
    }
    pub fn snapshot(&self) -> Snapshot {
        self.current.clone()
    }
    pub fn stage(&mut self, manifest: &Manifest, id: PageId, bytes: &[u8]) -> Result<StagedPage> {
        if manifest.pages.get(&id) != Some(&bytes.len()) {
            return Err(AssetError("page absent or length differs from manifest"));
        }
        Mesh::decode(bytes, Some(id), crate::DEFAULT_BUDGET)?;
        self.allocations
            .retain(|_, value| value.strong_count() != 0);
        let allocation = if let Some(existing) = self.allocations.get(&id).and_then(Weak::upgrade) {
            existing
        } else {
            if self
                .live_bytes()
                .checked_add(bytes.len())
                .is_none_or(|v| v > self.budget)
            {
                return Err(AssetError("live page byte budget exhausted"));
            }
            let allocation: Arc<[u8]> = Arc::from(bytes);
            self.allocations.insert(id, Arc::downgrade(&allocation));
            allocation
        };
        Ok(StagedPage {
            owner: self.owner,
            id,
            bytes: allocation,
        })
    }
    /// Atomic CPU publication; invalid removals leave the current epoch unchanged.
    pub fn publish(&mut self, staged: Option<StagedPage>, remove: &[PageId]) -> Result<Snapshot> {
        if staged.as_ref().is_some_and(|v| v.owner != self.owner) {
            return Err(AssetError("foreign staged page"));
        }
        let mut pages = self.current.pages.clone();
        for id in remove {
            if pages.remove(id).is_none() {
                return Err(AssetError("missing or duplicate page removal"));
            }
        }
        if let Some(staged) = staged {
            pages.insert(staged.id, staged.bytes);
        }
        let epoch = self
            .current
            .epoch
            .checked_add(1)
            .ok_or(AssetError("residency epoch exhausted"))?;
        self.current = Snapshot { epoch, pages };
        Ok(self.snapshot())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(tag: u8) -> (Vec<u8>, PageId, Manifest) {
        let mut bytes = crate::tests::fixture();
        bytes[crate::HEADER_BYTES] = tag;
        let digest = crate::content_id(&bytes[crate::HEADER_BYTES..]);
        bytes[44..76].copy_from_slice(&digest);
        let id = crate::content_id(&bytes);
        let mut manifest = b"THEIACEL".to_vec();
        manifest.extend(1u32.to_le_bytes());
        manifest.extend(1u32.to_le_bytes());
        manifest.extend(7u64.to_le_bytes());
        manifest.extend(id);
        manifest.extend((bytes.len() as u64).to_le_bytes());
        let decoded = Manifest::decode(&manifest, crate::content_id(&manifest)).unwrap();
        (bytes, id, decoded)
    }
    #[test]
    fn retained_readers_block_reuse_until_last_release() {
        let (a, aid, ma) = fixture(b'a');
        let (b, bid, mb) = fixture(b'b');
        let mut store = Residency::new(a.len()).unwrap();
        let staged = store.stage(&ma, aid, &a).unwrap();
        let reader = store.publish(Some(staged), &[]).unwrap();
        let empty = store.publish(None, &[aid]).unwrap();
        assert_eq!(empty.page_count(), 0);
        assert_eq!(reader.page(&aid), Some(a.as_slice()));
        assert!(store.stage(&mb, bid, &b).is_err());
        drop(reader);
        assert_eq!(store.live_bytes(), 0);
        let staged = store.stage(&mb, bid, &b).unwrap();
        assert_eq!(store.publish(Some(staged), &[]).unwrap().epoch, 3);
    }
    #[test]
    fn staged_reservations_deduplicate_and_cancel() {
        let (a, id, m) = fixture(b'a');
        let mut store = Residency::new(a.len()).unwrap();
        let first = store.stage(&m, id, &a).unwrap();
        let second = store.stage(&m, id, &a).unwrap();
        assert_eq!(store.live_bytes(), a.len());
        drop(first);
        assert_eq!(store.live_bytes(), a.len());
        drop(second);
        assert_eq!(store.live_bytes(), 0);
        let mut corrupt = a.clone();
        corrupt[80] ^= 1;
        assert!(store.stage(&m, id, &corrupt).is_err());
        assert_eq!(store.live_bytes(), 0);
    }
    #[test]
    fn failed_publication_is_atomic_and_foreign_pages_rejected() {
        let (a, id, m) = fixture(b'a');
        let mut store = Residency::new(a.len()).unwrap();
        let mut other = Residency::new(a.len()).unwrap();
        let foreign = other.stage(&m, id, &a).unwrap();
        assert!(store.publish(Some(foreign), &[]).is_err());
        let staged = store.stage(&m, id, &a).unwrap();
        store.publish(Some(staged), &[]).unwrap();
        assert!(store.publish(None, &[id, id]).is_err());
        assert_eq!(store.snapshot().epoch, 1);
        assert_eq!(store.snapshot().page(&id), Some(a.as_slice()));
        store.current.epoch = u64::MAX;
        assert!(store.publish(None, &[id]).is_err());
        assert_eq!(store.snapshot().page_count(), 1);
    }
    #[test]
    fn malformed_manifests_fail_closed() {
        let (_, id, m) = fixture(b'a');
        assert_eq!(m.cell, 7);
        assert!(m.pages().contains_key(&id));
        let mut bytes = b"THEIACEL".to_vec();
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(2u32.to_le_bytes());
        bytes.extend(7u64.to_le_bytes());
        for _ in 0..2 {
            bytes.extend(id);
            bytes.extend(128u64.to_le_bytes());
        }
        assert!(Manifest::decode(&bytes, crate::content_id(&bytes)).is_err());
        for len in 0..bytes.len() {
            assert!(Manifest::decode(&bytes[..len], crate::content_id(&bytes[..len])).is_err());
        }
        bytes[8] = 2;
        assert!(Manifest::decode(&bytes, crate::content_id(&bytes)).is_err());
    }
    #[test]
    fn randomized_retention_matches_independent_live_set() {
        use std::collections::BTreeSet;
        let fixtures: Vec<_> = (b'a'..=b'd').map(fixture).collect();
        let size = fixtures[0].0.len();
        let mut store = Residency::new(size * 2).unwrap();
        let mut current = BTreeSet::new();
        let mut readers: Vec<(Snapshot, BTreeSet<PageId>)> = Vec::new();
        let mut seed = 719u64;
        for _ in 0..2000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let mut live = current.clone();
            for (_, ids) in &readers {
                live.extend(ids);
            }
            match (seed >> 32) % 4 {
                0 => {
                    let (bytes, id, manifest) = &fixtures[(seed as usize) % fixtures.len()];
                    let fits = live.contains(id) || live.len() < 2;
                    let staged = store.stage(manifest, *id, bytes);
                    assert_eq!(staged.is_ok(), fits);
                    if let Ok(staged) = staged {
                        store.publish(Some(staged), &[]).unwrap();
                        current.insert(*id);
                    }
                }
                1 => {
                    if let Some(id) = current.iter().next().copied() {
                        store.publish(None, &[id]).unwrap();
                        current.remove(&id);
                    }
                }
                2 => readers.push((store.snapshot(), current.clone())),
                _ => {
                    if !readers.is_empty() {
                        readers.swap_remove(seed as usize % readers.len());
                    }
                }
            }
            let mut live = current.clone();
            for (snapshot, ids) in &readers {
                live.extend(ids);
                assert_eq!(snapshot.page_count(), ids.len());
                for id in ids {
                    assert!(snapshot.page(id).is_some());
                }
            }
            assert_eq!(store.live_bytes(), live.len() * size);
            assert!(store.live_bytes() <= size * 2);
        }
    }
}
