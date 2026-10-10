mod gpu;
use std::{
    collections::BTreeSet,
    error::Error,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};
use theia_ecs::{Archetype, Command, Entity, Handle, MutationError, Oracle, Reader};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const WORDS: usize = 1541;
const GEN: usize = 5;
const WORLD_LO: usize = 133;
const WORLD_HI: usize = 261;
const ARCH: usize = 389;
const ROW: usize = 517;
const RETIRED: usize = 645;
const EXHAUSTED: usize = 773;
const HANDLE: usize = 901;
const PX: usize = 1029;
const PY: usize = 1157;
const VX: usize = 1285;
const VY: usize = 1413;
fn encode(commands: &[Command]) -> Vec<u32> {
    commands
        .iter()
        .flat_map(|command| {
            let mut words = [0u32; 10];
            match *command {
                Command::Spawn(entity) => {
                    words[3] = entity.world_id as u32;
                    words[4] = (entity.world_id >> 32) as u32;
                    words[5] = entity.position[0].to_bits();
                    words[6] = entity.position[1].to_bits();
                    if let Some(v) = entity.velocity {
                        words[7] = v[0].to_bits();
                        words[8] = v[1].to_bits();
                        words[9] = 1;
                    }
                }
                Command::Despawn(handle) => {
                    words[0] = 1;
                    words[1] = handle.slot;
                    words[2] = handle.generation;
                }
                Command::SetVelocity(handle, velocity) => {
                    words[0] = 2;
                    words[1] = handle.slot;
                    words[2] = handle.generation;
                    if let Some(v) = velocity {
                        words[7] = v[0].to_bits();
                        words[8] = v[1].to_bits();
                        words[9] = 1;
                    }
                }
            }
            words
        })
        .collect()
}
fn status(error: MutationError) -> u32 {
    match error {
        MutationError::StaleHandle => 1,
        MutationError::DuplicateWorldId => 2,
        MutationError::SlotsExhausted => 3,
        MutationError::ChunksExhausted => 4,
        MutationError::NonFiniteComponent => 5,
    }
}
struct Qualification {
    gpu: gpu::Compute,
    oracle: Oracle,
    reader: Option<Reader>,
    retained: Option<Vec<u32>>,
    batches: u32,
    operations: usize,
}
impl Qualification {
    fn batch(&mut self, commands: &[Command], dt: f32, snapshot: bool) -> Result<Vec<Handle>> {
        if commands.len() > 64 {
            return Err("Mutation queue exceeds qualification bound".into());
        }
        if self.oracle.epoch() >= u32::MAX as u64 {
            return Err("32-bit qualification publication epoch exhausted".into());
        }
        let oldest = self.reader.as_ref().map_or(u32::MAX, |reader| {
            u32::try_from(reader.epoch).expect("qualification epoch bound")
        });
        let expected = self.oracle.publish(commands);
        self.oracle.integrate(dt)?;
        let actual = self.gpu.batch(
            &encode(commands),
            oldest,
            dt,
            snapshot,
            self.reader.is_some(),
        )?;
        let mut handles = Vec::new();
        for (index, expected) in expected.iter().enumerate() {
            let record = &actual[index * 3..index * 3 + 3];
            match expected {
                Ok(handle) => {
                    if record != [0, handle.slot, handle.generation] {
                        return Err(format!(
                            "Batch {} operation {index}: result {record:?}, expected {handle:?}",
                            self.batches
                        )
                        .into());
                    }
                    handles.push(*handle);
                }
                Err(error) => {
                    if record[0] != status(*error) {
                        return Err(format!(
                            "Batch {} operation {index}: error {}, expected {error:?}",
                            self.batches, record[0]
                        )
                        .into());
                    }
                }
            }
        }
        self.oracle.check_invariants()?;
        self.check_state(&self.gpu.state())?;
        if self.reader.is_some() && self.gpu.retained() != *self.retained.as_ref().unwrap() {
            return Err("GPU retained reader data changed after publication".into());
        }
        if snapshot {
            if self.reader.is_some() {
                return Err("Only one retained GPU version in qualification".into());
            }
            self.retained = Some(self.gpu.state());
            self.reader = Some(self.oracle.retain());
        }
        self.batches += 1;
        self.operations += commands.len();
        Ok(handles)
    }
    fn release(&mut self) {
        self.oracle.release(self.reader.take().unwrap());
        self.retained = None;
    }
    fn check_state(&self, words: &[u32]) -> Result<()> {
        if words[0] as u64 != self.oracle.epoch() {
            return Err("GPU publication epoch mismatch".into());
        }
        let expected = self.oracle.entities();
        let mut seen = BTreeSet::new();
        for chunk in 0..4 {
            let count = words[1 + chunk] as usize;
            if count > 32 {
                return Err("GPU chunk capacity exceeded".into());
            }
            for row in chunk * 32..chunk * 32 + count {
                let slot = words[HANDLE + row] as usize;
                if slot >= 128
                    || !seen.insert(slot)
                    || words[ROW + slot] as usize != row
                    || words[ARCH + slot] as usize != chunk / 2 + 1
                {
                    return Err("GPU dense identity/location mapping invalid".into());
                }
            }
        }
        if seen.len() != expected.len() {
            return Err("GPU live count mismatch".into());
        }
        for slot in 0..128 {
            if (words[ARCH + slot] != 0) != seen.contains(&slot) {
                return Err("GPU orphan metadata".into());
            }
            if seen.contains(&slot) && (words[RETIRED + slot] != 0 || words[EXHAUSTED + slot] != 0)
            {
                return Err("GPU live slot retired/exhausted".into());
            }
        }
        for (handle, entity) in expected {
            let slot = handle.slot as usize;
            let location = self.oracle.location(handle).unwrap();
            let arch = if location.archetype == Archetype::Moving {
                2
            } else {
                1
            };
            let row = (arch - 1) * 64 + location.chunk * 32 + location.row;
            let world = words[WORLD_LO + slot] as u64 | ((words[WORLD_HI + slot] as u64) << 32);
            if words[GEN + slot] != handle.generation
                || world != entity.world_id
                || words[ROW + slot] as usize != row
                || words[ARCH + slot] as usize != arch
            {
                return Err(format!("GPU identity/location differs for {handle:?}").into());
            }
            let position = [
                f32::from_bits(words[PX + row]),
                f32::from_bits(words[PY + row]),
            ];
            if position != entity.position {
                return Err(format!(
                    "GPU position {position:?}, expected {:?} for {handle:?}",
                    entity.position
                )
                .into());
            }
            if let Some(velocity) = entity.velocity
                && [
                    f32::from_bits(words[VX + row]),
                    f32::from_bits(words[VY + row]),
                ] != velocity
            {
                return Err("GPU velocity differs".into());
            }
        }
        Ok(())
    }
}
fn entity(world: u64, moving: bool) -> Entity {
    Entity {
        world_id: world,
        position: [(world & 255) as f32, 2.0],
        velocity: moving.then_some([0.5, -0.25]),
    }
}
fn run(gpu: gpu::Compute) -> Result<()> {
    let mut q = Qualification {
        gpu,
        oracle: Oracle::new(128, 2, 32),
        reader: None,
        retained: None,
        batches: 0,
        operations: 0,
    };
    // Fill one archetype, overflow its chunk pool, then fill the other and overflow slots.
    let mut history = q.batch(
        &(0..64)
            .map(|i| Command::Spawn(entity((1u64 << 40) + i, false)))
            .collect::<Vec<_>>(),
        0.0,
        false,
    )?;
    q.batch(&[Command::Spawn(entity(999, false))], 0.0, false)?;
    history.extend(
        q.batch(
            &(0..64)
                .map(|i| Command::Spawn(entity((2u64 << 40) + i, true)))
                .collect::<Vec<_>>(),
            0.25,
            true,
        )?,
    );
    q.batch(
        &[
            Command::Spawn(entity(999, true)),
            Command::SetVelocity(history[0], Some([1.0, 1.0])),
            Command::Spawn(entity(1u64 << 40, true)),
            Command::SetVelocity(history[1], Some([f32::NAN, 0.0])),
        ],
        0.25,
        false,
    )?;
    // Retained GPU data survives compaction/migration and retirement. Reuse is forbidden.
    for handles in history.chunks(64) {
        q.batch(
            &handles
                .iter()
                .copied()
                .map(Command::Despawn)
                .collect::<Vec<_>>(),
            0.25,
            false,
        )?;
    }
    q.batch(
        &[
            Command::Spawn(entity(999, true)),
            Command::Despawn(history[0]),
            Command::SetVelocity(history[0], None),
        ],
        0.25,
        false,
    )?;
    q.release();
    let reused = q.batch(
        &[
            Command::Spawn(entity(999, true)),
            Command::Despawn(history[0]),
        ],
        0.25,
        false,
    )?;
    if reused[0].slot != history[0].slot || reused[0].generation <= history[0].generation {
        return Err("Generation did not advance on slot reuse".into());
    }
    history.extend(reused);
    q.batch(
        &[
            Command::SetVelocity(history[history.len() - 1], None),
            Command::SetVelocity(history[history.len() - 1], Some([0.5, -0.25])),
        ],
        0.25,
        false,
    )?;
    println!(
        "GPU deterministic qualification: chunk/slot overflow, conflicts, nonfinite input, retained epochs and stale generation passed"
    );
    let mut seed = 0x1324_5678_u64;
    let mut world = 10_000_u64;
    for iteration in 0..512 {
        let mut commands = Vec::new();
        for _ in 0..16 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            match (seed >> 32) % 3 {
                0 => {
                    commands.push(Command::Spawn(entity(world, seed & 1 != 0)));
                    world += 1;
                }
                1 => commands.push(Command::Despawn(history[seed as usize % history.len()])),
                _ => commands.push(Command::SetVelocity(
                    history[seed as usize % history.len()],
                    (seed & 1 != 0).then_some([0.5, -0.25]),
                )),
            }
        }
        let snapshot = iteration % 32 == 0;
        history.extend(q.batch(&commands, 0.25, snapshot)?);
        if iteration % 32 == 7 {
            q.release();
        }
    }
    // Explicit generation-exhaustion seed: no counter wrap, even after retained readers release.
    q.gpu.reset(u32::MAX)?;
    q.oracle = Oracle::with_initial_generation(128, 2, 32, u32::MAX);
    let mut final_handles = Vec::new();
    for moving in [false, true] {
        final_handles.extend(
            q.batch(
                &(0..64)
                    .map(|i| Command::Spawn(entity(50_000 + i + u64::from(moving) * 64, moving)))
                    .collect::<Vec<_>>(),
                0.25,
                moving,
            )?,
        );
    }
    for handles in final_handles.chunks(64) {
        q.batch(
            &handles
                .iter()
                .copied()
                .map(Command::Despawn)
                .collect::<Vec<_>>(),
            0.25,
            false,
        )?;
    }
    q.release();
    q.batch(&[Command::Spawn(entity(60_000, false))], 0.0, false)?;
    let final_state = q.gpu.state();
    if final_state[GEN..GEN + 128]
        .iter()
        .any(|generation| *generation != u32::MAX)
        || final_state[EXHAUSTED..EXHAUSTED + 128]
            .iter()
            .any(|exhausted| *exhausted != 1)
    {
        return Err("GPU generation exhaustion wrapped or failed to retire slots".into());
    }
    println!("GPU generation exhaustion: all 128 slots permanently retired after reader release");
    println!(
        "GPU ECS verified: {} batches, {} commands, dense state/results vs CPU oracle; parallel integration and retained GPU reader verified",
        q.batches, q.operations
    );
    Ok(())
}
fn main() -> Result<()> {
    let mut filter = "7900".to_string();
    let mut shaders = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build/shaders");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--gpu" => filter = args.next().ok_or("--gpu needs a name")?,
            "--shader-dir" => shaders = args.next().ok_or("--shader-dir needs a directory")?.into(),
            "--help" => {
                println!("Theia M1 ECS qualification: --gpu <substring> --shader-dir <directory>");
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}").into()),
        }
    }
    let validation = Arc::new(gpu::Validation::default());
    let result = gpu::Compute::new(&filter, &shaders, validation.clone()).and_then(run);
    let warnings = validation.warnings.load(Ordering::Relaxed);
    let errors = validation.errors.load(Ordering::Relaxed);
    println!("ECS validation totals including teardown: {warnings} warnings, {errors} errors");
    result?;
    if errors > 0 {
        return Err("ECS Vulkan validation errors".into());
    }
    Ok(())
}
