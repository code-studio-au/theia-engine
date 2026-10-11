//! Headless local authority qualification. No sockets, authentication or persistence.
use std::error::Error;
use theia_sim::{Error as SimError, MoveInput, Rect, Rules, Scene, World, WorldId};
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut ticks = 10000u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--ticks" => ticks = args.next().ok_or("--ticks needs count")?.parse()?,
            "--help" => {
                println!(
                    "Theia local authority lab: --ticks <1..100000>; default 10000; no networking"
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown argument {arg}").into()),
        }
    }
    if !(1..=100000).contains(&ticks) {
        return Err("Ticks must be 1..100000".into());
    }
    let rules = Rules::default();
    let scene = Scene::new(
        Rect {
            min: [-4000, -4000],
            max: [4000, 4000],
        },
        vec![Rect {
            min: [-100, -100],
            max: [100, 100],
        }],
    )?;
    let mut world = World::new(scene, rules)?;
    let ids = [WorldId::new(10)?, WorldId::new(20)?, WorldId::new(30)?];
    let positions = [[-1000, -1000], [-1000, -1000], [1000, 1000]];
    let sessions: Vec<_> = ids
        .iter()
        .zip(positions)
        .map(|(&id, p)| world.spawn_at_boundary(id, p))
        .collect::<Result<_, _>>()?;
    assert_eq!(
        world.actor(ids[0]).unwrap().position_mm,
        world.actor(ids[1]).unwrap().position_mm
    );
    println!(
        "Theia authority lab: fixed {} ms simulation steps, {} mm/s speed, {} mm axis-aligned half extent; ordinary actors non-blocking; static floor/rectangles only",
        rules.tick_ms, rules.speed_mm_per_second, rules.half_extent_mm
    );
    let mut seed = 7919u64;
    let mut changes = 0;
    let mut blocked = 0;
    let mut replay_rejections = 0;
    let mut invalid_rejections = 0;
    let mut maximum_squared_step = 0i64;
    for sequence in 0..ticks {
        let before = world.snapshot();
        for &session in &sessions {
            let axes = std::array::from_fn(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((seed >> 32) % 2001) as i16 - 1000
            });
            world.submit(session, MoveInput { sequence, axes })?;
            if sequence % 32 == 0 {
                assert_eq!(
                    world.submit(session, MoveInput { sequence, axes }),
                    Err(SimError::Sequence)
                );
                replay_rejections += 1;
                assert_eq!(
                    world.submit(
                        session,
                        MoveInput {
                            sequence: sequence + 1,
                            axes: [1001, 0]
                        }
                    ),
                    Err(SimError::InvalidIntent)
                );
                invalid_rejections += 1;
            }
        }
        let frame = world.step()?;
        assert_eq!(frame.tick, sequence + 1);
        assert_eq!(frame.applied.len(), 3);
        changes += frame.changes.len();
        blocked += frame.blocked.len();
        for old in before {
            let p = world.actor(old.id).unwrap().position_mm;
            let squared = (0..2)
                .map(|i| {
                    let d = i64::from(p[i]) - i64::from(old.position_mm[i]);
                    d * d
                })
                .sum::<i64>();
            maximum_squared_step = maximum_squared_step.max(squared);
            assert!(squared <= 150 * 150);
            assert!(p.iter().all(|v| (-3750..=3750).contains(v)));
            assert!(p[0] < -350 || p[0] > 350 || p[1] < -350 || p[1] > 350);
        }
    }
    println!(
        "Verified {ticks} authoritative ticks, {} admitted inputs, {changes} movement changes, {blocked} static/world blocks, {replay_rejections} replay and {invalid_rejections} malformed-intent rejections; max squared movement {maximum_squared_step} <= 22500 mm^2",
        ticks * 3
    );
    println!(
        "No networking, collision mesh/navigation, combat, persistence or GPU-authority claim."
    );
    Ok(())
}
