//! Basic server-owned range/LOS/cooldown/health. Point targets on the flat fixture.
use crate::{World, WorldId};
#[derive(Clone, Copy, Debug)]
pub struct AttackInput {
    pub sequence: u64,
    pub target: WorldId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackReject {
    SelfTarget,
    UnknownTarget,
    DeadAttacker,
    DeadTarget,
    Cooldown,
    Range,
    LineOfSight,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackResult {
    Hit { health_after: u32 },
    Rejected(AttackReject),
}
#[derive(Clone, Copy, Debug)]
pub struct AttackOutcome {
    pub attacker: WorldId,
    pub target: WorldId,
    pub sequence: u64,
    pub result: AttackResult,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealthChange {
    pub id: WorldId,
    pub health: u32,
}
impl World {
    pub(crate) fn resolve_attack(
        &mut self,
        attacker: WorldId,
        target: WorldId,
        tick: u64,
        ready_tick: u64,
        dirty: &mut [bool],
    ) -> AttackResult {
        let row = self.rows[&attacker];
        let rejection = if self.health[row] == 0 {
            Some(AttackReject::DeadAttacker)
        } else if attacker == target {
            Some(AttackReject::SelfTarget)
        } else {
            None
        };
        if let Some(reason) = rejection {
            return AttackResult::Rejected(reason);
        }
        let Some(&target_row) = self.rows.get(&target) else {
            return AttackResult::Rejected(AttackReject::UnknownTarget);
        };
        if self.health[target_row] == 0 {
            return AttackResult::Rejected(AttackReject::DeadTarget);
        }
        if tick < self.next_attack[row] {
            return AttackResult::Rejected(AttackReject::Cooldown);
        }
        let distance = (0..2)
            .map(|i| {
                let d =
                    i64::from(self.positions[row][i]) - i64::from(self.positions[target_row][i]);
                d * d
            })
            .sum::<i64>() as u64;
        if distance > u64::from(self.rules.attack_range_mm) * u64::from(self.rules.attack_range_mm)
        {
            return AttackResult::Rejected(AttackReject::Range);
        }
        if !self
            .collision
            .line_of_sight(self.positions[row], self.positions[target_row])
        {
            return AttackResult::Rejected(AttackReject::LineOfSight);
        }
        self.next_attack[row] = ready_tick;
        self.health[target_row] = self.health[target_row].saturating_sub(self.rules.attack_damage);
        if self.health[target_row] == 0 {
            self.intents[target_row] = [0; 2];
            self.expires[target_row] = 0;
        }
        dirty[target_row] = true;
        AttackResult::Hit {
            health_after: self.health[target_row],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, MoveInput, Rect, Rules, Scene};
    fn id(n: u64) -> WorldId {
        WorldId::new(n).unwrap()
    }
    fn world(obstacles: Vec<Rect>, rules: Rules) -> World {
        World::new(
            Scene::new(
                Rect {
                    min: [-10000, -10000],
                    max: [10000, 10000],
                },
                obstacles,
            )
            .unwrap(),
            rules,
        )
        .unwrap()
    }
    #[test]
    fn server_damage_cooldown_replay_and_death_are_authoritative() {
        let mut w = world(vec![], Rules::default());
        let a = w.spawn_at_boundary(id(1), [0, 0]).unwrap();
        let b = w.spawn_at_boundary(id(2), [1000, 0]).unwrap();
        for sequence in 0..4 {
            w.submit_attack(
                a,
                AttackInput {
                    sequence,
                    target: id(2),
                },
            )
            .unwrap();
        }
        let frame = w.step().unwrap();
        assert_eq!(
            frame.attacks[0].result,
            AttackResult::Hit { health_after: 75 }
        );
        assert!(
            frame.attacks[1..]
                .iter()
                .all(|v| v.result == AttackResult::Rejected(AttackReject::Cooldown))
        );
        assert_eq!(
            frame.health_changes,
            vec![HealthChange {
                id: id(2),
                health: 75
            }]
        );
        assert_eq!(
            w.submit_attack(
                a,
                AttackInput {
                    sequence: 3,
                    target: id(2)
                }
            ),
            Err(Error::Sequence)
        );
        for tick in 2..=13 {
            w.submit_attack(
                a,
                AttackInput {
                    sequence: tick + 2,
                    target: id(2),
                },
            )
            .unwrap();
            let frame = w.step().unwrap();
            assert_eq!(
                frame.attacks[0].result,
                if (tick - 1) % 4 == 0 {
                    AttackResult::Hit {
                        health_after: 100 - 25 * ((tick - 1) / 4 + 1) as u32,
                    }
                } else {
                    AttackResult::Rejected(AttackReject::Cooldown)
                }
            );
        }
        assert_eq!(w.actor(id(2)).unwrap().health, 0);
        assert_eq!(
            w.submit(
                b,
                MoveInput {
                    sequence: 0,
                    axes: [1000, 0]
                }
            ),
            Err(Error::DeadActor)
        );
        assert_eq!(
            w.submit_attack(
                b,
                AttackInput {
                    sequence: 0,
                    target: id(1)
                }
            ),
            Err(Error::DeadActor)
        );
        w.submit_attack(
            a,
            AttackInput {
                sequence: 99,
                target: id(2),
            },
        )
        .unwrap();
        assert_eq!(
            w.step().unwrap().attacks[0].result,
            AttackResult::Rejected(AttackReject::DeadTarget)
        );
    }
    #[test]
    fn target_range_self_and_line_of_sight_checks_use_server_state() {
        let wall = Rect {
            min: [-100, -100],
            max: [100, 100],
        };
        let mut w = world(vec![wall], Rules::default());
        let a = w.spawn_at_boundary(id(1), [-700, 0]).unwrap();
        w.spawn_at_boundary(id(2), [700, 0]).unwrap();
        w.spawn_at_boundary(id(3), [3000, 0]).unwrap();
        for (sequence, target, reason) in [
            (0, id(1), AttackReject::SelfTarget),
            (1, id(99), AttackReject::UnknownTarget),
            (2, id(3), AttackReject::Range),
            (3, id(2), AttackReject::LineOfSight),
        ] {
            w.submit_attack(a, AttackInput { sequence, target })
                .unwrap();
            assert_eq!(
                w.step().unwrap().attacks[0].result,
                AttackResult::Rejected(reason)
            );
        }
        assert_eq!(w.actor(id(2)).unwrap().health, 100);
        assert_eq!(w.next_attack[w.rows[&id(1)]], 0);
    }
    #[test]
    fn movement_precedes_combat_and_dead_attackers_cannot_hit_later_in_tick() {
        let mut w = world(
            vec![],
            Rules {
                attack_damage: 100,
                ..Rules::default()
            },
        );
        let a = w.spawn_at_boundary(id(1), [0, 0]).unwrap();
        let b = w.spawn_at_boundary(id(2), [1600, 0]).unwrap();
        w.submit_attack(
            a,
            AttackInput {
                sequence: 0,
                target: id(2),
            },
        )
        .unwrap();
        w.submit(
            b,
            MoveInput {
                sequence: 0,
                axes: [-1000, 0],
            },
        )
        .unwrap();
        w.submit_attack(
            b,
            AttackInput {
                sequence: 1,
                target: id(1),
            },
        )
        .unwrap();
        let frame = w.step().unwrap();
        assert_eq!(w.actor(id(2)).unwrap().position_mm, [1450, 0]);
        assert_eq!(
            frame.attacks[0].result,
            AttackResult::Hit { health_after: 0 }
        );
        assert_eq!(
            frame.attacks[1].result,
            AttackResult::Rejected(AttackReject::DeadAttacker)
        );
        assert_eq!(w.actor(id(1)).unwrap().health, 100);
        let next = w.step().unwrap();
        assert!(next.changes.is_empty());
    }
    #[test]
    fn multiple_sources_coalesce_health_and_nonblocking_actors_do_not_occlude() {
        let mut w = world(vec![], Rules::default());
        let a = w.spawn_at_boundary(id(1), [0, 0]).unwrap();
        let b = w.spawn_at_boundary(id(2), [0, 0]).unwrap();
        w.spawn_at_boundary(id(3), [500, 0]).unwrap();
        w.spawn_at_boundary(id(4), [1000, 0]).unwrap();
        for s in [a, b] {
            w.submit_attack(
                s,
                AttackInput {
                    sequence: 0,
                    target: id(4),
                },
            )
            .unwrap();
        }
        let frame = w.step().unwrap();
        assert_eq!(frame.attacks.len(), 2);
        assert!(
            frame
                .attacks
                .iter()
                .all(|v| matches!(v.result, AttackResult::Hit { .. }))
        );
        assert_eq!(
            frame.health_changes,
            vec![HealthChange {
                id: id(4),
                health: 50
            }]
        );
    }
    #[test]
    fn exact_range_and_target_despawn_never_damage_a_compacted_row() {
        let mut w = world(vec![], Rules::default());
        let a = w.spawn_at_boundary(id(1), [0, 0]).unwrap();
        w.spawn_at_boundary(id(2), [1500, 0]).unwrap();
        w.spawn_at_boundary(id(3), [1000, 0]).unwrap();
        w.submit_attack(
            a,
            AttackInput {
                sequence: 0,
                target: id(2),
            },
        )
        .unwrap();
        assert_eq!(
            w.step().unwrap().attacks[0].result,
            AttackResult::Hit { health_after: 75 }
        );
        w.submit_attack(
            a,
            AttackInput {
                sequence: 1,
                target: id(2),
            },
        )
        .unwrap();
        w.despawn_at_boundary(id(2)).unwrap();
        let frame = w.step().unwrap();
        assert_eq!(
            frame.attacks[0].result,
            AttackResult::Rejected(AttackReject::UnknownTarget)
        );
        assert_eq!(w.actor(id(3)).unwrap().health, 100);
        assert!(frame.health_changes.is_empty());
    }
    #[test]
    fn attack_counter_exhaustion_preserves_queued_actions_and_health() {
        let mut w = world(vec![], Rules::default());
        let a = w.spawn_at_boundary(id(1), [0, 0]).unwrap();
        w.spawn_at_boundary(id(2), [1000, 0]).unwrap();
        w.submit_attack(
            a,
            AttackInput {
                sequence: 0,
                target: id(2),
            },
        )
        .unwrap();
        w.tick = u64::MAX - 1;
        let before = w.snapshot();
        assert!(matches!(w.step(), Err(Error::CounterExhausted)));
        assert_eq!(w.snapshot(), before);
        assert_eq!(w.pending.len(), 1);
        for rules in [
            Rules {
                max_health: 0,
                ..Rules::default()
            },
            Rules {
                attack_damage: 0,
                ..Rules::default()
            },
            Rules {
                attack_range_mm: 0,
                ..Rules::default()
            },
            Rules {
                attack_cooldown_ticks: 0,
                ..Rules::default()
            },
        ] {
            assert!(matches!(
                World::new(
                    Scene::new(
                        Rect {
                            min: [-10000; 2],
                            max: [10000; 2]
                        },
                        vec![]
                    )
                    .unwrap(),
                    rules
                ),
                Err(Error::InvalidRules)
            ));
        }
    }
    #[test]
    fn seeded_attack_schedule_matches_independent_health_cooldown_model() {
        let mut w = world(
            vec![],
            Rules {
                max_health: 100000,
                attack_damage: 7,
                attack_cooldown_ticks: 5,
                ..Rules::default()
            },
        );
        let a = w.spawn_at_boundary(id(1), [0, 0]).unwrap();
        w.spawn_at_boundary(id(2), [1000, 0]).unwrap();
        let mut seed = 7919u64;
        let mut health = 100000;
        let mut ready = 0;
        let mut sequence = 0;
        for tick in 1..=5000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let requests = ((seed >> 32) % 5) as usize;
            for _ in 0..requests {
                w.submit_attack(
                    a,
                    AttackInput {
                        sequence,
                        target: id(2),
                    },
                )
                .unwrap();
                sequence += 1;
            }
            let frame = w.step().unwrap();
            let mut changed = false;
            assert_eq!(frame.attacks.len(), requests);
            for outcome in frame.attacks {
                let expected = if tick >= ready {
                    health -= 7;
                    ready = tick + 5;
                    changed = true;
                    AttackResult::Hit {
                        health_after: health,
                    }
                } else {
                    AttackResult::Rejected(AttackReject::Cooldown)
                };
                assert_eq!(outcome.result, expected);
            }
            assert_eq!(w.actor(id(2)).unwrap().health, health);
            assert_eq!(frame.health_changes.len(), usize::from(changed));
        }
    }
}
