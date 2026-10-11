//! Adversarial typed-message qualification; deliberately no socket/transport claim.
use std::error::Error;
use theia_protocol::{Authority, Error as ProtocolError, Limits, Reply};
use theia_sim::{AttackInput, AttackReject, AttackResult, MoveInput, Rect, Rules, Scene, WorldId};
pub fn qualify() -> Result<(), Box<dyn Error>> {
    let scene = Scene::new(
        Rect {
            min: [-4000; 2],
            max: [4000; 2],
        },
        vec![],
    )?;
    let rules = Rules {
        max_health: 1_000_000,
        attack_damage: 7,
        attack_cooldown_ticks: 1,
        max_inputs_per_session: 16,
        ..Rules::default()
    };
    let mut authority = Authority::new(
        scene,
        rules,
        Limits {
            max_peers: 2,
            request_window: 4,
        },
    )?;
    let attacker = WorldId::new(10)?;
    let target = WorldId::new(20)?;
    let peer = authority.attach_actor(attacker, [0, 0])?;
    authority.attach_actor(target, [1000, 0])?;
    let mut seed = 7919u64;
    let mut expected_health = rules.max_health;
    let mut duplicates = 0;
    let mut conflicts = 0;
    let mut pressure = 0;
    for tick in 1..=10000u64 {
        let base = (tick - 1) * 4;
        let mut order = [0, 1, 2, 3];
        for i in (1..4).rev() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            order.swap(i, ((seed >> 32) % (i as u64 + 1)) as usize);
        }
        let movement = MoveInput {
            sequence: 1_000_000 + tick * 17,
            axes: [0, 0],
        };
        authority.movement(peer, movement)?;
        for &offset in &order {
            let input = AttackInput {
                sequence: base + offset,
                target,
            };
            assert_eq!(authority.attack(peer, input)?, Reply::Queued);
            assert_eq!(authority.attack(peer, input)?, Reply::Pending);
            duplicates += 1;
            assert_eq!(
                authority.acknowledge(peer, input.sequence),
                Err(ProtocolError::OutcomePending)
            );
        }
        let outside = AttackInput {
            sequence: base + 4,
            target,
        };
        assert_eq!(
            authority.attack(peer, outside),
            Err(ProtocolError::RequestWindow)
        );
        pressure += 1;
        let frame = authority.step()?;
        assert_eq!(frame.simulation.tick, tick);
        assert_eq!(frame.move_acks, vec![(peer, movement.sequence)]);
        assert_eq!(frame.outcomes.len(), 4);
        expected_health -= rules.attack_damage;
        assert_eq!(authority.actor(target).unwrap().health, expected_health);
        for (index, &(owner, outcome)) in frame.outcomes.iter().enumerate() {
            assert_eq!(owner, peer);
            assert_eq!(outcome.request, base + order[index]);
            assert_eq!(outcome.tick, tick);
            let expected = if index == 0 {
                AttackResult::Hit {
                    health_after: expected_health,
                }
            } else {
                AttackResult::Rejected(AttackReject::Cooldown)
            };
            assert_eq!(outcome.result, expected);
            assert_eq!(
                authority.attack(
                    peer,
                    AttackInput {
                        sequence: outcome.request,
                        target
                    }
                )?,
                Reply::Completed(outcome)
            );
            duplicates += 1;
            assert_eq!(
                authority.attack(
                    peer,
                    AttackInput {
                        sequence: outcome.request,
                        target: attacker
                    }
                ),
                Err(ProtocolError::RequestConflict)
            );
            conflicts += 1;
        }
        // ACK high IDs first: a gap cannot discard their replay tombstones or open the window.
        for offset in [3, 2, 1] {
            authority.acknowledge(peer, base + offset)?;
            assert_eq!(
                authority.attack(
                    peer,
                    AttackInput {
                        sequence: base + offset,
                        target
                    }
                )?,
                Reply::Acknowledged
            );
            duplicates += 1;
        }
        assert_eq!(authority.retained_outcomes(peer)?, 4);
        assert_eq!(
            authority.attack(peer, outside),
            Err(ProtocolError::RequestWindow)
        );
        pressure += 1;
        authority.acknowledge(peer, base)?;
        authority.acknowledge(peer, base)?;
        assert_eq!(authority.retained_outcomes(peer)?, 0);
        assert_eq!(
            authority.attack(
                peer,
                AttackInput {
                    sequence: base,
                    target
                }
            ),
            Err(ProtocolError::RequestStale)
        );
        assert_eq!(
            authority.movement(peer, movement),
            Err(ProtocolError::StaleMove)
        );
    }
    println!(
        "Verified 10000 typed-message ticks, 40000 distinct requests, {duplicates} pending/completed/tombstone duplicate replies, {conflicts} conflicting payload rejections and {pressure} window rejections; final health {expected_health}; retained outcomes 0."
    );
    println!(
        "Independent one-hit-per-tick health/cooldown model matches; no networking, encryption, authentication, durable effects, reconnect continuity or performance claim."
    );
    Ok(())
}
