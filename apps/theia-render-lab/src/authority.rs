//! Fixed server-owned two-tick fixture for GPU presentation qualification.
use crate::Result;
use theia_protocol::{
    presentation::Time,
    replica::{Replica, Snapshot},
};
use theia_sim::{AttackInput, AttackResult, MoveInput, Rect, Rules, Scene, World, WorldId};
pub struct Fixture {
    pub bytes: [u8; 64],
    pub reference: serde_json::Value,
}
pub fn fixture(alpha: f32) -> Result<Fixture> {
    if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
        return Err("Authority alpha must be finite in [0,1]".into());
    }
    let scene = Scene::new(
        Rect {
            min: [-4000; 2],
            max: [4000; 2],
        },
        vec![Rect {
            min: [-100; 2],
            max: [100; 2],
        }],
    )?;
    let mut world = World::new(scene, Rules::default())?;
    let ids = [
        WorldId::new((2u64 << 32) + 10)?,
        WorldId::new((2u64 << 32) + 20)?,
    ];
    let session = world.spawn_at_boundary(ids[0], [-1000, -1000])?;
    world.spawn_at_boundary(ids[1], [500, -1000])?;
    world.submit(
        session,
        MoveInput {
            sequence: 0,
            axes: [1000, 0],
        },
    )?;
    world.submit_attack(
        session,
        AttackInput {
            sequence: 1,
            target: ids[1],
        },
    )?;
    let first = world.step()?;
    if first.attacks[0].result != (AttackResult::Hit { health_after: 75 }) {
        return Err("Authority fixture hit did not qualify".into());
    }
    let previous = world.snapshot();
    world.submit(
        session,
        MoveInput {
            sequence: 2,
            axes: [1000, 0],
        },
    )?;
    world.step()?;
    let current = world.snapshot();
    let mut replica = Replica::new(2)?;
    let source = replica.connect()?;
    replica.publish(
        source,
        Snapshot {
            tick: 1,
            applied_move: Some(0),
            actors: previous.clone(),
        },
    )?;
    replica.publish(
        source,
        Snapshot {
            tick: 2,
            applied_move: Some(2),
            actors: current.clone(),
        },
    )?;
    let time = if alpha == 1.0 {
        Time::new(2, 0.0)?
    } else {
        Time::new(1, f64::from(alpha))?
    };
    let frame = replica
        .present(time, 4)?
        .ok_or("No authority presentation")?;
    let mut bytes = [0u8; 64];
    for (base, states) in [(0, &previous), (16, &current)] {
        for (i, state) in states.iter().enumerate() {
            for (axis, value) in state.position_mm.iter().enumerate() {
                let offset = base + i * 8 + axis * 4;
                bytes[offset..offset + 4].copy_from_slice(&(*value as f32).to_ne_bytes());
            }
        }
    }
    for (i, id) in ids.iter().enumerate() {
        let offset = 32 + i * 8;
        let value = id.value();
        bytes[offset..offset + 4].copy_from_slice(&(value as u32).to_ne_bytes());
        bytes[offset + 4..offset + 8].copy_from_slice(&((value >> 32) as u32).to_ne_bytes());
        bytes[48 + i * 4..52 + i * 4].copy_from_slice(&current[i].health.to_ne_bytes());
    }
    bytes[56..60].copy_from_slice(&alpha.to_ne_bytes());
    bytes[60..64].copy_from_slice(&3u32.to_ne_bytes());
    let actors: Vec<_> = frame.actors.iter().map(|v| {
        let folded = (v.id.value() as u32) ^ ((v.id.value() >> 32) as u32);
        let base = if folded & 2 != 0 { [0.1, 0.8, 0.2] } else { [0.8, 0.2, 0.1] };
        let color = base.map(|x| x * f64::from(v.health) / 100.0);
        serde_json::json!({"id":v.id.value(),"position_mm":v.position_mm,"health":v.health,"color":color,"half_extent_mm":250})
    }).collect();
    Ok(Fixture {
        bytes,
        reference: serde_json::json!({"alpha":alpha,"from_tick":1,"to_tick":2,"actors":actors,
        "obstacle":{"position_mm":[0,0],"half_extent_mm":100,"color":[0.3,0.3,0.3]},"world_clip_half_extent_mm":4000,
        "background":[0.025,0.07,0.12],"push_bytes":bytes.to_vec(),"source":"server-owned fixed movement/combat ticks; readonly presentation oracle"}),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_fixture_packing_preserves_large_ids_health_and_interpolation() {
        for (alpha, x) in [(0.0, -850.0), (0.5, -775.0), (1.0, -700.0)] {
            let f = fixture(alpha).unwrap();
            assert_eq!(f.reference["actors"][0]["position_mm"][0], x);
            for i in 0..2 {
                assert_eq!(
                    u32::from_ne_bytes(f.bytes[36 + i * 8..40 + i * 8].try_into().unwrap()),
                    2
                );
            }
            assert_eq!(u32::from_ne_bytes(f.bytes[52..56].try_into().unwrap()), 75);
            assert_eq!(u32::from_ne_bytes(f.bytes[60..64].try_into().unwrap()), 3);
        }
        for alpha in [f32::NAN, f32::INFINITY, -1.0, 1.01] {
            assert!(fixture(alpha).is_err());
        }
    }
}
