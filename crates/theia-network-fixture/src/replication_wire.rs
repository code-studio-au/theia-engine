//! Separate bounded replication-envelope codec. The revised
//! THN3 listener/viewer uses this state envelope; other wire versions reject.
use theia_protocol::{delta::Delta, replica::Snapshot};
use theia_sim::{ActorState, WorldId};
pub const MAX_BYTES: usize = 128;
pub const MAX_ACTORS: usize = 2;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    Full(Snapshot),
    Delta(Delta),
    Ack(u64),
    Repair,
}
struct Read<'a>(&'a [u8]);
impl Read<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], &'static str> {
        let value = self
            .0
            .get(..N)
            .ok_or("truncated replication message")?
            .try_into()
            .map_err(|_| "invalid length")?;
        self.0 = &self.0[N..];
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8, &'static str> {
        Ok(self.take::<1>()?[0])
    }
    fn tick(&mut self) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(self.take()?))
    }
    fn id(&mut self) -> Result<WorldId, &'static str> {
        WorldId::new(self.tick()?).map_err(|_| "zero logical ID")
    }
    fn optional_tick(&mut self) -> Result<Option<u64>, &'static str> {
        match self.byte()? {
            0 => Ok(None),
            1 => Ok(Some(self.tick()?)),
            _ => Err("unknown optional tick tag"),
        }
    }
    fn count(&mut self) -> Result<usize, &'static str> {
        let n = self.byte()? as usize;
        if n > MAX_ACTORS {
            Err("replication actor count limit")
        } else {
            Ok(n)
        }
    }
    fn actors(&mut self) -> Result<Vec<ActorState>, &'static str> {
        let count = self.count()?;
        let mut actors = Vec::with_capacity(count);
        for _ in 0..count {
            actors.push(ActorState {
                id: self.id()?,
                position_mm: [
                    i32::from_le_bytes(self.take()?),
                    i32::from_le_bytes(self.take()?),
                ],
                health: u32::from_le_bytes(self.take()?),
            });
        }
        Ok(actors)
    }
}
fn optional_tick(bytes: &mut Vec<u8>, tick: Option<u64>) {
    bytes.push(u8::from(tick.is_some()));
    if let Some(tick) = tick {
        bytes.extend(tick.to_le_bytes());
    }
}
fn actors(bytes: &mut Vec<u8>, actors: &[ActorState]) {
    bytes.push(actors.len() as u8);
    for actor in actors {
        bytes.extend(actor.id.value().to_le_bytes());
        for p in actor.position_mm {
            bytes.extend(p.to_le_bytes());
        }
        bytes.extend(actor.health.to_le_bytes());
    }
}
/// Validate caller values/counts before encoding; invalid input never asserts/panics.
pub fn encode(message: &Message) -> Result<Vec<u8>, &'static str> {
    match message {
        Message::Full(s) => s
            .validate(MAX_ACTORS)
            .map_err(|_| "invalid complete state")?,
        Message::Delta(d) => d
            .validate(MAX_ACTORS)
            .map_err(|_| "invalid delta envelope")?,
        _ => {}
    }
    let mut bytes = Vec::with_capacity(MAX_BYTES);
    bytes.extend(b"TDR1");
    match message {
        Message::Full(s) => {
            bytes.push(0);
            bytes.extend(s.tick.to_le_bytes());
            optional_tick(&mut bytes, s.applied_move);
            actors(&mut bytes, &s.actors);
        }
        Message::Delta(d) => {
            bytes.push(1);
            bytes.extend(d.baseline_tick.to_le_bytes());
            bytes.extend(d.tick.to_le_bytes());
            optional_tick(&mut bytes, d.applied_move);
            actors(&mut bytes, &d.upserts);
            bytes.push(d.removed.len() as u8);
            for id in &d.removed {
                bytes.extend(id.value().to_le_bytes());
            }
        }
        Message::Ack(tick) => {
            bytes.push(2);
            bytes.extend(tick.to_le_bytes());
        }
        Message::Repair => bytes.push(3),
    }
    if bytes.len() > MAX_BYTES {
        return Err("oversize replication message");
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8]) -> Result<Message, &'static str> {
    if bytes.len() > MAX_BYTES {
        return Err("oversize replication message");
    }
    let mut read = Read(bytes);
    if read.take::<4>()? != *b"TDR1" {
        return Err("unknown replication version");
    }
    let message = match read.byte()? {
        0 => Message::Full(Snapshot {
            tick: read.tick()?,
            applied_move: read.optional_tick()?,
            actors: read.actors()?,
        }),
        1 => {
            let baseline_tick = read.tick()?;
            let tick = read.tick()?;
            let applied_move = read.optional_tick()?;
            let upserts = read.actors()?;
            let count = read.count()?;
            let mut removed = Vec::with_capacity(count);
            for _ in 0..count {
                removed.push(read.id()?);
            }
            Message::Delta(Delta {
                baseline_tick,
                tick,
                applied_move,
                upserts,
                removed,
            })
        }
        2 => Message::Ack(read.tick()?),
        3 => Message::Repair,
        _ => return Err("unknown replication kind"),
    };
    if !read.0.is_empty() {
        return Err("trailing replication bytes");
    }
    match &message {
        Message::Full(s) => s
            .validate(MAX_ACTORS)
            .map_err(|_| "invalid complete state")?,
        Message::Delta(d) => d
            .validate(MAX_ACTORS)
            .map_err(|_| "invalid delta envelope")?,
        _ => {}
    }
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn actor(id: u64) -> ActorState {
        ActorState {
            id: WorldId::new(id).unwrap(),
            position_mm: [-1000, 1000],
            health: 100,
        }
    }
    fn full() -> Snapshot {
        Snapshot {
            tick: u64::MAX - 1,
            applied_move: Some(u64::MAX),
            actors: vec![actor(10), actor(20)],
        }
    }
    #[test]
    fn canonical_empty_full_maximal_delta_ack_and_repair_roundtrip_all_truncations() {
        let base = full();
        let target = Snapshot {
            tick: u64::MAX,
            actors: vec![actor(30), actor(40)],
            ..base.clone()
        };
        let messages = [
            Message::Full(base.clone()),
            Message::Full(Snapshot {
                tick: 0,
                applied_move: None,
                actors: vec![],
            }),
            Message::Delta(Delta::between(&base, &target, MAX_ACTORS).unwrap()),
            Message::Ack(u64::MAX),
            Message::Repair,
        ];
        for message in messages {
            let bytes = encode(&message).unwrap();
            assert_eq!(decode(&bytes), Ok(message));
            assert!(bytes.len() <= MAX_BYTES);
            for n in 0..bytes.len() {
                assert!(decode(&bytes[..n]).is_err());
            }
            let mut longer = bytes.clone();
            longer.push(0);
            assert!(decode(&longer).is_err());
            let mut old = bytes;
            old[..4].copy_from_slice(b"THN2");
            assert!(decode(&old).is_err());
        }
        assert_eq!(encode(&Message::Repair), Ok(b"TDR1\x03".to_vec()));
        assert_eq!(
            encode(&Message::Ack(0x0807060504030201)),
            Ok(b"TDR1\x02\x01\x02\x03\x04\x05\x06\x07\x08".to_vec())
        );
    }
    #[test]
    fn bad_tags_counts_ids_values_order_and_delta_overlap_are_rejected() {
        let bytes = encode(&Message::Full(full())).unwrap();
        for (offset, value) in [(4, 255), (13, 2), (22, 3), (22, 255)] {
            let mut bad = bytes.clone();
            bad[offset] = value;
            assert!(decode(&bad).is_err());
        }
        let mut zero = bytes.clone();
        zero[23..31].fill(0);
        assert!(decode(&zero).is_err());
        let mut duplicate = bytes.clone();
        duplicate[43..51].copy_from_slice(&bytes[23..31]);
        assert!(decode(&duplicate).is_err());
        let mut position = bytes.clone();
        position[31..35].copy_from_slice(&1_000_001i32.to_le_bytes());
        assert!(decode(&position).is_err());
        let mut health = bytes;
        health[39..43].copy_from_slice(&1_000_001u32.to_le_bytes());
        assert!(decode(&health).is_err());
        let base = full();
        let next = Snapshot {
            tick: u64::MAX,
            actors: vec![actor(30), actor(40)],
            ..base.clone()
        };
        let delta = Delta::between(&base, &next, MAX_ACTORS).unwrap();
        let b = encode(&Message::Delta(delta.clone())).unwrap();
        assert_eq!(b.len(), 88);
        let mut invalid = b.clone();
        invalid[71] = 255;
        assert!(decode(&invalid).is_err());
        let mut overlap = b.clone();
        overlap[72..80].copy_from_slice(&b[31..39]);
        assert!(decode(&overlap).is_err());
        let mut repeated = b.clone();
        repeated[80..88].copy_from_slice(&b[72..80]);
        assert!(decode(&repeated).is_err());
        let mut tick = b;
        tick[13..21].copy_from_slice(&base.tick.to_le_bytes());
        assert!(decode(&tick).is_err());
        let mut oversize = full();
        oversize.actors.push(actor(30));
        assert!(encode(&Message::Full(oversize)).is_err());
        let mut bad = delta;
        bad.removed.push(WorldId::new(50).unwrap());
        assert!(encode(&Message::Delta(bad)).is_err());
        assert!(decode(&[0; MAX_BYTES + 1]).is_err());
    }
    #[test]
    fn arbitrary_bounded_bytes_are_canonical_if_accepted() {
        let mut seed = 0x7087u64;
        for n in 0..20000 {
            let mut bytes = vec![0; n % 160];
            for b in &mut bytes {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                *b = (seed >> 32) as u8;
            }
            if n % 2 == 0 && bytes.len() >= 4 {
                bytes[..4].copy_from_slice(b"TDR1");
            }
            if let Ok(message) = decode(&bytes) {
                assert_eq!(encode(&message), Ok(bytes));
            }
        }
    }
}
