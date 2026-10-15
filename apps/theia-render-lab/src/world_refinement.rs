//! One bounded immutable-file worker and GPU pending set per fine request.
//! The scripted constructor uses region 2 at frames 100/110; camera requests supply their region/timing.
use crate::{
    Result,
    bundle_streaming::{Gpu, Input},
    world_upload::{self, PendingSet, Set},
};
use ash::vk;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread::JoinHandle,
};
use theia_assets::{bundle_residency::StagedBundle, world_coverage::ProtectedWorld};
struct Loader {
    receiver: Receiver<std::result::Result<Input, String>>,
    worker: Option<JoinHandle<()>>,
}
impl Drop for Loader {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
struct Pending {
    // Retire submitted copies before dropping the complete encoded reservation.
    uploads: PendingSet,
    staged: StagedBundle,
}
pub(super) struct Ready {
    pub region: usize,
    pub set: Set,
    pub staged: StagedBundle,
}
#[derive(Clone)]
pub(super) struct Profile {
    device: ash::Device,
    memory: vk::PhysicalDeviceMemoryProperties,
    limits: vk::PhysicalDeviceLimits,
    image_limits: vk::ImageFormatProperties,
    family: u32,
    queue: vk::Queue,
}
impl Profile {
    pub fn new(gpu: Gpu<'_>) -> Self {
        Self {
            device: gpu.device.clone(),
            memory: *gpu.memory,
            limits: gpu.limits,
            image_limits: gpu.image_limits,
            family: gpu.family,
            queue: gpu.queue,
        }
    }
    pub fn gpu(&self) -> Gpu<'_> {
        Gpu {
            device: &self.device,
            memory: &self.memory,
            limits: self.limits,
            image_limits: self.image_limits,
            family: self.family,
            queue: self.queue,
        }
    }
}
pub(super) struct Reload {
    pending: Option<Pending>,
    loader: Option<Loader>,
    profile: Profile,
    path: PathBuf,
    region: usize,
    request_frame: u64,
    publication_frame: u64,
    budgets: [u64; 2],
    started: bool,
    pub published: bool,
    cpu_deferrals: u64,
    gpu_deferrals: u64,
}
impl Reload {
    pub fn new(path: PathBuf, budgets: [u64; 2], gpu: Gpu<'_>) -> Self {
        Self {
            pending: None,
            loader: None,
            profile: Profile::new(gpu),
            path,
            region: 2,
            request_frame: 100,
            publication_frame: 110,
            budgets,
            started: false,
            published: false,
            cpu_deferrals: 0,
            gpu_deferrals: 0,
        }
    }
    pub fn request(
        region: usize,
        path: PathBuf,
        budgets: [u64; 2],
        gpu: Gpu<'_>,
        frame: u64,
    ) -> Self {
        let mut request = Self::new(path, budgets, gpu);
        request.region = region;
        request.request_frame = frame;
        request.publication_frame = frame;
        request
    }
    pub fn region(&self) -> usize {
        self.region
    }
    pub fn deferred(&self) -> Option<bool> {
        if self.cpu_deferrals > 0 {
            Some(true)
        } else if self.gpu_deferrals > 0 {
            Some(false)
        } else {
            None
        }
    }
    /// Caller has retired its sole frame fence. Returned complete sets still need
    /// old presentation retirement and a complete next descriptor generation.
    pub fn tick(
        &mut self,
        frame: u64,
        cpu: &mut ProtectedWorld,
        active: [u64; 2],
    ) -> Result<(bool, Option<Ready>)> {
        if let Some(pending) = &self.pending {
            if frame >= self.publication_frame && pending.uploads.ready()? {
                let pending = self.pending.take().unwrap();
                return Ok((
                    true,
                    Some(Ready {
                        region: self.region,
                        set: pending.uploads.finish()?,
                        staged: pending.staged,
                    }),
                ));
            }
            return Ok((false, None));
        }
        if let Some(loader) = &self.loader {
            let input = match loader.receiver.try_recv() {
                Ok(result) => {
                    result.map_err(|e| format!("Protected fine reload source failed: {e}"))?
                }
                Err(mpsc::TryRecvError::Empty) => return Ok((false, None)),
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("Protected fine reload worker disconnected".into());
                }
            };
            drop(self.loader.take());
            let staged = match cpu.stage(
                &input.manifest,
                theia_assets::content_id(&input.manifest),
                std::array::from_fn(|i| input.roots[i].as_slice()),
                &input.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            ) {
                Ok(value) => value,
                Err(error) if error.0 == "live bundle byte budget exhausted" => {
                    self.cpu_deferrals += 1;
                    return Ok((true, None));
                }
                Err(error) => return Err(error.into()),
            };
            let remaining = self.budgets[0]
                .checked_sub(active[0])
                .ok_or("Protected reload physical ledger exceeds cap")?;
            let images = self.budgets[1]
                .checked_sub(active[1])
                .ok_or("Protected reload image ledger exceeds cap")?;
            let gpu = self.profile.gpu();
            match PendingSet::begin(&input, &gpu, remaining, images) {
                Ok(uploads) => self.pending = Some(Pending { uploads, staged }),
                Err(error) if world_upload::budget_rejection(error.as_ref()) => {
                    drop(staged);
                    self.gpu_deferrals += 1;
                }
                Err(error) => return Err(error),
            }
            return Ok((true, None));
        }
        if frame >= self.request_frame && !self.started {
            let path = self.path.clone();
            let (sender, receiver) = mpsc::sync_channel(1);
            let worker = std::thread::Builder::new()
                .name("theia-fine-reload".into())
                .spawn(move || {
                    let result = Input::load(&path).map_err(|e| e.to_string());
                    let _ = sender.send(result);
                })?;
            self.loader = Some(Loader {
                receiver,
                worker: Some(worker),
            });
            self.started = true;
            return Ok((true, None));
        }
        Ok((false, None))
    }
    pub fn stats(&self) -> Result<serde_json::Value> {
        Ok(
            serde_json::json!({"started":self.started,"loading":self.loader.is_some(),"pending":self.pending.is_some(),
            "published":self.published,"cpu_deferrals":self.cpu_deferrals,"gpu_deferrals":self.gpu_deferrals,
            "pending_gpu_bytes":self.pending.as_ref().map_or(0,|p|p.uploads.physical_bytes()),
            "pending_image_bytes":self.pending.as_ref().map_or(0,|p|p.uploads.image_bytes()),
            "pending_ready":self.pending.as_ref().map(|p|p.uploads.ready()).transpose()?,
            "region":self.region,"earliest_request_frame":self.request_frame,"earliest_publication_frame":self.publication_frame}),
        )
    }
}
