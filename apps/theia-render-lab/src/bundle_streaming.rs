//! Two complete indexed bundles, one bounded I/O worker and one pending GPU set.
use crate::{Result, asset, texture};
use ash::vk;
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread::JoinHandle,
};
use theia_assets::{
    bundle::{BYTES, Bundle, BundleProfile},
    bundle_residency::{Residency, StagedBundle},
    content_id,
};

struct Input {
    manifest: Vec<u8>,
    roots: [Vec<u8>; 4],
    leaves: Vec<Vec<u8>>,
}
fn hex(id: [u8; 32]) -> String {
    id.iter().map(|v| format!("{v:02x}")).collect()
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("Bundle stream input exceeds encoded limit".into());
    }
    Ok(bytes)
}
impl Input {
    fn load(path: &Path) -> Result<Self> {
        let manifest = read(path, BYTES)?;
        if path.file_stem().and_then(|v| v.to_str()) != Some(&hex(content_id(&manifest))) {
            return Err("Bundle stream manifest filename hash mismatch".into());
        }
        let bundle = Bundle::decode(&manifest, None)?;
        if bundle.profile != BundleProfile::TextureTable {
            return Err("Bundle stream qualification requires v2 indexed bundles".into());
        }
        let parent = path.parent().ok_or("Bundle has no directory")?;
        let mut roots = Vec::new();
        for (dep, ext) in bundle.dependencies.iter().zip([
            "theia-mesh",
            "theia-material",
            "theia-uv",
            "theia-textures",
        ]) {
            roots.push(read(
                &parent.join(format!("{}.{ext}", hex(dep.content_id))),
                dep.bytes as usize,
            )?);
        }
        let roots: [Vec<u8>; 4] = roots.try_into().map_err(|_| "Wrong root count")?;
        let checked = bundle.validate_table_roots(std::array::from_fn(|i| roots[i].as_slice()))?;
        if checked.mesh.meshlets.len() > 32
            || checked
                .uvs
                .texcoords
                .iter()
                .flatten()
                .any(|v| v.abs() > 64.)
        {
            return Err("Bundle stream exceeds meshlet/UV qualification bounds".into());
        }
        let mut leaves = Vec::new();
        for dep in &checked.table.textures {
            leaves.push(read(
                &parent.join(hex(dep.content_id) + ".ktx2"),
                dep.bytes as usize,
            )?);
        }
        checked
            .table
            .validate_textures(&leaves.iter().map(Vec::as_slice).collect::<Vec<_>>())?;
        Ok(Self {
            manifest,
            roots,
            leaves,
        })
    }
    fn stage(&self, store: &mut Residency) -> Result<StagedBundle> {
        Ok(store.stage(
            &self.manifest,
            content_id(&self.manifest),
            std::array::from_fn(|i| self.roots[i].as_slice()),
            &self.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        )?)
    }
}
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
    // Transfers retire before dropping the complete CPU reservation.
    buffers: Vec<asset::Upload>,
    images: Vec<texture::Upload>,
    staged: StagedBundle,
    index: usize,
}
pub struct Ready {
    pub buffers: [asset::Buffer; 4],
    pub images: Vec<texture::ResidentTexture>,
    staged: StagedBundle,
    index: usize,
}
pub struct Publication {
    staged: StagedBundle,
    index: usize,
}
impl Ready {
    pub fn into_parts(
        self,
    ) -> (
        [asset::Buffer; 4],
        Vec<texture::ResidentTexture>,
        Publication,
    ) {
        (
            self.buffers,
            self.images,
            Publication {
                staged: self.staged,
                index: self.index,
            },
        )
    }
}
pub struct Gpu<'a> {
    pub device: &'a ash::Device,
    pub memory: &'a vk::PhysicalDeviceMemoryProperties,
    pub family: u32,
    pub queue: vk::Queue,
    pub limits: vk::PhysicalDeviceLimits,
    pub image_limits: vk::ImageFormatProperties,
}
pub struct Stream {
    pending: Option<Pending>,
    loader: Option<Loader>,
    store: Residency,
    paths: [PathBuf; 2],
    active: usize,
    last_attempt: u64,
    gpu_budget: u64,
    image_budget: u64,
    pub publications: u64,
    pub rejections: u64,
}
impl Stream {
    pub fn new(
        paths: [PathBuf; 2],
        cpu_budget: usize,
        gpu_budget: u64,
        image_budget: u64,
    ) -> Result<Self> {
        let mut store = Residency::new(cpu_budget)?;
        let initial = Input::load(&paths[0])?;
        let staged = initial.stage(&mut store)?;
        store.publish(Some(staged))?;
        if paths[0] == paths[1] {
            return Err("Bundle streaming needs two different manifest paths".into());
        }
        Ok(Self {
            pending: None,
            loader: None,
            store,
            paths,
            active: 0,
            last_attempt: 0,
            gpu_budget,
            image_budget,
            publications: 0,
            rejections: 0,
        })
    }
    pub fn budget(&self) -> u64 {
        self.gpu_budget
    }
    /// Frame fence must be retired by the caller before accepting any Ready set.
    pub fn tick(
        &mut self,
        frame: u64,
        active_bytes: u64,
        active_image_bytes: u64,
        gpu: Gpu<'_>,
    ) -> Result<Option<Ready>> {
        if let Some(pending) = &self.pending {
            let buffers_ready = pending
                .buffers
                .iter()
                .map(asset::Upload::ready)
                .collect::<Result<Vec<_>>>()?;
            let images_ready = pending
                .images
                .iter()
                .map(texture::Upload::ready)
                .collect::<Result<Vec<_>>>()?;
            if buffers_ready.iter().chain(&images_ready).all(|v| *v) {
                let pending = self.pending.take().unwrap();
                let buffers = pending
                    .buffers
                    .into_iter()
                    .map(asset::Upload::finish)
                    .collect::<Result<Vec<_>>>()?;
                let images = pending
                    .images
                    .into_iter()
                    .map(texture::Upload::finish)
                    .collect::<Result<Vec<_>>>()?;
                return Ok(Some(Ready {
                    buffers: buffers.try_into().map_err(|_| "Wrong GPU root count")?,
                    images,
                    staged: pending.staged,
                    index: pending.index,
                }));
            }
        }
        if let Some(loader) = &self.loader {
            match loader.receiver.try_recv() {
                Ok(result) => {
                    let index = loader.index;
                    self.loader = None;
                    let input = result.map_err(|v| format!("Bundle I/O failed: {v}"))?;
                    let staged = match input.stage(&mut self.store) {
                        Ok(v) => v,
                        Err(error) if error.to_string() == "live bundle byte budget exhausted" => {
                            self.rejections += 1;
                            println!(
                                "Bundle replacement deferred by CPU encoded-byte budget; current complete set retained"
                            );
                            return Ok(None);
                        }
                        Err(error) => return Err(error),
                    };
                    let count = input.leaves.len() as u32;
                    if count + 4 > gpu.limits.max_per_stage_descriptor_sampled_images
                        || count + 4 > gpu.limits.max_descriptor_set_sampled_images
                        || input
                            .roots
                            .iter()
                            .any(|v| v.len() > gpu.limits.max_storage_buffer_range as usize)
                    {
                        return Err("Bundle set exceeds device descriptor/storage limits".into());
                    }
                    let mut remaining = self
                        .gpu_budget
                        .checked_sub(active_bytes)
                        .ok_or("Active bundle exceeds physical resource budget")?;
                    let mut image_remaining = self
                        .image_budget
                        .checked_sub(active_image_bytes)
                        .ok_or("Active images exceed aggregate image budget")?;
                    let mut pending = Pending {
                        buffers: Vec::new(),
                        images: Vec::new(),
                        staged,
                        index,
                    };
                    let result = (|| -> Result<()> {
                        for bytes in &input.roots {
                            let upload = asset::Upload::begin(
                                gpu.device, gpu.memory, bytes, gpu.family, gpu.queue, remaining,
                                None,
                            )?;
                            remaining = remaining
                                .checked_sub(upload.resident_bytes())
                                .ok_or("Bundle buffer budget accounting underflow")?;
                            pending.buffers.push(upload);
                        }
                        for bytes in &input.leaves {
                            let image = theia_assets::texture::Texture::decode(
                                bytes,
                                None,
                                theia_assets::texture::DEFAULT_BUDGET,
                            )?;
                            if image.width > gpu.image_limits.max_extent.width
                                || image.height > gpu.image_limits.max_extent.height
                            {
                                return Err("Bundle image exceeds device format limits".into());
                            }
                            let upload = texture::Upload::begin(
                                gpu.device,
                                gpu.memory,
                                &image,
                                gpu.family,
                                gpu.queue,
                                remaining.min(image_remaining),
                            )?;
                            remaining = remaining
                                .checked_sub(upload.resident_bytes())
                                .ok_or("Bundle image budget accounting underflow")?;
                            image_remaining = image_remaining
                                .checked_sub(upload.resident_bytes())
                                .ok_or("Bundle image-only budget accounting underflow")?;
                            pending.images.push(upload);
                        }
                        Ok(())
                    })();
                    if let Err(error) = result {
                        if error
                            .downcast_ref::<asset::AllocationBudgetExceeded>()
                            .is_some()
                            || error.to_string()
                                == "Texture image allocation exceeds physical byte budget"
                        {
                            drop(pending);
                            self.rejections += 1;
                            println!(
                                "Bundle replacement deferred by aggregate physical resource budget; pending transfers retired, current complete set retained; CPU unique encoded {} bytes",
                                self.store.live_bytes()
                            );
                            return Ok(None);
                        }
                        return Err(error);
                    }
                    let staging: u64 = pending
                        .buffers
                        .iter()
                        .map(asset::Upload::staging_bytes)
                        .chain(pending.images.iter().map(texture::Upload::staging_bytes))
                        .sum();
                    println!(
                        "Bundle complete set submitted at frame {frame}: 4 root buffers, {count} images; active {active_bytes} + pending {} physical bytes <= {}; staging {staging} bytes separately; CPU unique encoded {} bytes",
                        self.gpu_budget - remaining - active_bytes,
                        self.gpu_budget,
                        self.store.live_bytes()
                    );
                    self.pending = Some(pending);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("Bundle I/O worker disconnected".into());
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
                .name("theia-bundle-io".into())
                .spawn(move || {
                    let result = Input::load(&path).map_err(|v| v.to_string());
                    let _ = sender.try_send(result);
                })?;
            self.loader = Some(Loader {
                receiver,
                worker: Some(worker),
                index,
            });
            self.last_attempt = frame;
            println!("Bundle asynchronous complete read requested at frame {frame}, index {index}");
        }
        Ok(None)
    }
    pub fn published(&mut self, ready: Publication, frame: u64) -> Result<()> {
        // Renderer must have moved all GPU owners into its new complete descriptor generation.
        let snapshot = self.store.publish(Some(ready.staged))?;
        self.active = ready.index;
        self.publications += 1;
        println!(
            "Bundle complete set published at frame {frame}, CPU epoch {}, index {}, {} resources; prior graphics generation retired before old GPU owners",
            snapshot.epoch(),
            self.active,
            snapshot.resource_count()
        );
        Ok(())
    }
}
