//! Bounded CPU chunk-distance demand policy. No I/O, GPU culling or publication.
//! Uses horizontal bounds for the outdoor 2.5D prototype; altitude is ignored.
use crate::{
    AssetError, Result,
    bundle_residency::{ChunkId, MAX_WORLD_CHUNKS},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug)]
pub struct ChunkBounds {
    pub id: ChunkId,
    /// Inclusive XY bounds enclosing all geometry belonging to this chunk.
    pub min: [f32; 2],
    pub max: [f32; 2],
}

/// Radius to load a missing chunk; larger radius to keep an already resident one.
/// These are experimental application parameters, not an engine streaming ADR.
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    pub load_radius: f32,
    pub retain_radius: f32,
    pub slots: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    /// Nearest eligible chunks first, ties by ascending ID; limited by slots.
    pub desired: Vec<ChunkId>,
    /// Desired chunks absent from the supplied actual resident set, in priority order.
    pub loads: Vec<ChunkId>,
    /// Actual residents outside the chosen set, ascending ID.
    pub removals: Vec<ChunkId>,
    /// Eligible chunks omitted under slot pressure, in priority order.
    pub deferred: Vec<ChunkId>,
}

pub struct Catalog {
    chunks: BTreeMap<ChunkId, ChunkBounds>,
}
impl Catalog {
    pub fn new(chunks: &[ChunkBounds]) -> Result<Self> {
        if chunks.is_empty() || chunks.len() > MAX_WORLD_CHUNKS {
            return Err(AssetError("invalid demand catalog size"));
        }
        let mut indexed = BTreeMap::new();
        for chunk in chunks {
            if chunk.min.iter().chain(&chunk.max).any(|v| !v.is_finite())
                || (0..2).any(|axis| chunk.min[axis] > chunk.max[axis])
                || indexed.insert(chunk.id, *chunk).is_some()
            {
                return Err(AssetError("invalid or duplicate demand chunk bounds"));
            }
        }
        Ok(Self { chunks: indexed })
    }
    /// Stateless planning from *actual* residency, never the previous desired set.
    /// A failed I/O/upload must not make an absent chunk eligible for retention.
    /// Slot pressure may evict retained chunks; nearest distance wins, not pinning.
    /// Bytes, pending requests, fallback coverage and upload coexistence budgets
    /// must be handled separately before executing any plan. No state is mutated.
    pub fn plan(&self, eye_xy: [f32; 2], resident: &[ChunkId], policy: Policy) -> Result<Plan> {
        if eye_xy.iter().any(|v| !v.is_finite())
            || !policy.load_radius.is_finite()
            || !policy.retain_radius.is_finite()
            || policy.load_radius < 0.
            || policy.retain_radius < policy.load_radius
            || !(1..=MAX_WORLD_CHUNKS).contains(&policy.slots)
            || resident.len() > self.chunks.len()
        {
            return Err(AssetError("invalid world demand query/policy"));
        }
        let mut actual = BTreeSet::new();
        for id in resident {
            if !self.chunks.contains_key(id) || !actual.insert(*id) {
                return Err(AssetError("unknown or duplicate resident demand chunk"));
            }
        }
        let mut eligible = Vec::new();
        for chunk in self.chunks.values() {
            // Widen before subtracting/squaring: finite f32 coordinates, including
            // opposite extreme values, cannot overflow this f64 distance model.
            let distance_squared: f64 = (0..2)
                .map(|axis| {
                    let point = f64::from(eye_xy[axis]);
                    let delta = (f64::from(chunk.min[axis]) - point)
                        .max(point - f64::from(chunk.max[axis]))
                        .max(0.);
                    delta * delta
                })
                .sum();
            let radius = f64::from(if actual.contains(&chunk.id) {
                policy.retain_radius
            } else {
                policy.load_radius
            });
            if distance_squared <= radius * radius {
                eligible.push((distance_squared, chunk.id));
            }
        }
        eligible.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let desired: Vec<_> = eligible
            .iter()
            .take(policy.slots)
            .map(|(_, id)| *id)
            .collect();
        let selected: BTreeSet<_> = desired.iter().copied().collect();
        Ok(Plan {
            loads: desired
                .iter()
                .filter(|id| !actual.contains(id))
                .copied()
                .collect(),
            removals: actual.difference(&selected).copied().collect(),
            deferred: eligible
                .iter()
                .skip(policy.slots)
                .map(|(_, id)| *id)
                .collect(),
            desired,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn chunk(id: u64, x: f32, y: f32) -> ChunkBounds {
        ChunkBounds {
            id,
            min: [x, y],
            max: [x + 2., y + 2.],
        }
    }
    fn policy() -> Policy {
        Policy {
            load_radius: 3.,
            retain_radius: 5.,
            slots: 4,
        }
    }

    #[test]
    fn inclusive_rectangular_distance_and_retention_use_actual_residency() {
        let c = Catalog::new(&[chunk(7, 0., 0.)]).unwrap();
        assert_eq!(c.plan([5., 1.], &[], policy()).unwrap().loads, [7]);
        assert!(
            c.plan([5.01, 1.], &[], policy())
                .unwrap()
                .desired
                .is_empty()
        );
        assert_eq!(c.plan([7., 1.], &[7], policy()).unwrap().desired, [7]);
        assert_eq!(c.plan([7.01, 1.], &[7], policy()).unwrap().removals, [7]);
        assert!(c.plan([7., 1.], &[], policy()).unwrap().desired.is_empty());
        // Diagonal corner distance is Euclidean, not an expanded square.
        assert!(c.plan([5., 5.], &[], policy()).unwrap().desired.is_empty());
        assert_eq!(c.plan([5., 6.], &[7], policy()).unwrap().desired, [7]);
        assert_eq!(
            c.plan(
                [1., 1.],
                &[],
                Policy {
                    load_radius: 0.,
                    retain_radius: 0.,
                    slots: 1
                }
            )
            .unwrap()
            .loads,
            [7]
        );
    }

    #[test]
    fn pressure_orders_by_distance_then_id_and_does_not_pin_retained_chunks() {
        let c = Catalog::new(&[chunk(20, 3., 0.), chunk(4, -5., 0.), chunk(9, -1., 0.)]).unwrap();
        let p = Policy {
            load_radius: 10.,
            retain_radius: 20.,
            slots: 2,
        };
        let plan = c.plan([0., 0.], &[20], p).unwrap();
        assert_eq!(plan.desired, [9, 4]);
        assert_eq!(plan.loads, [9, 4]);
        assert_eq!(plan.removals, [20]);
        assert_eq!(plan.deferred, [20]);
        assert_eq!(c.plan([0., 0.], &[9, 4], p).unwrap().loads, []);
    }

    #[test]
    fn rejects_bad_catalog_queries_and_handles_extreme_finite_coordinates() {
        assert!(Catalog::new(&[]).is_err());
        assert!(Catalog::new(&vec![chunk(1, 0., 0.); MAX_WORLD_CHUNKS + 1]).is_err());
        assert!(Catalog::new(&[chunk(1, 0., 0.), chunk(1, 1., 1.)]).is_err());
        assert!(
            Catalog::new(&[ChunkBounds {
                id: 0,
                min: [1., 0.],
                max: [0., 1.]
            }])
            .is_err()
        );
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(Catalog::new(&[chunk(1, value, 0.)]).is_err());
            let c = Catalog::new(&[chunk(1, 0., 0.)]).unwrap();
            assert!(c.plan([value, 0.], &[], policy()).is_err());
            assert!(
                c.plan(
                    [0.; 2],
                    &[],
                    Policy {
                        load_radius: value,
                        ..policy()
                    }
                )
                .is_err()
            );
            assert!(
                c.plan(
                    [0.; 2],
                    &[],
                    Policy {
                        retain_radius: value,
                        ..policy()
                    }
                )
                .is_err()
            );
        }
        let c = Catalog::new(&[chunk(1, 0., 0.)]).unwrap();
        for p in [
            Policy {
                load_radius: -1.,
                ..policy()
            },
            Policy {
                retain_radius: 2.,
                ..policy()
            },
            Policy {
                slots: 0,
                ..policy()
            },
            Policy {
                slots: MAX_WORLD_CHUNKS + 1,
                ..policy()
            },
        ] {
            assert!(c.plan([0.; 2], &[], p).is_err());
        }
        assert!(c.plan([0.; 2], &[2], policy()).is_err());
        assert!(c.plan([0.; 2], &[1, 1], policy()).is_err());
        let c = Catalog::new(&[ChunkBounds {
            id: 1,
            min: [f32::MAX; 2],
            max: [f32::MAX; 2],
        }])
        .unwrap();
        assert!(
            c.plan(
                [-f32::MAX; 2],
                &[],
                Policy {
                    load_radius: f32::MAX,
                    retain_radius: f32::MAX,
                    slots: 1
                }
            )
            .unwrap()
            .desired
            .is_empty()
        );
    }

    #[test]
    fn randomized_queries_match_enumerated_integer_lattice_distance_oracle() {
        let chunks: Vec<_> = (0..16)
            .map(|i| chunk(i, ((i % 4) as f32 - 2.) * 5., ((i / 4) as f32 - 2.) * 5.))
            .collect();
        let c = Catalog::new(&chunks).unwrap();
        let mut seed = 937u64;
        let mut resident = Vec::new();
        for _ in 0..2000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let eye = [(seed % 41) as i32 - 20, ((seed >> 16) % 41) as i32 - 20];
            let p = Policy {
                load_radius: 5.,
                retain_radius: 8.,
                slots: (seed as usize % 8) + 1,
            };
            let mut expected = Vec::new();
            for chunk in &chunks {
                // Integer eye and rectangle endpoints make enumeration of every
                // integer point an exact independent closest-point oracle.
                let mut distance = i32::MAX;
                for x in chunk.min[0] as i32..=chunk.max[0] as i32 {
                    for y in chunk.min[1] as i32..=chunk.max[1] as i32 {
                        distance = distance.min((eye[0] - x).pow(2) + (eye[1] - y).pow(2));
                    }
                }
                let radius: i32 = if resident.contains(&chunk.id) { 8 } else { 5 };
                if distance <= radius.pow(2) {
                    expected.push((distance, chunk.id));
                }
            }
            expected.sort();
            let desired: Vec<_> = expected.iter().take(p.slots).map(|(_, id)| *id).collect();
            let plan = c.plan(eye.map(|v| v as f32), &resident, p).unwrap();
            assert_eq!(plan.desired, desired);
            assert_eq!(
                plan.loads,
                desired
                    .iter()
                    .filter(|id| !resident.contains(id))
                    .copied()
                    .collect::<Vec<_>>()
            );
            let mut removed: Vec<_> = resident
                .iter()
                .filter(|id| !desired.contains(id))
                .copied()
                .collect();
            removed.sort();
            assert_eq!(plan.removals, removed);
            assert_eq!(
                plan.deferred,
                expected
                    .iter()
                    .skip(p.slots)
                    .map(|(_, id)| *id)
                    .collect::<Vec<_>>()
            );
            // Simulate occasional failed requests: do not publish the desired set.
            if !seed.is_multiple_of(3) {
                resident = desired;
            }
        }
    }
}
