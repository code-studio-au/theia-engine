//! Bounded two-page qualification with one I/O worker and one GPU upload in flight.
use crate::{
    Result,
    asset::{self, Upload},
    indirect::IndirectWork,
};
use ash::vk;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread::JoinHandle,
};
use theia_assets::{
    content_id,
    residency::{Manifest, PageId, Residency, Snapshot, StagedPage},
};

pub struct Stream {
    // Pending upload drops (and retires its fence) before its CPU reservation.
    pending: Option<(Upload, StagedPage, PageId)>,
    loader: Option<Loader>,
    manifest: Manifest,
    directory: PathBuf,
    pages: [PageId; 2],
    active: PageId,
    store: Residency,
    frame_reader: Option<Snapshot>,
    budget: u64,
    last_attempt: u64,
    pub publications: u64,
    pub rejections: u64,
}
struct Loader {
    receiver: Receiver<std::result::Result<Vec<u8>, String>>,
    worker: Option<JoinHandle<()>>,
    id: PageId,
}
impl Drop for Loader {
    fn drop(&mut self) {
        // Worker performs one bounded local file read and a nonblocking channel send.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn page_path(directory: &Path, id: PageId) -> PathBuf {
    let hex: String = id.iter().map(|byte| format!("{byte:02x}")).collect();
    directory.join(format!("{hex}.theia-mesh"))
}
impl Stream {
    pub fn new(path: &Path, budget: u64) -> Result<(Self, Vec<u8>)> {
        let mut bytes = Vec::new();
        File::open(path)?
            .take(24 + 4096 * 40 + 1)
            .read_to_end(&mut bytes)?;
        let id = content_id(&bytes);
        let hex: String = id.iter().map(|byte| format!("{byte:02x}")).collect();
        if path.file_stem().and_then(|name| name.to_str()) != Some(hex.as_str()) {
            return Err("Cell filename differs from content address".into());
        }
        let manifest = Manifest::decode(&bytes, id)?;
        let directory = path
            .parent()
            .ok_or("Cell has no page directory")?
            .to_path_buf();
        let mut entries: Vec<_> = manifest
            .pages()
            .iter()
            .map(|(id, size)| (*id, *size))
            .collect();
        entries.sort_by_key(|(_, size)| *size);
        if entries.len() != 2 || entries[0].1 >= entries[1].1 {
            return Err("Cell qualification requires two pages with distinct encoded sizes".into());
        }
        let pages = [entries[0].0, entries[1].0];
        // Reuse source-link/bounds checks for this fixture's protected coarse baseline.
        let coarse = asset::select(
            &page_path(&directory, pages[1]),
            Some(&page_path(&directory, pages[0])),
            entries[0].1,
        )?;
        let mut store = Residency::new(theia_assets::DEFAULT_BUDGET * 2)?;
        let staged = store.stage(&manifest, pages[0], &coarse)?;
        store.publish(Some(staged), &[])?;
        println!(
            "Cell {} loaded: protected coarse baseline; physical geometry budget {budget} bytes (staging tracked separately)",
            manifest.cell()
        );
        Ok((
            Self {
                pending: None,
                loader: None,
                manifest,
                directory,
                pages,
                active: pages[0],
                store,
                frame_reader: None,
                budget,
                last_attempt: 0,
                publications: 0,
                rejections: 0,
            },
            coarse,
        ))
    }
    pub fn check_initial(&self, work: &IndirectWork) -> Result<()> {
        if work.resident_bytes() > self.budget {
            return Err("Initial coarse page exceeds physical geometry budget".into());
        }
        Ok(())
    }
    /// Called after the sole frame fence retires, before any new recording.
    pub fn tick(
        &mut self,
        frame: u64,
        work: &mut IndirectWork,
        device: &ash::Device,
        memory: &vk::PhysicalDeviceMemoryProperties,
        queue: (u32, vk::Queue),
        storage_limit: u32,
    ) -> Result<()> {
        self.frame_reader = None;
        let ready = self
            .pending
            .as_ref()
            .map(|(upload, _, _)| upload.ready())
            .transpose()?
            .unwrap_or(false);
        if ready {
            let (upload, staged, id) = self.pending.take().unwrap();
            let new = upload.finish()?;
            self.store.publish(Some(staged), &[self.active])?;
            let retired = work.replace_asset(new);
            println!(
                "Cell page published at frame {frame}, epoch {}; old {} allocation bytes retired after frame fence; live resident {} bytes",
                self.store.snapshot().epoch(),
                retired.allocated_bytes,
                work.resident_bytes()
            );
            drop(retired);
            self.active = id;
            self.publications += 1;
        }
        if let Some(loader) = &self.loader {
            match loader.receiver.try_recv() {
                Ok(result) => {
                    let id = loader.id;
                    let bytes = result.map_err(|message| format!("Cell I/O failed: {message}"))?;
                    self.loader = None;
                    if bytes.len() > storage_limit as usize {
                        return Err("Cell page exceeds GPU storage-buffer range".into());
                    }
                    let staged = self.store.stage(&self.manifest, id, &bytes)?;
                    let remaining = self
                        .budget
                        .checked_sub(work.resident_bytes())
                        .ok_or("Resident budget accounting underflow")?;
                    match Upload::begin(device, memory, &bytes, queue.0, queue.1, remaining) {
                        Ok(upload) => {
                            println!(
                                "Cell upload queued: active {} + pending {} physical geometry bytes <= {}; staging {} bytes separately; CPU live encoded {} bytes",
                                work.resident_bytes(),
                                upload.resident_bytes(),
                                self.budget,
                                upload.staging_bytes(),
                                self.store.live_bytes()
                            );
                            self.pending = Some((upload, staged, id));
                        }
                        Err(error)
                            if error
                                .downcast_ref::<asset::AllocationBudgetExceeded>()
                                .is_some() =>
                        {
                            self.rejections += 1;
                            println!(
                                "Cell replacement deferred by physical budget; protected current page remains resident"
                            );
                        }
                        Err(error) => return Err(error),
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("Cell I/O worker disconnected".into());
                }
            }
        }
        if self.loader.is_none() && self.pending.is_none() && frame >= self.last_attempt + 20 {
            let id = if self.active == self.pages[0] {
                self.pages[1]
            } else {
                self.pages[0]
            };
            let path = page_path(&self.directory, id);
            let (sender, receiver) = mpsc::sync_channel(1);
            let worker = std::thread::Builder::new()
                .name("theia-page-io".into())
                .spawn(move || {
                    let result = asset::load(&path).map_err(|error| error.to_string());
                    let _ = sender.try_send(result);
                })?;
            self.loader = Some(Loader {
                receiver,
                worker: Some(worker),
                id,
            });
            self.last_attempt = frame;
            println!("Cell asynchronous read requested at frame {frame}");
        }
        Ok(())
    }
    pub fn submitted(&mut self) {
        self.frame_reader = Some(self.store.snapshot());
    }
}
