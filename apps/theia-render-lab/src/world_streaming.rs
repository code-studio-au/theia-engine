//! One fixed complete world chunk plus one asynchronously replaced complete chunk.
//! One bounded local-file worker and pending GPU set; no camera demand scheduler.
use crate::{
    Result,
    bundle_streaming::{Gpu, Input},
    world_upload::{self, PendingSet, Set},
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread::JoinHandle,
};
use theia_assets::bundle_residency::{StagedBundle, WorldResidency};

struct Loader {
    receiver: Receiver<std::result::Result<Input, String>>,
    worker: Option<JoinHandle<()>>,
    index: usize,
}
impl Drop for Loader {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
struct Pending {
    // Retire submitted copies before releasing their complete encoded reservation.
    uploads: PendingSet,
    staged: StagedBundle,
    index: usize,
}
pub(super) struct Ready {
    pub set: Set,
    staged: StagedBundle,
    index: usize,
}
pub(super) struct Publication {
    staged: StagedBundle,
    index: usize,
}
impl Ready {
    pub fn into_parts(self) -> (Set, Publication) {
        (
            self.set,
            Publication {
                staged: self.staged,
                index: self.index,
            },
        )
    }
}
pub(super) struct Stream {
    pending: Option<Pending>,
    loader: Option<Loader>,
    cpu: WorldResidency,
    paths: [PathBuf; 2],
    active: usize,
    first_images: u32,
    budgets: [u64; 2],
    last_attempt: u64,
    history: VecDeque<serde_json::Value>,
    dropped_history: u64,
    peak_physical: u64,
    peak_images: u64,
    peak_cpu: usize,
    pub publications: u64,
    pub deferrals: u64,
    report_path: Option<PathBuf>,
}
impl Stream {
    pub fn new(
        cpu: WorldResidency,
        paths: [PathBuf; 2],
        first_images: u32,
        budgets: [u64; 2],
        report_path: Option<PathBuf>,
    ) -> Result<Self> {
        let snapshot = cpu.snapshot();
        if snapshot.chunk_count() != 2
            || snapshot.chunk(0).is_none()
            || snapshot.chunk(1).is_none()
            || paths[0].file_stem() == paths[1].file_stem()
        {
            return Err(
                "World stream requires two resident chunks and a different replacement".into(),
            );
        }
        let initial = serde_json::json!({"frame":0,"index":0,"epoch":snapshot.epoch(),
            "fixed_epoch":snapshot.chunk(0).unwrap().epoch()});
        Ok(Self {
            pending: None,
            loader: None,
            cpu,
            paths,
            active: 0,
            first_images,
            budgets,
            last_attempt: 0,
            history: VecDeque::from([initial]),
            dropped_history: 0,
            peak_physical: 0,
            peak_images: 0,
            peak_cpu: 0,
            publications: 0,
            deferrals: 0,
            report_path,
        })
    }
    /// The caller retires its sole frame fence before polling. A returned set
    /// still needs retirement of old graphics descriptors/presentation and a
    /// complete replacement generation before CPU publication is acknowledged.
    pub fn tick(&mut self, frame: u64, active: [u64; 2], gpu: Gpu<'_>) -> Result<Option<Ready>> {
        self.peak_physical = self.peak_physical.max(active[0]);
        self.peak_images = self.peak_images.max(active[1]);
        self.peak_cpu = self.peak_cpu.max(self.cpu.live_bytes());
        if let Some(pending) = &self.pending
            && pending.uploads.ready()?
        {
            let pending = self.pending.take().unwrap();
            let set = pending.uploads.finish()?;
            return Ok(Some(Ready {
                set,
                staged: pending.staged,
                index: pending.index,
            }));
        }
        if let Some(loader) = &self.loader {
            match loader.receiver.try_recv() {
                Ok(result) => {
                    let index = loader.index;
                    self.loader = None;
                    let input = result.map_err(|e| format!("World chunk I/O failed: {e}"))?;
                    let staged = match self.cpu.stage(
                        &input.manifest,
                        theia_assets::content_id(&input.manifest),
                        std::array::from_fn(|i| input.roots[i].as_slice()),
                        &input.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                    ) {
                        Ok(value) => value,
                        Err(error) if error.0 == "live bundle byte budget exhausted" => {
                            self.deferrals = self
                                .deferrals
                                .checked_add(1)
                                .ok_or("World deferral counter exhausted")?;
                            println!(
                                "World chunk 1 deferred by CPU encoded-byte cap; both current complete chunks retained"
                            );
                            return Ok(None);
                        }
                        Err(error) => return Err(error.into()),
                    };
                    self.peak_cpu = self.peak_cpu.max(self.cpu.live_bytes());
                    world_upload::check_pair_limits(
                        self.first_images + input.leaves.len() as u32,
                        &gpu,
                    )?;
                    let remaining = self.budgets[0]
                        .checked_sub(active[0])
                        .ok_or("Active world exceeds physical cap")?;
                    let image_remaining = self.budgets[1]
                        .checked_sub(active[1])
                        .ok_or("Active world exceeds image cap")?;
                    let uploads = match PendingSet::begin(&input, &gpu, remaining, image_remaining)
                    {
                        Ok(value) => value,
                        Err(error) if world_upload::budget_rejection(error.as_ref()) => {
                            self.deferrals = self
                                .deferrals
                                .checked_add(1)
                                .ok_or("World deferral counter exhausted")?;
                            println!(
                                "World chunk 1 deferred by physical/image cap; partial transfers retired, both current complete chunks retained"
                            );
                            return Ok(None);
                        }
                        Err(error) => return Err(error),
                    };
                    self.peak_physical =
                        self.peak_physical.max(active[0] + uploads.physical_bytes());
                    self.peak_images = self.peak_images.max(active[1] + uploads.image_bytes());
                    self.peak_cpu = self.peak_cpu.max(self.cpu.live_bytes());
                    println!(
                        "World chunk 1 submitted at frame {frame}, index {index}: active {} + pending {} physical bytes, active images {} + pending {}; staging {} separately",
                        active[0],
                        uploads.physical_bytes(),
                        active[1],
                        uploads.image_bytes(),
                        uploads.staging_bytes()
                    );
                    self.pending = Some(Pending {
                        uploads,
                        staged,
                        index,
                    });
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("World I/O worker disconnected".into());
                }
            }
        }
        if self.pending.is_none()
            && self.loader.is_none()
            && frame.saturating_sub(self.last_attempt) >= 20
        {
            let index = 1 - self.active;
            let path = self.paths[index].clone();
            let (sender, receiver) = mpsc::sync_channel(1);
            let worker = std::thread::Builder::new()
                .name("theia-world-chunk-io".into())
                .spawn(move || {
                    let result = Input::load(&path).map_err(|e| e.to_string());
                    let _ = sender.try_send(result);
                })?;
            self.loader = Some(Loader {
                receiver,
                worker: Some(worker),
                index,
            });
            self.last_attempt = frame;
            println!("World chunk 1 asynchronous read requested at frame {frame}, index {index}");
        }
        Ok(None)
    }
    pub fn published(&mut self, ready: Publication, frame: u64) -> Result<()> {
        // Renderer moved the complete GPU set into a rebuilt generation and has
        // retired all old graphics users before acknowledging this CPU epoch.
        let snapshot = self.cpu.publish(vec![(1, ready.staged)], &[])?;
        self.active = ready.index;
        self.publications = self
            .publications
            .checked_add(1)
            .ok_or("World publication counter exhausted")?;
        if self.history.len() == 64 {
            self.history.pop_front();
            self.dropped_history = self
                .dropped_history
                .checked_add(1)
                .ok_or("World history counter exhausted")?;
        }
        self.history
            .push_back(serde_json::json!({"frame":frame,"index":self.active,
            "epoch":snapshot.epoch(),"fixed_epoch":snapshot.chunk(0).unwrap().epoch()}));
        println!(
            "World chunk 1 published at frame {frame}, CPU world epoch {}; fixed chunk epoch {} retained",
            snapshot.epoch(),
            snapshot.chunk(0).unwrap().epoch()
        );
        Ok(())
    }
    pub fn report(&self) -> Result<()> {
        let report = serde_json::json!({"schema":1,"publications":self.publications,
            "deferrals":self.deferrals,"active_index":self.active,"cpu_epoch":self.cpu.snapshot().epoch(),
            "fixed_epoch":self.cpu.snapshot().chunk(0).unwrap().epoch(),"history":self.history,
            "dropped_history":self.dropped_history,"peak_complete_set_physical_bytes":self.peak_physical,
            "peak_complete_set_image_bytes":self.peak_images,"peak_cpu_bytes":self.peak_cpu,
            "current_cpu_bytes":self.cpu.live_bytes(),"pending":self.pending.is_some(),
            "loading":self.loader.is_some(),"authority":false,"camera_driven":false});
        println!("World chunk stream proof: {report}");
        if let Some(path) = &self.report_path {
            std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
        }
        Ok(())
    }
}
