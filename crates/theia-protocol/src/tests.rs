use super::*;
use theia_sim::Rect;
fn id(n: u64) -> WorldId {
    WorldId::new(n).unwrap()
}
fn make(rules: Rules, window: usize) -> Authority {
    Authority::new(
        Scene::new(
            Rect {
                min: [-4000; 2],
                max: [4000; 2],
            },
            vec![],
        )
        .unwrap(),
        rules,
        Limits {
            max_peers: rules.max_actors,
            request_window: window,
        },
    )
    .unwrap()
}
fn pair(a: &mut Authority) -> (Peer, Peer) {
    (
        a.attach_actor(id(1), [0, 0]).unwrap(),
        a.attach_actor(id(2), [1000, 0]).unwrap(),
    )
}
fn attack(sequence: u64) -> AttackInput {
    AttackInput {
        sequence,
        target: id(2),
    }
}
#[test]
fn independent_domains_and_cached_outcome_survive_later_damage() {
    let mut a = make(
        Rules {
            attack_cooldown_ticks: 1,
            ..Rules::default()
        },
        4,
    );
    let (p, _) = pair(&mut a);
    a.movement(
        p,
        MoveInput {
            sequence: 1_000_000,
            axes: [0, 0],
        },
    )
    .unwrap();
    assert_eq!(a.attack(p, attack(0)), Ok(Reply::Queued));
    assert_eq!(a.attack(p, attack(0)), Ok(Reply::Pending));
    assert_eq!(a.acknowledge(p, 0), Err(Error::OutcomePending));
    let f = a.step().unwrap();
    assert_eq!(f.move_acks, vec![(p, 1_000_000)]);
    let first = f.outcomes[0].1;
    assert_eq!(first.result, AttackResult::Hit { health_after: 75 });
    assert_eq!(a.attack(p, attack(0)), Ok(Reply::Completed(first)));
    assert_eq!(
        a.attack(
            p,
            AttackInput {
                sequence: 0,
                target: id(1)
            }
        ),
        Err(Error::RequestConflict)
    );
    a.attack(p, attack(1)).unwrap();
    a.step().unwrap();
    assert_eq!(a.actor(id(2)).unwrap().health, 50);
    assert_eq!(a.attack(p, attack(0)), Ok(Reply::Completed(first)));
    assert!(a.step().unwrap().outcomes.is_empty());
    assert_eq!(a.applied_move(p), Ok(Some(1_000_000)));
}
#[test]
fn out_of_order_ack_holes_retain_tombstones_and_backpressure() {
    let mut a = make(Rules::default(), 2);
    let (p, _) = pair(&mut a);
    a.attack(p, attack(1)).unwrap();
    a.step().unwrap();
    a.acknowledge(p, 1).unwrap();
    assert_eq!(a.attack(p, attack(1)), Ok(Reply::Acknowledged));
    assert_eq!(a.attack(p, attack(2)), Err(Error::RequestWindow));
    assert_eq!(a.acknowledge(p, 0), Err(Error::OutcomeUnknown));
    a.attack(p, attack(0)).unwrap();
    a.step().unwrap();
    a.acknowledge(p, 0).unwrap();
    a.acknowledge(p, 0).unwrap();
    assert_eq!(a.retained_outcomes(p), Ok(0));
    assert_eq!(a.attack(p, attack(1)), Err(Error::RequestStale));
    assert_eq!(a.attack(p, attack(2)), Ok(Reply::Queued));
}
#[test]
fn rejected_admission_preserves_retry_ids_and_movement_watermark() {
    let mut a = make(
        Rules {
            max_inputs: 1,
            max_inputs_per_session: 1,
            ..Rules::default()
        },
        4,
    );
    let (p, _) = pair(&mut a);
    let m = MoveInput {
        sequence: 55,
        axes: [0, 0],
    };
    assert_eq!(
        a.movement(
            p,
            MoveInput {
                axes: [1001, 0],
                ..m
            }
        ),
        Err(Error::Simulation(theia_sim::Error::InvalidIntent))
    );
    a.movement(p, m).unwrap();
    assert_eq!(a.applied_move(p), Ok(None));
    assert_eq!(
        a.attack(p, attack(0)),
        Err(Error::Simulation(theia_sim::Error::InputBudget))
    );
    assert_eq!(a.retained_outcomes(p), Ok(0));
    a.step().unwrap();
    a.attack(p, attack(0)).unwrap();
    let next = MoveInput { sequence: 56, ..m };
    assert_eq!(
        a.movement(p, next),
        Err(Error::Simulation(theia_sim::Error::InputBudget))
    );
    a.step().unwrap();
    a.movement(p, next).unwrap();
    a.step().unwrap();
    assert_eq!(a.applied_move(p), Ok(Some(56)));
    assert_eq!(a.movement(p, m), Err(Error::StaleMove));
}
#[test]
fn foreign_disconnected_peers_and_target_compaction_are_safe() {
    let mut a = make(Rules::default(), 4);
    let (p, q) = pair(&mut a);
    let mut other = make(Rules::default(), 4);
    let (foreign, _) = pair(&mut other);
    assert_eq!(a.attack(foreign, attack(0)), Err(Error::UnknownPeer));
    a.attack(p, attack(0)).unwrap();
    a.disconnect(q).unwrap();
    a.attach_actor(id(3), [1000, 0]).unwrap();
    assert_eq!(
        a.step().unwrap().outcomes[0].1.result,
        AttackResult::Rejected(AttackReject::UnknownTarget)
    );
    assert_eq!(a.actor(id(3)).unwrap().health, 100);
    a.attack(p, attack(1)).unwrap();
    a.disconnect(p).unwrap();
    let fresh = a.attach_actor(id(1), [0, 0]).unwrap();
    assert_ne!(fresh, p);
    assert_eq!(a.attack(p, attack(2)), Err(Error::UnknownPeer));
    assert!(a.step().unwrap().outcomes.is_empty());
    assert_eq!(a.attack(fresh, attack(0)), Ok(Reply::Queued));
}
#[test]
fn dead_attacker_has_cached_semantic_rejection() {
    let mut a = make(
        Rules {
            attack_damage: 100,
            ..Rules::default()
        },
        4,
    );
    let (p, q) = pair(&mut a);
    a.attack(p, attack(0)).unwrap();
    a.step().unwrap();
    let input = AttackInput {
        sequence: 0,
        target: id(1),
    };
    let Reply::Completed(result) = a.attack(q, input).unwrap() else {
        panic!("expected completed dead rejection")
    };
    assert_eq!(
        result.result,
        AttackResult::Rejected(AttackReject::DeadAttacker)
    );
    assert_eq!(a.attack(q, input), Ok(Reply::Completed(result)));
    a.acknowledge(q, 0).unwrap();
    assert_eq!(a.attack(q, input), Err(Error::RequestStale));
}
#[test]
fn counters_never_wrap_and_terminal_request_can_be_acknowledged() {
    let mut a = make(Rules::default(), 4);
    let (p, _) = pair(&mut a);
    a.peers.get_mut(&p).unwrap().next_internal = Some(u64::MAX);
    a.movement(
        p,
        MoveInput {
            sequence: u64::MAX,
            axes: [0, 0],
        },
    )
    .unwrap();
    a.step().unwrap();
    assert_eq!(a.attack(p, attack(0)), Err(Error::SequenceExhausted));
    assert_eq!(a.retained_outcomes(p), Ok(0));
    let mut a = make(Rules::default(), 4);
    let (p, _) = pair(&mut a);
    a.peers.get_mut(&p).unwrap().floor = u64::MAX;
    a.attack(p, attack(u64::MAX)).unwrap();
    a.step().unwrap();
    a.acknowledge(p, u64::MAX).unwrap();
    assert_eq!(a.retained_outcomes(p), Ok(0));
    assert_eq!(a.attack(p, attack(u64::MAX)), Err(Error::RequestExhausted));
}
#[test]
fn invalid_limits_and_capacity_leave_live_peers_intact() {
    let scene = || {
        Scene::new(
            Rect {
                min: [-4000; 2],
                max: [4000; 2],
            },
            vec![],
        )
        .unwrap()
    };
    for window in [0, 257] {
        assert!(matches!(
            Authority::new(
                scene(),
                Rules::default(),
                Limits {
                    request_window: window,
                    ..Limits::default()
                }
            ),
            Err(Error::InvalidLimits)
        ));
    }
    let mut a = Authority::new(
        scene(),
        Rules::default(),
        Limits {
            max_peers: 1,
            request_window: 1,
        },
    )
    .unwrap();
    let p = a.attach_actor(id(1), [0, 0]).unwrap();
    assert_eq!(a.attach_actor(id(2), [1000, 0]), Err(Error::Capacity));
    a.attack(p, attack(0)).unwrap();
    a.step().unwrap();
    assert_eq!(a.attack(p, attack(1)), Err(Error::RequestWindow));
    assert_eq!(a.retained_outcomes(p), Ok(1));
}
