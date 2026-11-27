//! Headless basic combat qualification; no external clients or hit-volume system.
use std::error::Error;
use theia_sim::{
    AttackInput, AttackReject, AttackResult, MoveInput, Rect, Rules, Scene, World, WorldId,
};
fn id(value: u64) -> WorldId {
    WorldId::new(value).unwrap()
}
fn world(rules: Rules, obstacles: Vec<Rect>) -> Result<World, Box<dyn Error>> {
    Ok(World::new(
        Scene::new(
            Rect {
                min: [-10000; 2],
                max: [10000; 2],
            },
            obstacles,
        )?,
        rules,
    )?)
}
pub fn qualify() -> Result<(), Box<dyn Error>> {
    let rules = Rules {
        max_health: 100000,
        attack_damage: 7,
        attack_cooldown_ticks: 5,
        ..Rules::default()
    };
    let mut w = world(rules, vec![])?;
    let source = w.spawn_at_boundary(id(10), [0, 0])?;
    w.spawn_at_boundary(id(20), [1000, 0])?;
    let mut seed = 7919u64;
    let mut sequence = 0;
    let mut expected_health = rules.max_health;
    let mut ready = 0;
    let mut hits = 0;
    let mut cooldowns = 0;
    for tick in 1..=5000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let count = ((seed >> 32) % 5) as usize;
        for _ in 0..count {
            w.submit_attack(
                source,
                AttackInput {
                    sequence,
                    target: id(20),
                },
            )?;
            sequence += 1;
        }
        let frame = w.step()?;
        assert_eq!(frame.attacks.len(), count);
        let mut changed = false;
        for outcome in frame.attacks {
            let expected = if tick >= ready {
                expected_health -= rules.attack_damage;
                ready = tick + rules.attack_cooldown_ticks;
                hits += 1;
                changed = true;
                AttackResult::Hit {
                    health_after: expected_health,
                }
            } else {
                cooldowns += 1;
                AttackResult::Rejected(AttackReject::Cooldown)
            };
            assert_eq!(outcome.result, expected);
        }
        assert_eq!(frame.health_changes.len(), usize::from(changed));
        assert_eq!(w.actor(id(20)).unwrap().health, expected_health);
    }
    let wall = Rect {
        min: [-100, -100],
        max: [100, 100],
    };
    let mut blocked = world(Rules::default(), vec![wall])?;
    let s = blocked.spawn_at_boundary(id(1), [-700, 0])?;
    blocked.spawn_at_boundary(id(2), [700, 0])?;
    blocked.submit_attack(
        s,
        AttackInput {
            sequence: 0,
            target: id(2),
        },
    )?;
    assert_eq!(
        blocked.step()?.attacks[0].result,
        AttackResult::Rejected(AttackReject::LineOfSight)
    );
    assert_eq!(blocked.actor(id(2)).unwrap().health, 100);
    let mut lethal = world(
        Rules {
            attack_damage: 100,
            ..Rules::default()
        },
        vec![],
    )?;
    let a = lethal.spawn_at_boundary(id(1), [0, 0])?;
    let b = lethal.spawn_at_boundary(id(2), [1600, 0])?;
    lethal.submit_attack(
        a,
        AttackInput {
            sequence: 0,
            target: id(2),
        },
    )?;
    lethal.submit(
        b,
        MoveInput {
            sequence: 0,
            axes: [-1000, 0],
        },
    )?;
    lethal.submit_attack(
        b,
        AttackInput {
            sequence: 1,
            target: id(1),
        },
    )?;
    let frame = lethal.step()?;
    assert_eq!(
        frame.attacks[0].result,
        AttackResult::Hit { health_after: 0 }
    );
    assert_eq!(
        frame.attacks[1].result,
        AttackResult::Rejected(AttackReject::DeadAttacker)
    );
    assert_eq!(lethal.actor(id(1)).unwrap().health, 100);
    assert!(lethal.step()?.changes.is_empty());
    println!(
        "Verified 5000 authoritative combat ticks, {sequence} attack requests, {hits} hits, {cooldowns} cooldown rejections; final health {expected_health}; independent health/cooldown oracle, coalesced health stream, static LOS, post-movement range, death and attack ordering passed"
    );
    Ok(())
}
