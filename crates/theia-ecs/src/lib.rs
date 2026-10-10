//! Bounded CPU reference for the M1 presentation ECS qualification.
//! Not an authoritative gameplay ECS or the production GPU allocator.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_READER: AtomicU64 = AtomicU64::new(1);

pub type Epoch = u64;
pub type WorldId = u64;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Handle {
    pub slot: u32,
    pub generation: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Archetype {
    Stationary,
    Moving,
}
impl Archetype {
    fn index(self) -> usize {
        match self {
            Self::Stationary => 0,
            Self::Moving => 1,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub archetype: Archetype,
    pub chunk: usize,
    pub row: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entity {
    pub world_id: WorldId,
    pub position: [f32; 2],
    pub velocity: Option<[f32; 2]>,
}
#[derive(Clone, Copy, Debug)]
pub enum Command {
    Spawn(Entity),
    Despawn(Handle),
    SetVelocity(Handle, Option<[f32; 2]>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationError {
    StaleHandle,
    DuplicateWorldId,
    SlotsExhausted,
    ChunksExhausted,
    NonFiniteComponent,
}
#[derive(Clone, Debug, Default)]
struct Chunk {
    handles: Vec<Handle>,
    positions: Vec<[f32; 2]>,
    velocities: Vec<[f32; 2]>,
}
#[derive(Clone, Debug)]
struct Slot {
    generation: u32,
    world_id: WorldId,
    location: Option<Location>,
    retired: Option<Epoch>,
    exhausted: bool,
}
/// Owned reader lease plus reference data. Release through the originating Oracle.
/// Data is copied for CPU correctness comparison, not a runtime full-ECS copy policy.
#[derive(Debug)]
pub struct Reader {
    id: u64,
    pub epoch: Epoch,
    pub entities: BTreeMap<Handle, Entity>,
}
#[derive(Debug)]
pub struct Oracle {
    epoch: Epoch,
    chunk_capacity: usize,
    chunks: [Vec<Chunk>; 2],
    slots: Vec<Slot>,
    readers: BTreeMap<u64, Epoch>,
}
impl Oracle {
    pub fn new(slots: u32, chunks_per_archetype: usize, chunk_capacity: usize) -> Self {
        assert!(slots > 0 && chunks_per_archetype > 0 && chunk_capacity > 0);
        Self {
            epoch: 0,
            chunk_capacity,
            chunks: std::array::from_fn(|_| vec![Chunk::default(); chunks_per_archetype]),
            slots: vec![
                Slot {
                    generation: 1,
                    world_id: 0,
                    location: None,
                    retired: None,
                    exhausted: false,
                };
                slots as usize
            ],
            readers: BTreeMap::new(),
        }
    }
    pub fn epoch(&self) -> Epoch {
        self.epoch
    }
    pub fn location(&self, handle: Handle) -> Option<Location> {
        let slot = self.slots.get(handle.slot as usize)?;
        (slot.generation == handle.generation)
            .then_some(slot.location)
            .flatten()
    }
    pub fn entity(&self, handle: Handle) -> Option<Entity> {
        let location = self.location(handle)?;
        let chunk = &self.chunks[location.archetype.index()][location.chunk];
        Some(Entity {
            world_id: self.slots[handle.slot as usize].world_id,
            position: chunk.positions[location.row],
            velocity: (location.archetype == Archetype::Moving)
                .then(|| chunk.velocities[location.row]),
        })
    }
    pub fn entities(&self) -> BTreeMap<Handle, Entity> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                let handle = Handle {
                    slot: index as u32,
                    generation: slot.generation,
                };
                self.entity(handle).map(|entity| (handle, entity))
            })
            .collect()
    }
    pub fn retain(&mut self) -> Reader {
        let id = NEXT_READER
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .expect("reader ID overflow");
        self.readers.insert(id, self.epoch);
        Reader {
            id,
            epoch: self.epoch,
            entities: self.entities(),
        }
    }
    pub fn release(&mut self, reader: Reader) {
        assert_eq!(
            self.readers.remove(&reader.id),
            Some(reader.epoch),
            "foreign reader lease"
        );
        self.reclaim();
    }
    fn reclaim(&mut self) {
        let oldest = self.readers.values().min().copied();
        for slot in &mut self.slots {
            if slot
                .retired
                .is_some_and(|epoch| oldest.is_none_or(|reader| reader >= epoch))
            {
                slot.retired = None;
                if let Some(next) = slot.generation.checked_add(1) {
                    slot.generation = next;
                } else {
                    slot.exhausted = true;
                }
            }
        }
    }
    fn destination(&self, archetype: Archetype) -> Result<Location, MutationError> {
        self.chunks[archetype.index()]
            .iter()
            .enumerate()
            .find(|(_, chunk)| chunk.handles.len() < self.chunk_capacity)
            .map(|(chunk, column)| Location {
                archetype,
                chunk,
                row: column.handles.len(),
            })
            .ok_or(MutationError::ChunksExhausted)
    }
    fn insert(&mut self, handle: Handle, entity: Entity, location: Location) {
        let chunk = &mut self.chunks[location.archetype.index()][location.chunk];
        chunk.handles.push(handle);
        chunk.positions.push(entity.position);
        if let Some(velocity) = entity.velocity {
            chunk.velocities.push(velocity);
        }
        let slot = &mut self.slots[handle.slot as usize];
        slot.world_id = entity.world_id;
        slot.location = Some(location);
    }
    fn remove(&mut self, location: Location) {
        let chunk = &mut self.chunks[location.archetype.index()][location.chunk];
        chunk.handles.swap_remove(location.row);
        chunk.positions.swap_remove(location.row);
        if location.archetype == Archetype::Moving {
            chunk.velocities.swap_remove(location.row);
        }
        if let Some(moved) = chunk.handles.get(location.row) {
            self.slots[moved.slot as usize].location = Some(location);
        }
    }
    fn mutate(&mut self, command: Command) -> Result<Handle, MutationError> {
        match command {
            Command::Spawn(entity) => {
                if !finite(entity.position) || entity.velocity.is_some_and(|v| !finite(v)) {
                    return Err(MutationError::NonFiniteComponent);
                }
                if self
                    .slots
                    .iter()
                    .any(|slot| slot.location.is_some() && slot.world_id == entity.world_id)
                {
                    return Err(MutationError::DuplicateWorldId);
                }
                let index = self
                    .slots
                    .iter()
                    .position(|slot| {
                        slot.location.is_none() && slot.retired.is_none() && !slot.exhausted
                    })
                    .ok_or(MutationError::SlotsExhausted)?;
                let archetype = if entity.velocity.is_some() {
                    Archetype::Moving
                } else {
                    Archetype::Stationary
                };
                let destination = self.destination(archetype)?;
                let handle = Handle {
                    slot: index as u32,
                    generation: self.slots[index].generation,
                };
                self.insert(handle, entity, destination);
                Ok(handle)
            }
            Command::Despawn(handle) => {
                let location = self.location(handle).ok_or(MutationError::StaleHandle)?;
                self.remove(location);
                let slot = &mut self.slots[handle.slot as usize];
                slot.location = None;
                slot.retired = Some(self.epoch);
                Ok(handle)
            }
            Command::SetVelocity(handle, velocity) => {
                if velocity.is_some_and(|v| !finite(v)) {
                    return Err(MutationError::NonFiniteComponent);
                }
                let source = self.location(handle).ok_or(MutationError::StaleHandle)?;
                let mut entity = self.entity(handle).unwrap();
                entity.velocity = velocity;
                let archetype = if velocity.is_some() {
                    Archetype::Moving
                } else {
                    Archetype::Stationary
                };
                if source.archetype == archetype {
                    if let Some(value) = velocity {
                        self.chunks[archetype.index()][source.chunk].velocities[source.row] = value;
                    }
                } else {
                    // Reserve destination before changing the source: overflow leaves the entity intact.
                    let destination = self.destination(archetype)?;
                    self.remove(source);
                    self.insert(handle, entity, destination);
                }
                Ok(handle)
            }
        }
    }
    /// Publish a bounded, ordered mutation batch. Individual failures leave their entity unchanged;
    /// earlier successes stay committed. Empty batches still publish an epoch for retirement tests.
    pub fn publish(&mut self, commands: &[Command]) -> Vec<Result<Handle, MutationError>> {
        self.epoch = self.epoch.checked_add(1).expect("epoch overflow");
        self.reclaim();
        let results = commands
            .iter()
            .map(|command| self.mutate(*command))
            .collect();
        self.reclaim();
        results
    }
    /// Bulk visual integration; structural epoch publication remains explicit.
    pub fn integrate(&mut self, dt: f32) -> Result<(), MutationError> {
        if !dt.is_finite() || dt < 0.0 {
            return Err(MutationError::NonFiniteComponent);
        }
        for chunk in &mut self.chunks[Archetype::Moving.index()] {
            for (position, velocity) in chunk.positions.iter_mut().zip(&chunk.velocities) {
                for axis in 0..2 {
                    position[axis] += velocity[axis] * dt;
                }
            }
        }
        Ok(())
    }
    /// Validate dense columns and the identity/location bijection after every qualification batch.
    pub fn check_invariants(&self) -> Result<(), String> {
        let mut seen = BTreeSet::new();
        let mut worlds = BTreeSet::new();
        for archetype in [Archetype::Stationary, Archetype::Moving] {
            for (chunk_index, chunk) in self.chunks[archetype.index()].iter().enumerate() {
                if chunk.handles.len() > self.chunk_capacity
                    || chunk.positions.len() != chunk.handles.len()
                    || chunk.velocities.len()
                        != if archetype == Archetype::Moving {
                            chunk.handles.len()
                        } else {
                            0
                        }
                {
                    return Err("invalid chunk columns".into());
                }
                for (row, handle) in chunk.handles.iter().enumerate() {
                    if !seen.insert(*handle)
                        || self.location(*handle)
                            != Some(Location {
                                archetype,
                                chunk: chunk_index,
                                row,
                            })
                    {
                        return Err("invalid dense identity/location mapping".into());
                    }
                    let entity = self.entity(*handle).ok_or("missing entity")?;
                    if !worlds.insert(entity.world_id) {
                        return Err("duplicate world identity".into());
                    }
                }
            }
        }
        if seen.len()
            != self
                .slots
                .iter()
                .filter(|slot| slot.location.is_some())
                .count()
        {
            return Err("orphan metadata location".into());
        }
        if self
            .slots
            .iter()
            .any(|slot| slot.location.is_some() && (slot.retired.is_some() || slot.exhausted))
        {
            return Err("live retired/exhausted slot".into());
        }
        Ok(())
    }
}
fn finite(value: [f32; 2]) -> bool {
    value.iter().all(|axis| axis.is_finite())
}

#[cfg(test)]
mod tests;
