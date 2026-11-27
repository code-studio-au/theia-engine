//! Bounded host-backed arena range oracle. Leases release only after their last owner.
use crate::{AssetError, Result};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
#[derive(Clone)]
pub struct Arena {
    state: Arc<Mutex<State>>,
}
struct State {
    capacity: u64,
    alignment: u64,
    next: u64,
    free: BTreeMap<u64, u64>,
    used: BTreeMap<u64, (u64, u64)>,
}
#[derive(Clone)]
pub struct Lease(Arc<Range>);
struct Range {
    state: Arc<Mutex<State>>,
    ticket: u64,
    offset: u64,
    bytes: u64,
    reserved: u64,
}
impl Arena {
    pub fn new(capacity: u64, alignment: u64) -> Result<Self> {
        if capacity == 0 || !alignment.is_power_of_two() {
            return Err(AssetError("invalid arena capacity/alignment"));
        }
        Ok(Self {
            state: Arc::new(Mutex::new(State {
                capacity,
                alignment,
                next: 1,
                free: BTreeMap::from([(0, capacity)]),
                used: BTreeMap::new(),
            })),
        })
    }
    pub fn reserve(&self, bytes: u64) -> Result<Lease> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AssetError("arena state poisoned"))?;
        if bytes == 0 {
            return Err(AssetError("empty arena reservation"));
        }
        let reserved = bytes
            .checked_add(state.alignment - 1)
            .map(|v| v & !(state.alignment - 1))
            .ok_or(AssetError("arena reservation size overflow"))?;
        let (&offset, &available) = state
            .free
            .iter()
            .find(|(_, size)| **size >= reserved)
            .ok_or(AssetError("arena has no contiguous aligned range"))?;
        let ticket = state.next;
        state.next = ticket
            .checked_add(1)
            .ok_or(AssetError("arena ticket exhausted"))?;
        state.free.remove(&offset);
        if available > reserved {
            state.free.insert(offset + reserved, available - reserved);
        }
        state.used.insert(ticket, (offset, reserved));
        Ok(Lease(Arc::new(Range {
            state: self.state.clone(),
            ticket,
            offset,
            bytes,
            reserved,
        })))
    }
    pub fn live_bytes(&self) -> Result<u64> {
        Ok(self
            .state
            .lock()
            .map_err(|_| AssetError("arena state poisoned"))?
            .used
            .values()
            .map(|(_, size)| size)
            .sum())
    }
    pub fn capacity(&self) -> Result<u64> {
        Ok(self
            .state
            .lock()
            .map_err(|_| AssetError("arena state poisoned"))?
            .capacity)
    }
}
impl Lease {
    pub fn offset(&self) -> u64 {
        self.0.offset
    }
    pub fn bytes(&self) -> u64 {
        self.0.bytes
    }
    pub fn reserved(&self) -> u64 {
        self.0.reserved
    }
}
impl Drop for Range {
    fn drop(&mut self) {
        // No manual/free-token API: a stale clone cannot release someone else's range.
        // On poisoning, fail closed by leaking the reservation instead of reusing it.
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let Some((mut offset, mut size)) = state.used.remove(&self.ticket) else {
            return;
        };
        if let Some((&previous, &length)) = state.free.range(..offset).next_back()
            && previous + length == offset
        {
            state.free.remove(&previous);
            offset = previous;
            size += length;
        }
        if let Some((&next, &length)) = state.free.range(offset..).next()
            && offset + size == next
        {
            state.free.remove(&next);
            size += length;
        }
        state.free.insert(offset, size);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_leases_protect_aligned_ranges_and_coalesce() {
        let arena = Arena::new(128, 16).unwrap();
        let first = arena.reserve(17).unwrap();
        assert_eq!(
            (first.offset(), first.bytes(), first.reserved()),
            (0, 17, 32)
        );
        let retained = first.clone();
        let second = arena.reserve(96).unwrap();
        drop(first);
        assert!(arena.reserve(1).is_err());
        drop(retained);
        assert_eq!(arena.live_bytes().unwrap(), 96);
        drop(second);
        assert_eq!(arena.reserve(128).unwrap().offset(), 0);
    }
    #[test]
    fn fragmentation_overflow_and_exhaustion_fail_without_mutation() {
        let arena = Arena::new(64, 16).unwrap();
        let mut leases: Vec<_> = (0..4).map(|_| Some(arena.reserve(16).unwrap())).collect();
        leases[0] = None;
        leases[2] = None;
        assert!(arena.reserve(32).is_err());
        assert_eq!(arena.live_bytes().unwrap(), 32);
        assert!(arena.reserve(0).is_err());
        assert!(arena.reserve(u64::MAX).is_err());
        leases[1] = None;
        assert_eq!(arena.reserve(48).unwrap().offset(), 0);
        assert!(Arena::new(0, 16).is_err());
        assert!(Arena::new(64, 3).is_err());
        arena.state.lock().unwrap().next = u64::MAX;
        assert!(arena.reserve(16).is_err());
        assert_eq!(arena.live_bytes().unwrap(), 16);
    }
    #[test]
    fn randomized_ranges_match_independent_occupancy_bitmap() {
        let arena = Arena::new(1024, 16).unwrap();
        let mut leases: Vec<Lease> = Vec::new();
        let mut seed = 1729u64;
        for _ in 0..5000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            if seed.is_multiple_of(3) && !leases.is_empty() {
                leases.swap_remove(seed as usize % leases.len());
            } else if let Ok(lease) = arena.reserve(((seed >> 32) % 100) + 1) {
                leases.push(lease);
            }
            let mut occupied = [false; 64];
            for lease in &leases {
                assert_eq!(lease.offset() % 16, 0);
                assert!(lease.bytes() <= lease.reserved());
                for index in lease.offset() / 16..(lease.offset() + lease.reserved()) / 16 {
                    assert!(!occupied[index as usize]);
                    occupied[index as usize] = true;
                }
            }
            assert_eq!(
                arena.live_bytes().unwrap(),
                occupied.iter().filter(|v| **v).count() as u64 * 16
            );
        }
        drop(leases);
        assert_eq!(arena.live_bytes().unwrap(), 0);
        assert_eq!(
            arena.reserve(arena.capacity().unwrap()).unwrap().offset(),
            0
        );
    }
}
