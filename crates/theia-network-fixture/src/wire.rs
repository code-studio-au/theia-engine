//! Qualification-only wire version; not a locked engine replication schema.
use theia_protocol::{Error, Outcome, Reply};
use theia_sim::{ActorState, AttackInput, AttackReject, AttackResult, MoveInput, WorldId};
pub const MAX_BYTES: usize = 128;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Profile {
    Laboratory = 0,
    LocalDemo = 1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Welcome {
    pub actor: WorldId,
    pub target: WorldId,
    pub profile: Profile,
}
impl Welcome {
    pub fn validate(self) -> Result<(), &'static str> {
        if self.actor == self.target {
            return Err("duplicate welcome actor/target");
        }
        Ok(())
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum Message {
    Welcome(Welcome),
    Move(MoveInput),
    Attack(AttackInput),
    Ack(u64),
    Response {
        request: u64,
        reply: Result<Reply, Reject>,
    },
    Snapshot {
        tick: u64,
        applied_move: Option<u64>,
        actors: [ActorState; 2],
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    Stale,
    Conflict,
    Window,
    Budget,
    Pending,
    Unknown,
    Exhausted,
    Invalid,
}
impl From<Error> for Reject {
    fn from(error: Error) -> Self {
        match error {
            Error::RequestStale | Error::StaleMove => Self::Stale,
            Error::RequestConflict => Self::Conflict,
            Error::RequestWindow => Self::Window,
            Error::Simulation(theia_sim::Error::InputBudget) | Error::Capacity => Self::Budget,
            Error::OutcomePending => Self::Pending,
            Error::UnknownPeer | Error::OutcomeUnknown => Self::Unknown,
            Error::SequenceExhausted | Error::RequestExhausted => Self::Exhausted,
            _ => Self::Invalid,
        }
    }
}
struct Read<'a>(&'a [u8]);
impl Read<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], &'static str> {
        let bytes = self.0.get(..N).ok_or("truncated message")?;
        let value = bytes.try_into().map_err(|_| "invalid length")?;
        self.0 = &self.0[N..];
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8, &'static str> {
        Ok(self.take::<1>()?[0])
    }
    fn u64(&mut self) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(self.take()?))
    }
    fn u32(&mut self) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(self.take()?))
    }
    fn id(&mut self) -> Result<WorldId, &'static str> {
        WorldId::new(self.u64()?).map_err(|_| "zero logical ID")
    }
}
pub fn encode(message: &Message) -> Vec<u8> {
    let mut b = Vec::with_capacity(MAX_BYTES);
    b.extend_from_slice(b"THN3");
    match message {
        Message::Welcome(v) => {
            b.push(5);
            b.extend(v.actor.value().to_le_bytes());
            b.extend(v.target.value().to_le_bytes());
            b.push(v.profile as u8);
        }
        Message::Move(v) => {
            b.push(0);
            b.extend(v.sequence.to_le_bytes());
            for a in v.axes {
                b.extend(a.to_le_bytes());
            }
        }
        Message::Attack(v) => {
            b.push(1);
            b.extend(v.sequence.to_le_bytes());
            b.extend(v.target.value().to_le_bytes());
        }
        Message::Ack(v) => {
            b.push(2);
            b.extend(v.to_le_bytes());
        }
        Message::Response { request, reply } => {
            b.push(3);
            b.extend(request.to_le_bytes());
            match reply {
                Ok(Reply::Queued) => b.push(0),
                Ok(Reply::Pending) => b.push(1),
                Ok(Reply::Acknowledged) => b.push(3),
                Ok(Reply::Completed(v)) => {
                    assert_eq!(*request, v.request);
                    b.push(2);
                    b.extend(v.tick.to_le_bytes());
                    b.extend(v.attacker.value().to_le_bytes());
                    b.extend(v.target.value().to_le_bytes());
                    match v.result {
                        AttackResult::Hit { health_after } => {
                            b.push(0);
                            b.extend(health_after.to_le_bytes());
                        }
                        AttackResult::Rejected(reason) => b.push(match reason {
                            AttackReject::SelfTarget => 1,
                            AttackReject::UnknownTarget => 2,
                            AttackReject::DeadAttacker => 3,
                            AttackReject::DeadTarget => 4,
                            AttackReject::Cooldown => 5,
                            AttackReject::Range => 6,
                            AttackReject::LineOfSight => 7,
                        }),
                    }
                }
                Err(v) => {
                    b.push(4);
                    b.push(match v {
                        Reject::Stale => 0,
                        Reject::Conflict => 1,
                        Reject::Window => 2,
                        Reject::Budget => 3,
                        Reject::Pending => 4,
                        Reject::Unknown => 5,
                        Reject::Exhausted => 6,
                        Reject::Invalid => 7,
                    });
                }
            }
        }
        Message::Snapshot {
            tick,
            applied_move,
            actors,
        } => {
            b.push(4);
            b.extend(tick.to_le_bytes());
            b.push(u8::from(applied_move.is_some()));
            if let Some(v) = applied_move {
                b.extend(v.to_le_bytes());
            }
            for v in actors {
                b.extend(v.id.value().to_le_bytes());
                for p in v.position_mm {
                    b.extend(p.to_le_bytes());
                }
                b.extend(v.health.to_le_bytes());
            }
        }
    }
    assert!(b.len() <= MAX_BYTES);
    b
}
pub fn decode(bytes: &[u8]) -> Result<Message, &'static str> {
    if bytes.len() > MAX_BYTES {
        return Err("oversize message");
    }
    let mut r = Read(bytes);
    if r.take::<4>()? != *b"THN3" {
        return Err("unknown version");
    }
    let message = match r.byte()? {
        0 => {
            let sequence = r.u64()?;
            let axes = [i16::from_le_bytes(r.take()?), i16::from_le_bytes(r.take()?)];
            if axes.iter().any(|v| !(-1000..=1000).contains(v)) {
                return Err("invalid intent");
            }
            Message::Move(MoveInput { sequence, axes })
        }
        1 => Message::Attack(AttackInput {
            sequence: r.u64()?,
            target: r.id()?,
        }),
        2 => Message::Ack(r.u64()?),
        3 => {
            let request = r.u64()?;
            let reply = match r.byte()? {
                0 => Ok(Reply::Queued),
                1 => Ok(Reply::Pending),
                3 => Ok(Reply::Acknowledged),
                2 => {
                    let tick = r.u64()?;
                    let attacker = r.id()?;
                    let target = r.id()?;
                    let result = match r.byte()? {
                        0 => {
                            let health_after = r.u32()?;
                            if health_after > 1_000_000 {
                                return Err("invalid health");
                            }
                            AttackResult::Hit { health_after }
                        }
                        tag @ 1..=7 => AttackResult::Rejected(match tag {
                            1 => AttackReject::SelfTarget,
                            2 => AttackReject::UnknownTarget,
                            3 => AttackReject::DeadAttacker,
                            4 => AttackReject::DeadTarget,
                            5 => AttackReject::Cooldown,
                            6 => AttackReject::Range,
                            _ => AttackReject::LineOfSight,
                        }),
                        _ => return Err("unknown attack result"),
                    };
                    Ok(Reply::Completed(Outcome {
                        request,
                        tick,
                        attacker,
                        target,
                        result,
                    }))
                }
                4 => Err(match r.byte()? {
                    0 => Reject::Stale,
                    1 => Reject::Conflict,
                    2 => Reject::Window,
                    3 => Reject::Budget,
                    4 => Reject::Pending,
                    5 => Reject::Unknown,
                    6 => Reject::Exhausted,
                    7 => Reject::Invalid,
                    _ => return Err("unknown rejection"),
                }),
                _ => return Err("unknown reply"),
            };
            Message::Response { request, reply }
        }
        4 => {
            let tick = r.u64()?;
            let applied_move = match r.byte()? {
                0 => None,
                1 => Some(r.u64()?),
                _ => return Err("invalid optional sequence"),
            };
            let mut actor = || -> Result<ActorState, &'static str> {
                let id = r.id()?;
                let position_mm = [i32::from_le_bytes(r.take()?), i32::from_le_bytes(r.take()?)];
                let health = r.u32()?;
                if position_mm
                    .iter()
                    .any(|v| !(-1_000_000..=1_000_000).contains(v))
                    || health > 1_000_000
                {
                    return Err("invalid actor state");
                }
                Ok(ActorState {
                    id,
                    position_mm,
                    health,
                })
            };
            let actors = [actor()?, actor()?];
            if actors[0].id >= actors[1].id {
                return Err("unordered or duplicate snapshot ID");
            }
            Message::Snapshot {
                tick,
                applied_move,
                actors,
            }
        }
        5 => {
            let actor = r.id()?;
            let target = r.id()?;
            let profile = match r.byte()? {
                0 => Profile::Laboratory,
                1 => Profile::LocalDemo,
                _ => return Err("unknown scene profile"),
            };
            let welcome = Welcome {
                actor,
                target,
                profile,
            };
            welcome.validate()?;
            Message::Welcome(welcome)
        }
        _ => return Err("unknown kind"),
    };
    if !r.0.is_empty() {
        return Err("trailing bytes");
    }
    Ok(message)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_roundtrip_truncation_extension_and_unknown_version() {
        let id = WorldId::new(10).unwrap();
        let target = WorldId::new(20).unwrap();
        let actor = ActorState {
            id,
            position_mm: [-1000, 1000],
            health: 100,
        };
        let messages = [
            Message::Welcome(Welcome {
                actor: id,
                target,
                profile: Profile::LocalDemo,
            }),
            Message::Move(MoveInput {
                sequence: u64::MAX,
                axes: [-1000, 1000],
            }),
            Message::Attack(AttackInput {
                sequence: 0,
                target,
            }),
            Message::Ack(u64::MAX),
            Message::Response {
                request: 5,
                reply: Ok(Reply::Completed(Outcome {
                    request: 5,
                    tick: 33,
                    attacker: id,
                    target,
                    result: AttackResult::Hit { health_after: 0 },
                })),
            },
            Message::Snapshot {
                tick: 2,
                applied_move: Some(u64::MAX),
                actors: [
                    actor,
                    ActorState {
                        id: target,
                        ..actor
                    },
                ],
            },
        ];
        for message in messages {
            let b = encode(&message);
            assert_eq!(decode(&b), Ok(message));
            for n in 0..b.len() {
                assert!(decode(&b[..n]).is_err());
            }
            let mut longer = b.clone();
            longer.push(0);
            assert!(decode(&longer).is_err());
            let mut wrong = b.clone();
            wrong[3] = b'1';
            assert!(decode(&wrong).is_err());
        }
    }
    #[test]
    fn explicit_reply_tags_invalid_ids_and_numeric_limits() {
        let id = WorldId::new(10).unwrap();
        for reject in [
            Reject::Stale,
            Reject::Conflict,
            Reject::Window,
            Reject::Budget,
            Reject::Pending,
            Reject::Unknown,
            Reject::Exhausted,
            Reject::Invalid,
        ] {
            let message = Message::Response {
                request: 0,
                reply: Err(reject),
            };
            assert_eq!(decode(&encode(&message)), Ok(message));
        }
        for reason in [
            AttackReject::SelfTarget,
            AttackReject::UnknownTarget,
            AttackReject::DeadAttacker,
            AttackReject::DeadTarget,
            AttackReject::Cooldown,
            AttackReject::Range,
            AttackReject::LineOfSight,
        ] {
            let message = Message::Response {
                request: 0,
                reply: Ok(Reply::Completed(Outcome {
                    request: 0,
                    tick: 1,
                    attacker: id,
                    target: id,
                    result: AttackResult::Rejected(reason),
                })),
            };
            assert_eq!(decode(&encode(&message)), Ok(message));
        }
        let movement = encode(&Message::Move(MoveInput {
            sequence: 0,
            axes: [0, 0],
        }));
        assert_eq!(movement, [b"THN3".as_slice(), &[0; 13]].concat());
        let mut invalid = movement;
        invalid[13..15].copy_from_slice(&1001i16.to_le_bytes());
        assert!(decode(&invalid).is_err());
        let mut attack = encode(&Message::Attack(AttackInput {
            sequence: 0,
            target: id,
        }));
        attack[13..21].fill(0);
        assert!(decode(&attack).is_err());
        let mut reply = encode(&Message::Response {
            request: 0,
            reply: Ok(Reply::Queued),
        });
        reply[13] = 255;
        assert!(decode(&reply).is_err());
        let actor = ActorState {
            id,
            position_mm: [0, 0],
            health: 100,
        };
        assert!(
            decode(&encode(&Message::Snapshot {
                tick: 0,
                applied_move: None,
                actors: [actor; 2]
            }))
            .is_err()
        );
        let target = WorldId::new(20).unwrap();
        for bad in [
            ActorState {
                id: target,
                health: 1_000_001,
                ..actor
            },
            ActorState {
                id: target,
                position_mm: [i32::MIN, 0],
                ..actor
            },
        ] {
            assert!(
                decode(&encode(&Message::Snapshot {
                    tick: 0,
                    applied_move: None,
                    actors: [actor, bad]
                }))
                .is_err()
            );
        }
    }
    #[test]
    fn welcome_rejects_invalid_profile_duplicate_actor_and_old_wire_version() {
        let actor = WorldId::new(10).unwrap();
        let target = WorldId::new(20).unwrap();
        let mut bytes = encode(&Message::Welcome(Welcome {
            actor,
            target,
            profile: Profile::LocalDemo,
        }));
        bytes[21] = 2;
        assert!(decode(&bytes).is_err());
        let duplicate = Message::Welcome(Welcome {
            actor,
            target: actor,
            profile: Profile::Laboratory,
        });
        assert!(decode(&encode(&duplicate)).is_err());
        let mut old = encode(&Message::Move(MoveInput {
            sequence: 0,
            axes: [0, 0],
        }));
        for version in *b"12" {
            old[3] = version;
            assert!(decode(&old).is_err());
        }
    }
    #[test]
    fn arbitrary_bytes_are_bounded_and_canonical_when_accepted() {
        let mut seed = 719u64;
        for n in 0..20000 {
            let mut b = vec![0; n % 160];
            for v in &mut b {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                *v = (seed >> 32) as u8;
            }
            if n % 2 == 0 && b.len() >= 4 {
                b[..4].copy_from_slice(b"THN3");
            }
            if let Ok(message) = decode(&b) {
                assert_eq!(encode(&message), b);
            }
        }
    }
}
