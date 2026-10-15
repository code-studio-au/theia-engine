//! Bounded camera demand scheduler. Four protected coarse regions, one file/GPU
//! request, actual committed fine residency and explicit publication acknowledgement.
use crate::{
    Result, benchmark,
    bundle_streaming::Gpu,
    world_refinement::{Profile, Ready, Reload},
};
use std::{collections::VecDeque, path::PathBuf};
use theia_assets::{world_coverage::ProtectedWorld, world_demand::Policy};
const HISTORY: usize = 64;
pub(super) enum Change {
    Evict(usize),
    Refine(Box<Ready>),
}
pub(super) struct Tick {
    pub event: Option<&'static str>,
    pub change: Option<Change>,
}
pub(super) struct Stream {
    request: Option<Reload>,
    profile: Profile,
    sources: [PathBuf; 4],
    budgets: [u64; 2],
    policy: Policy,
    config: benchmark::Config,
    blocked: [Option<u64>; 4],
    history: VecDeque<serde_json::Value>,
    dropped: u64,
    report: Option<PathBuf>,
    plan: serde_json::Value,
    pub publications: u64,
    pub evictions: u64,
    cancellations: u64,
    ready_cancellations: u64,
    deferrals_cpu: u64,
    deferrals_gpu: u64,
    requests: u64,
    uploads: u64,
    peak_gpu: u64,
    pub fixed_gpu_bytes: u64,
    peak_images: u64,
    peak_cpu: usize,
}
impl Stream {
    pub fn new(
        sources: [PathBuf; 4],
        budgets: [u64; 2],
        route: (u64, usize, Option<PathBuf>),
        gpu: Gpu<'_>,
    ) -> Result<Self> {
        let (period, slots, report) = route;
        if !(1..=4).contains(&slots) {
            return Err("Camera demand fine slots must be 1..4".into());
        }
        let config = benchmark::Config {
            players: 0,
            overlay: false,
            frames: period,
            warmup: 0,
            seconds: 60.,
            report: "unused.json".into(),
        };
        config.validate()?;
        Ok(Self {
            request: None,
            profile: Profile::new(gpu),
            sources,
            budgets,
            policy: Policy {
                load_radius: 20.,
                retain_radius: 35.,
                slots,
            },
            config,
            blocked: [None; 4],
            history: VecDeque::new(),
            dropped: 0,
            report,
            plan: serde_json::Value::Null,
            publications: 0,
            evictions: 0,
            cancellations: 0,
            ready_cancellations: 0,
            deferrals_cpu: 0,
            deferrals_gpu: 0,
            requests: 0,
            uploads: 0,
            peak_gpu: 0,
            fixed_gpu_bytes: 0,
            peak_images: 0,
            peak_cpu: 0,
        })
    }
    pub fn eye(&self, frame: u64) -> [f32; 3] {
        benchmark::rig(
            self.config.time(frame % self.config.frames),
            self.config.seconds,
        )
        .0
    }
    pub fn rows(&self, frame: u64, width: u32, height: u32) -> Result<[[f32; 4]; 4]> {
        Ok(benchmark::push(&self.config, frame % self.config.frames, width, height)?.1)
    }
    pub fn proxy_push(
        &self,
        frame: u64,
        width: u32,
        height: u32,
        players: u32,
        mask: u32,
    ) -> Result<[u8; 96]> {
        let mut config = self.config.clone();
        config.players = players;
        let mut push = benchmark::push(&config, frame % config.frames, width, height)?.0;
        push[88..92].copy_from_slice(&1u32.to_ne_bytes());
        push[92..96].copy_from_slice(&mask.to_ne_bytes());
        Ok(push)
    }
    pub fn pending(&self) -> Result<Option<serde_json::Value>> {
        self.request.as_ref().map(Reload::stats).transpose()
    }
    pub fn tick(&mut self, frame: u64, cpu: &mut ProtectedWorld, active: [u64; 2]) -> Result<Tick> {
        let eye = self.eye(frame);
        let plan = cpu.plan([eye[0], eye[1]], self.policy)?;
        self.plan = serde_json::json!({"eye":eye,"desired":plan.desired,"loads":plan.loads,"removals":plan.removals,"deferred":plan.deferred});
        let epoch = cpu.snapshot().world().epoch();
        if let Some(request) = &self.request {
            self.plan["candidate"] = request.region().into();
        }
        if let Some(request) = &self.request
            && !plan.desired.contains(&(request.region() as u64))
        {
            let state = request.stats()?;
            self.ready_cancellations += u64::from(state["pending_ready"] == true);
            drop(self.request.take());
            self.cancellations += 1;
            self.plan["cancelled"] = state;
            return Ok(Tick {
                event: Some("cancelled"),
                change: None,
            });
        }
        if let Some(&slot) = plan.removals.first() {
            return Ok(Tick {
                event: None,
                change: Some(Change::Evict(slot as usize)),
            });
        }
        if let Some(request) = &mut self.request {
            let (changed, ready) = request.tick(frame, cpu, active)?;
            if let Some(ready) = ready {
                return Ok(Tick {
                    event: None,
                    change: Some(Change::Refine(Box::new(ready))),
                });
            }
            if let Some(cpu_cap) = request.deferred() {
                self.blocked[request.region()] = Some(epoch);
                if cpu_cap {
                    self.deferrals_cpu += 1;
                } else {
                    self.deferrals_gpu += 1;
                }
                drop(self.request.take());
                return Ok(Tick {
                    event: Some(if cpu_cap {
                        "deferred_cpu"
                    } else {
                        "deferred_gpu"
                    }),
                    change: None,
                });
            }
            if changed {
                self.uploads += 1;
            }
            return Ok(Tick {
                event: changed.then_some("pending"),
                change: None,
            });
        }
        if let Some(&slot) = plan
            .loads
            .iter()
            .find(|&&id| self.blocked[id as usize] != Some(epoch))
        {
            let slot = slot as usize;
            let mut request = Reload::request(
                slot,
                self.sources[slot].clone(),
                self.budgets,
                self.profile.gpu(),
                frame,
            );
            request.tick(frame, cpu, active)?;
            self.plan["candidate"] = slot.into();
            self.request = Some(request);
            self.requests += 1;
            return Ok(Tick {
                event: Some("requested"),
                change: None,
            });
        }
        Ok(Tick {
            event: None,
            change: None,
        })
    }
    pub fn published(&mut self, slot: usize) -> Result<()> {
        let request = self
            .request
            .as_ref()
            .ok_or("Missing camera fine request acknowledgement")?;
        if request.region() != slot {
            return Err("Camera fine request region mismatch".into());
        }
        drop(self.request.take());
        self.publications += 1;
        Ok(())
    }
    pub fn evicted(&mut self) {
        self.evictions += 1;
    }
    pub fn record(&mut self, mut row: serde_json::Value, kind: &str) -> Result<()> {
        let pending = self.pending()?;
        self.peak_gpu = self.peak_gpu.max(
            row["physical_bytes"].as_u64().unwrap()
                + pending
                    .as_ref()
                    .and_then(|p| p["pending_gpu_bytes"].as_u64())
                    .unwrap_or(0),
        );
        self.peak_images = self.peak_images.max(
            row["image_bytes"].as_u64().unwrap()
                + pending
                    .as_ref()
                    .and_then(|p| p["pending_image_bytes"].as_u64())
                    .unwrap_or(0),
        );
        self.peak_cpu = self
            .peak_cpu
            .max(row["cpu_unique_bytes"].as_u64().unwrap() as usize);
        row["kind"] = kind.into();
        row["plan"] = self.plan.clone();
        row["pending"] = serde_json::to_value(pending)?;
        if self.history.len() == HISTORY {
            self.history.pop_front();
            self.dropped += 1;
        }
        self.history.push_back(row);
        Ok(())
    }
    pub fn report(&self, final_state: serde_json::Value, completed: bool) -> Result<()> {
        if let Some(path) = &self.report {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                path,
                serde_json::to_vec_pretty(
                    &serde_json::json!({"completed":completed,"history":self.history,
                "dropped_history":self.dropped,"final":final_state,"last_plan":self.plan,"pending":self.pending()?,"route_frames":self.config.frames,
                "fine_slots":self.policy.slots,"load_radius":20,"retain_radius":35,"publications":self.publications,
                "evictions":self.evictions,"requests":self.requests,"uploads":self.uploads,"cancellations":self.cancellations,"ready_cancellations":self.ready_cancellations,
                "cpu_deferrals":self.deferrals_cpu,"gpu_deferrals":self.deferrals_gpu,"peak_gpu_bytes":self.peak_gpu,"fixed_gpu_bytes":self.fixed_gpu_bytes,"peak_total_gpu_bytes":self.peak_gpu+self.fixed_gpu_bytes,
                "peak_image_bytes":self.peak_images,"peak_cpu_bytes":self.peak_cpu,"authority":false}),
                )?,
            )?;
        }
        Ok(())
    }
}
