//! Small local authority demo. Input requests direction/attack; private server
//! World decides position/health. Bounded clock driver is not a production scheduler.
use crate::{Result, authority};
use std::time::Duration;
use theia_protocol::replica::{Replica, Snapshot, Source};
use theia_sim::{
    ActorState, AttackInput, AttackResult, MoveInput, Rect, Rules, Scene, Session, World, WorldId,
};
const TICK: Duration = Duration::from_millis(50);
pub struct Demo {
    world: World,
    session: Session,
    ids: [WorldId; 2],
    sequence: u64,
    keys: [bool; 8],
    attack: bool,
    script: bool,
    replica: Replica,
    source: Source,
    previous: Vec<ActorState>,
    now: Duration,
    next_tick: Duration,
    last_tick: Duration,
    snap: bool,
    pub skipped: u64,
    pub blocked: u64,
    pub hits: u64,
}
impl Demo {
    pub fn new(script: bool) -> Result<Self> {
        let scene = Scene::new(
            Rect {
                min: [-4000; 2],
                max: [4000; 2],
            },
            vec![Rect {
                min: [-100; 2],
                max: [100; 2],
            }],
        )?;
        let mut world = World::new(scene, Rules::default())?;
        let ids = [
            WorldId::new((2u64 << 32) + 10)?,
            WorldId::new((2u64 << 32) + 20)?,
        ];
        let session = world.spawn_at_boundary(ids[0], [-1000, -1000])?;
        world.spawn_at_boundary(ids[1], [1000, -1000])?;
        let mut replica = Replica::new(2)?;
        let source = replica.connect()?;
        let previous = world.snapshot();
        replica.publish(
            source,
            Snapshot {
                tick: 0,
                applied_move: None,
                actors: previous.clone(),
            },
        )?;
        Ok(Self {
            world,
            session,
            ids,
            sequence: 0,
            keys: [false; 8],
            attack: false,
            script,
            replica,
            source,
            previous,
            now: Duration::ZERO,
            next_tick: TICK,
            last_tick: Duration::ZERO,
            snap: true,
            skipped: 0,
            blocked: 0,
            hits: 0,
        })
    }
    /// 0..3 WASD left/right/up/down; 4..7 arrow aliases. Track each physical
    /// binding independently so releasing one alias cannot clear another held key.
    pub fn direction(&mut self, index: usize, pressed: bool) {
        if let Some(key) = self.keys.get_mut(index) {
            *key = pressed;
        }
    }
    /// Coalesce at most one pending attack edge per tick; no client damage parameter.
    pub fn attack(&mut self) {
        self.attack = true;
    }
    pub fn release_input(&mut self) {
        self.keys = [false; 8];
        self.attack = false;
    }
    pub fn tick(&self) -> u64 {
        self.world.tick()
    }
    pub fn next_tick(&self) -> Duration {
        self.next_tick
    }
    fn step(&mut self) -> Result<()> {
        let tick = self
            .world
            .tick()
            .checked_add(1)
            .ok_or("Authority tick exhausted")?;
        let (axes, attack) = if self.script {
            let axes = match tick {
                1..=4 => [1000, 0],
                17..=21 => [0, 1000],
                22..=25 => [1000, 0],
                _ => [0, 0],
            };
            (axes, matches!(tick, 4 | 8 | 12 | 16))
        } else {
            (
                [
                    (i16::from(self.keys[1] || self.keys[5])
                        - i16::from(self.keys[0] || self.keys[4]))
                        * 1000,
                    (i16::from(self.keys[3] || self.keys[7])
                        - i16::from(self.keys[2] || self.keys[6]))
                        * 1000,
                ],
                self.attack,
            )
        };
        let movement = self.sequence;
        self.sequence
            .checked_add(if attack { 2 } else { 1 })
            .ok_or("Authority input sequence exhausted")?;
        self.world.submit(
            self.session,
            MoveInput {
                sequence: movement,
                axes,
            },
        )?;
        self.sequence += 1;
        if attack {
            self.world.submit_attack(
                self.session,
                AttackInput {
                    sequence: self.sequence,
                    target: self.ids[1],
                },
            )?;
            self.sequence += 1;
        }
        self.attack = false;
        self.previous = self.world.snapshot();
        let frame = self.world.step()?;
        self.blocked += frame.blocked.len() as u64;
        self.hits += frame
            .attacks
            .iter()
            .filter(|v| matches!(v.result, AttackResult::Hit { .. }))
            .count() as u64;
        for outcome in &frame.attacks {
            println!(
                "Local authority tick {} attack: {:?}",
                frame.tick, outcome.result
            );
        }
        self.replica.publish(
            self.source,
            Snapshot {
                tick: frame.tick,
                applied_move: Some(movement),
                actors: self.world.snapshot(),
            },
        )?;
        self.snap = false;
        Ok(())
    }
    /// Run at most four fixed steps per wake, skip remaining wall intervals visibly.
    /// Call from an independent event-loop timer even if rendering is occluded.
    pub fn advance(&mut self, now: Duration) -> Result<()> {
        if now < self.now {
            return Err("Local authority clock moved backwards".into());
        }
        self.now = now;
        let mut steps = 0;
        while now >= self.next_tick && steps < 4 {
            let next = self
                .next_tick
                .checked_add(TICK)
                .ok_or("Local authority clock exhausted")?;
            self.step()?;
            self.last_tick = self.next_tick;
            self.next_tick = next;
            steps += 1;
        }
        if now >= self.next_tick {
            let skipped = u64::try_from((now - self.next_tick).as_millis() / 50 + 1)?;
            self.next_tick = now
                .checked_add(TICK)
                .ok_or("Local authority clock exhausted")?;
            self.last_tick = now;
            self.skipped = self
                .skipped
                .checked_add(skipped)
                .ok_or("Skipped interval count exhausted")?;
            self.snap = true;
            println!(
                "Local authority: skipped {skipped} wall intervals after a bounded four-step catchup"
            );
        }
        Ok(())
    }
    pub fn push(&self) -> Result<[u8; 64]> {
        let alpha = if self.snap {
            1.0
        } else {
            ((self.now - self.last_tick).as_secs_f64() / TICK.as_secs_f64()).clamp(0.0, 1.0) as f32
        };
        authority::pack(
            &self.previous,
            &self
                .replica
                .latest()
                .ok_or("Missing local authority snapshot")?
                .actors,
            alpha,
        )
    }
    /// Stable end-of-script readonly snapshot oracle for late GPU capture.
    pub fn reference(&self) -> Result<serde_json::Value> {
        if !self.script || self.tick() < 26 {
            return Err("Script reference needs completed 26-tick fixture".into());
        }
        let actors: Vec<_> = self.replica.latest().ok_or("No script snapshot")?.actors.iter().map(|v| {
            let id = v.id.value(); let folded = (id as u32)^((id>>32) as u32);
            let base = if folded & 2 != 0 { [0.1,0.8,0.2] } else { [0.8,0.2,0.1] };
            let color = if v.health == 0 { [0.15;3] } else { base.map(|x| x*f64::from(v.health)/100.0) };
            serde_json::json!({"id":id,"position_mm":v.position_mm,"health":v.health,"color":color,"half_extent_mm":250})
        }).collect();
        Ok(
            serde_json::json!({"actors":actors,"obstacle":{"position_mm":[0,0],"half_extent_mm":100,"color":[0.3,0.3,0.3]},
            "background":[0.025,0.07,0.12],"world_clip_half_extent_mm":4000,"tick":self.tick(),"push_bytes":self.push()?.to_vec(),
            "source":"completed local server intent script; stationary after tick 26"}),
        )
    }
    pub fn verify(&self) -> Result<()> {
        if self.script
            && (self.tick() < 26
                || self.hits != 4
                || self.blocked < 4
                || self.world.actor(self.ids[1]).unwrap().health != 0)
        {
            return Err(
                "Script needs 26 fixed ticks, four hits/death and four static collision blocks"
                    .into(),
            );
        }
        println!(
            "Local authority demo: {} fixed ticks, {} hits, {} blocks, {} skipped wall intervals; target health {}",
            self.tick(),
            self.hits,
            self.blocked,
            self.skipped,
            self.world.actor(self.ids[1]).unwrap().health
        );
        Ok(())
    }
}
pub fn script_reference() -> Result<serde_json::Value> {
    let mut demo = Demo::new(true)?;
    for tick in 1..=26 {
        demo.advance(TICK * tick)?;
    }
    demo.verify()?;
    demo.reference()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scripted_intents_qualify_combat_death_and_static_collision_without_gpu_truth() {
        let mut demo = Demo::new(true).unwrap();
        for tick in 1..=40 {
            demo.advance(TICK * tick).unwrap();
        }
        demo.verify().unwrap();
        assert_eq!(demo.hits, 4);
        assert_eq!(demo.blocked, 4);
        assert_eq!(demo.skipped, 0);
        assert_eq!(
            demo.world.actor(demo.ids[0]).unwrap().position_mm,
            [-400, -250]
        );
        assert_eq!(
            u32::from_ne_bytes(demo.push().unwrap()[52..56].try_into().unwrap()),
            0
        );
    }
    #[test]
    fn held_release_opposing_keys_and_attack_coalescing_follow_fixed_ticks() {
        let mut demo = Demo::new(false).unwrap();
        demo.direction(1, true);
        demo.advance(TICK).unwrap();
        assert_eq!(
            demo.world.actor(demo.ids[0]).unwrap().position_mm,
            [-850, -1000]
        );
        demo.direction(0, true);
        demo.advance(TICK * 2).unwrap();
        assert_eq!(
            demo.world.actor(demo.ids[0]).unwrap().position_mm,
            [-850, -1000]
        );
        demo.release_input();
        demo.advance(TICK * 3).unwrap();
        assert_eq!(
            demo.world.actor(demo.ids[0]).unwrap().position_mm,
            [-850, -1000]
        );
        for tick in 4..=6 {
            demo.direction(1, true);
            demo.advance(TICK * tick).unwrap();
        }
        demo.release_input();
        demo.attack();
        demo.attack();
        demo.advance(TICK * 7).unwrap();
        assert_eq!(demo.hits, 1);
        demo.advance(TICK * 8).unwrap();
        assert_eq!(demo.hits, 1);
    }
    #[test]
    fn physical_alias_release_retains_other_held_binding_and_focus_reset_clears_all() {
        let mut demo = Demo::new(false).unwrap();
        demo.direction(1, true);
        demo.direction(5, true);
        demo.direction(1, false);
        demo.advance(TICK).unwrap();
        assert_eq!(
            demo.world.actor(demo.ids[0]).unwrap().position_mm,
            [-850, -1000]
        );
        demo.release_input();
        demo.advance(TICK * 2).unwrap();
        assert_eq!(
            demo.world.actor(demo.ids[0]).unwrap().position_mm,
            [-850, -1000]
        );
    }
    #[test]
    fn catchup_budget_clock_regression_and_gpu_phase_do_not_predict_authority() {
        let mut demo = Demo::new(false).unwrap();
        demo.direction(1, true);
        demo.advance(Duration::from_secs(1)).unwrap();
        assert_eq!(demo.tick(), 4);
        assert_eq!(demo.skipped, 16);
        assert_eq!(
            demo.world.actor(demo.ids[0]).unwrap().position_mm,
            [-400, -1000]
        );
        let before = demo.world.snapshot();
        assert!(demo.advance(Duration::ZERO).is_err());
        assert_eq!(demo.world.snapshot(), before);
        assert_eq!(
            f32::from_ne_bytes(demo.push().unwrap()[56..60].try_into().unwrap()),
            1.0
        );
        demo.release_input();
        demo.advance(Duration::from_millis(1050)).unwrap();
        demo.advance(Duration::from_millis(1075)).unwrap();
        assert_eq!(
            f32::from_ne_bytes(demo.push().unwrap()[56..60].try_into().unwrap()),
            0.5
        );
    }
}
