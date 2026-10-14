//! One bounded finer-tail request, background I/O, fence-polled GPU readiness.
use crate::{
    Result,
    texture::{self, ResidentTexture, Upload},
};
use ash::vk;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread::JoinHandle,
};
use theia_assets::{texture_mips::TextureMipChain, texture_residency::MipTail};

struct Loader {
    receiver: Receiver<std::result::Result<Vec<u8>, String>>,
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
    upload: Upload,
    metadata: [u32; 4],
}
pub struct Ready {
    pub texture: ResidentTexture,
    pub metadata: [u32; 4],
}
pub struct Gpu<'a> {
    pub device: &'a ash::Device,
    pub memory: &'a vk::PhysicalDeviceMemoryProperties,
    pub family: u32,
    pub queue: vk::Queue,
}
pub struct Refinement {
    // Upload owns transfer/fence/staging/image and retires them on error/drop.
    pending: Option<Pending>,
    loader: Option<Loader>,
    path: PathBuf,
    source: [u8; 32],
    active_first: u32,
    target_budget: usize,
    total_image_budget: u64,
    linear: bool,
    started: bool,
    done: bool,
    pub publications: u32,
    pub deferrals: u32,
    pub admission_failures: u32,
}
impl Refinement {
    pub fn new(
        path: PathBuf,
        source: [u8; 32],
        active_first: u32,
        target_budget: usize,
        total_image_budget: u64,
        linear: bool,
    ) -> Self {
        Self {
            pending: None,
            loader: None,
            path,
            source,
            active_first,
            target_budget,
            total_image_budget,
            linear,
            started: false,
            done: false,
            publications: 0,
            deferrals: 0,
            admission_failures: 0,
        }
    }
    /// The caller retires its sole frame fence before accepting a Ready owner.
    pub fn tick(&mut self, frame: u64, active_bytes: u64, gpu: Gpu<'_>) -> Result<Option<Ready>> {
        if let Some(pending) = &self.pending
            && pending.upload.ready()?
        {
            let pending = self.pending.take().unwrap();
            let texture = pending.upload.finish()?;
            self.done = true;
            return Ok(Some(Ready {
                texture,
                metadata: pending.metadata,
            }));
        }
        if let Some(loader) = &self.loader {
            match loader.receiver.try_recv() {
                Ok(result) => {
                    drop(self.loader.take());
                    let bytes = match result {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            self.done = true;
                            self.admission_failures += 1;
                            println!(
                                "Mip refinement source rejected at frame {frame}: {error}; protected coarse image retained"
                            );
                            return Ok(None);
                        }
                    };
                    let chain = TextureMipChain::decode(
                        &bytes,
                        Some(self.source),
                        theia_assets::texture::DEFAULT_BUDGET,
                    )?;
                    let tail = MipTail::fit(&chain, self.target_budget)?;
                    if tail.first_level() >= self.active_first {
                        return Err("Mip refinement must add finer source levels".into());
                    }
                    let remaining = self
                        .total_image_budget
                        .checked_sub(active_bytes)
                        .ok_or("Active mip image exceeds coexistence budget")?;
                    let upload = match Upload::begin_tail(
                        gpu.device,
                        gpu.memory,
                        &tail,
                        gpu.family,
                        gpu.queue,
                        remaining,
                        self.linear,
                    ) {
                        Ok(upload) => upload,
                        Err(error)
                            if error.to_string()
                                == "Texture image allocation exceeds physical byte budget" =>
                        {
                            self.done = true;
                            self.deferrals += 1;
                            println!(
                                "Mip refinement deferred at frame {frame}: coexistence image budget {}; protected coarse image retained",
                                self.total_image_budget
                            );
                            return Ok(None);
                        }
                        Err(error) => return Err(error),
                    };
                    let extent = tail.source_extent();
                    println!(
                        "Mip refinement transfer submitted at frame {frame}: source first {}, active {active_bytes} + pending {} physical image bytes <= {}; staging {} bytes separately; no publication before fence readiness",
                        tail.first_level(),
                        upload.resident_bytes(),
                        self.total_image_budget,
                        upload.staging_bytes()
                    );
                    self.pending = Some(Pending {
                        upload,
                        metadata: [
                            extent[0],
                            extent[1],
                            tail.first_level(),
                            tail.source_level_count(),
                        ],
                    });
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("Mip refinement I/O worker disconnected".into());
                }
            }
        }
        if !self.started && frame >= 20 {
            let path = self.path.clone();
            let (sender, receiver) = mpsc::sync_channel(1);
            let worker = std::thread::Builder::new()
                .name("theia-mip-io".into())
                .spawn(move || {
                    let result = texture::load_mips(&path).map_err(|error| error.to_string());
                    let _ = sender.try_send(result);
                })?;
            self.loader = Some(Loader {
                receiver,
                worker: Some(worker),
            });
            self.started = true;
            println!(
                "Mip refinement I/O queued at frame {frame}: one immutable full-source read, coarse image remains active"
            );
        }
        Ok(None)
    }
    pub fn published(&mut self, first: u32, frame: u64) {
        self.publications += 1;
        println!(
            "Mip refinement published at frame {frame}: source first {first}; prior graphics generation retired before old image/sampler owners"
        );
    }
    pub fn finished(&self) -> bool {
        self.done
    }
    pub fn check_complete(&self, frame: u64) -> Result<()> {
        println!(
            "Mip refinement qualification: {} publications, {} budget deferrals, {} source rejections",
            self.publications, self.deferrals, self.admission_failures
        );
        if frame >= 40
            && (!self.done || self.publications + self.deferrals + self.admission_failures != 1)
        {
            return Err("Mip refinement qualification did not complete its bounded request".into());
        }
        Ok(())
    }
}
