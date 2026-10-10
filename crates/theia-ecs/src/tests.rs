use super::*;

fn entity(world_id: u64, moving: bool) -> Entity {
    Entity {
        world_id,
        position: [world_id as f32, 2.0],
        velocity: moving.then_some([0.5, -0.25]),
    }
}
fn spawn(oracle: &mut Oracle, value: Entity) -> Handle {
    oracle.publish(&[Command::Spawn(value)])[0].unwrap()
}
#[test]
fn compaction_and_archetype_migration_preserve_identity() {
    let mut oracle = Oracle::new(8, 2, 2);
    let a = spawn(&mut oracle, entity(10, false));
    let b = spawn(&mut oracle, entity(20, false));
    let c = spawn(&mut oracle, entity(30, false));
    assert_eq!(oracle.location(c).unwrap().chunk, 1);
    oracle.publish(&[Command::Despawn(a)]);
    assert_eq!(oracle.location(b).unwrap().row, 0);
    let before = oracle.entity(b).unwrap();
    assert_eq!(
        oracle.publish(&[Command::SetVelocity(b, Some([1.0, 2.0]))]),
        vec![Ok(b)]
    );
    assert_eq!(oracle.entity(b).unwrap().world_id, before.world_id);
    assert_eq!(oracle.entity(b).unwrap().position, before.position);
    assert_eq!(oracle.location(b).unwrap().archetype, Archetype::Moving);
    oracle.integrate(0.5).unwrap();
    assert_eq!(oracle.entity(b).unwrap().position, [20.5, 3.0]);
    assert_eq!(oracle.entity(c).unwrap().position, [30.0, 2.0]);
    oracle.check_invariants().unwrap();
}
#[test]
fn readers_delay_generation_reuse_and_keep_old_data() {
    let mut oracle = Oracle::new(1, 1, 1);
    let old = spawn(&mut oracle, entity(10, false));
    let first = oracle.retain();
    let second = oracle.retain();
    oracle.publish(&[Command::Despawn(old)]);
    assert_eq!(
        oracle.publish(&[Command::Spawn(entity(20, true))]),
        vec![Err(MutationError::SlotsExhausted)]
    );
    assert_eq!(first.entities[&old], entity(10, false));
    oracle.release(first);
    assert_eq!(
        oracle.publish(&[Command::Spawn(entity(20, true))]),
        vec![Err(MutationError::SlotsExhausted)]
    );
    oracle.release(second);
    let new = spawn(&mut oracle, entity(20, true));
    assert_eq!(new.slot, old.slot);
    assert_eq!(new.generation, old.generation + 1);
    assert!(oracle.entity(old).is_none());
    assert_eq!(
        oracle.publish(&[Command::Despawn(old)]),
        vec![Err(MutationError::StaleHandle)]
    );
    oracle.check_invariants().unwrap();
}
#[test]
fn destination_overflow_and_conflicting_commands_have_explicit_results() {
    let mut oracle = Oracle::new(4, 1, 1);
    let stationary = spawn(&mut oracle, entity(10, false));
    let moving = spawn(&mut oracle, entity(20, true));
    let before = oracle.entities();
    assert_eq!(
        oracle.publish(&[Command::SetVelocity(stationary, Some([1.0, 1.0]))]),
        vec![Err(MutationError::ChunksExhausted)]
    );
    assert_eq!(oracle.entities(), before);
    assert_eq!(
        oracle.publish(&[Command::Spawn(entity(20, false))]),
        vec![Err(MutationError::DuplicateWorldId)]
    );
    assert_eq!(
        oracle.publish(&[
            Command::Despawn(moving),
            Command::Despawn(moving),
            Command::SetVelocity(moving, None)
        ]),
        vec![
            Ok(moving),
            Err(MutationError::StaleHandle),
            Err(MutationError::StaleHandle)
        ]
    );
    oracle.check_invariants().unwrap();
}
#[test]
fn generation_wrap_retires_slot_permanently() {
    let mut oracle = Oracle::new(1, 1, 1);
    oracle.slots[0].generation = u32::MAX;
    let last = spawn(&mut oracle, entity(10, true));
    oracle.publish(&[Command::Despawn(last)]);
    assert!(oracle.slots[0].exhausted);
    assert_eq!(
        oracle.publish(&[Command::Spawn(entity(20, false))]),
        vec![Err(MutationError::SlotsExhausted)]
    );
}
#[test]
fn invalid_components_cannot_mutate_live_data() {
    let mut oracle = Oracle::new(2, 1, 2);
    let handle = spawn(&mut oracle, entity(10, true));
    let before = oracle.entities();
    assert_eq!(
        oracle.publish(&[Command::SetVelocity(handle, Some([f32::NAN, 0.0]))]),
        vec![Err(MutationError::NonFiniteComponent)]
    );
    assert!(oracle.integrate(f32::INFINITY).is_err());
    assert!(oracle.integrate(-1.0).is_err());
    assert_eq!(oracle.entities(), before);
}
#[test]
fn deterministic_randomized_batches_match_an_independent_map() {
    let mut oracle = Oracle::new(128, 4, 32);
    let mut map = BTreeMap::<Handle, Entity>::new();
    let mut history = Vec::new();
    let mut seed = 0x1324_5678_u64;
    let mut next_world = 1;
    for iteration in 0..10_000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let choice = (seed >> 32) % 4;
        if choice == 0 && map.len() < 100 {
            let value = entity(next_world, seed & 1 != 0);
            next_world += 1;
            let handle = spawn(&mut oracle, value);
            map.insert(handle, value);
            history.push(handle);
        } else if choice < 3 && !history.is_empty() {
            let handle = history[(seed as usize) % history.len()];
            let command = if choice == 1 {
                Command::Despawn(handle)
            } else {
                Command::SetVelocity(handle, (seed & 1 != 0).then_some([0.5, -0.25]))
            };
            let existed = map.contains_key(&handle);
            let result = oracle.publish(&[command])[0];
            if existed {
                assert_eq!(result, Ok(handle));
                match command {
                    Command::Despawn(_) => {
                        map.remove(&handle);
                    }
                    Command::SetVelocity(_, velocity) => {
                        map.get_mut(&handle).unwrap().velocity = velocity
                    }
                    _ => unreachable!(),
                }
            } else {
                assert_eq!(result, Err(MutationError::StaleHandle));
            }
        } else {
            oracle.integrate(0.25).unwrap();
            for value in map.values_mut() {
                if let Some(velocity) = value.velocity {
                    for (position, speed) in value.position.iter_mut().zip(velocity) {
                        *position += speed * 0.25;
                    }
                }
            }
        }
        oracle.check_invariants().unwrap();
        assert_eq!(oracle.entities(), map, "iteration {iteration}");
    }
}
