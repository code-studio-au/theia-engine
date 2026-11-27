//! Independent CPU reference for the bounded M2 resident scene, not a host draw list.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}
impl Bounds {
    fn intersects(self, other: Self) -> bool {
        (0..3).all(|axis| self.min[axis] <= other.max[axis] && self.max[axis] >= other.min[axis])
    }
}
pub fn scene() -> Vec<Bounds> {
    (0..32)
        .map(|index| {
            let center = [
                -0.875 + (index % 8) as f32 * 0.25,
                -0.75 + (index / 8) as f32 * 0.5,
                0.125 + (index % 4) as f32 * 0.25,
            ];
            let half = [0.06, 0.06, 0.04];
            Bounds {
                min: std::array::from_fn(|axis| center[axis] - half[axis]),
                max: std::array::from_fn(|axis| center[axis] + half[axis]),
            }
        })
        .collect()
}
pub fn view(scenario: u32) -> Bounds {
    match scenario {
        0 => Bounds {
            min: [2.0; 3],
            max: [3.0; 3],
        },
        1 => Bounds {
            min: [-1.0, -1.0, -0.05],
            max: [0.0, 1.0, 0.5],
        },
        2 => Bounds {
            min: [-1.0; 3],
            max: [1.0; 3],
        },
        _ => panic!("bounded visibility scenario must be 0, 1 or 2"),
    }
}
fn cell(index: usize) -> Bounds {
    let x = -1.0 + (index % 8) as f32 * 0.25;
    let y = -1.0 + (index / 8) as f32 * 0.25;
    Bounds {
        min: [x, y, f32::NEG_INFINITY],
        max: [x + 0.25, y + 0.25, f32::INFINITY],
    }
}
pub struct Reference {
    pub offsets: Vec<u32>,
    pub members: Vec<u32>,
    pub mask: u32,
    pub visible: Vec<u32>,
}
pub fn reference(objects: &[Bounds], view: Bounds) -> Reference {
    assert!(objects.len() <= 32);
    // This bounded resident slice requires whole XY bounds inside its fixed world.
    assert!(
        objects
            .iter()
            .all(|object| (0..2).all(|axis| object.min[axis] >= -1.0 && object.max[axis] <= 1.0))
    );
    let mut offsets = vec![0];
    let mut members = Vec::new();
    let mut mask = 0u32;
    for index in 0..64 {
        let bounds = cell(index);
        for (entity, object) in objects.iter().enumerate() {
            if bounds.intersects(*object) {
                members.push(entity as u32);
                if bounds.intersects(view) {
                    mask |= 1 << entity;
                }
            }
        }
        offsets.push(members.len() as u32);
    }
    let visible = objects
        .iter()
        .enumerate()
        .filter_map(|(entity, object)| {
            (mask & (1 << entity) != 0 && object.intersects(view)).then_some(entity as u32)
        })
        .collect();
    Reference {
        offsets,
        members,
        mask,
        visible,
    }
}
pub fn print_reference(scenario: u32) {
    let objects = scene();
    let result = reference(&objects, view(scenario));
    let bounds: Vec<_> = objects
        .iter()
        .flat_map(|object| object.min.into_iter().chain(object.max).map(f32::to_bits))
        .collect();
    println!(
        "{{\"offsets\":{:?},\"members\":{:?},\"mask\":{},\"visible\":{:?},\"bounds_words\":{:?}}}",
        result.offsets, result.members, result.mask, result.visible, bounds
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grid_candidates_match_independent_brute_force_views() {
        let objects = scene();
        let mut seed = 0xabcdef_u64;
        for _ in 0..1000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let min = [
                (seed & 255) as f32 / 64.0 - 2.0,
                ((seed >> 8) & 255) as f32 / 64.0 - 2.0,
                ((seed >> 16) & 255) as f32 / 128.0 - 0.5,
            ];
            let view = Bounds {
                min,
                max: [min[0] + 0.6, min[1] + 0.6, min[2] + 0.4],
            };
            let result = reference(&objects, view);
            let brute: Vec<_> = objects
                .iter()
                .enumerate()
                .filter_map(|(index, object)| object.intersects(view).then_some(index as u32))
                .collect();
            assert_eq!(result.visible, brute);
            assert_eq!(result.offsets.len(), 65);
            assert_eq!(result.offsets[64] as usize, result.members.len());
        }
    }
    #[test]
    fn bounded_scenarios_include_empty_height_filter_and_full_visibility() {
        let objects = scene();
        assert!(reference(&objects, view(0)).visible.is_empty());
        assert_eq!(
            reference(&objects, view(1)).visible,
            vec![0, 1, 8, 9, 16, 17, 24, 25]
        );
        assert_eq!(
            reference(&objects, view(2)).visible,
            (0..32).collect::<Vec<_>>()
        );
        assert_eq!(reference(&objects, view(2)).members.len(), 64);
    }
}
