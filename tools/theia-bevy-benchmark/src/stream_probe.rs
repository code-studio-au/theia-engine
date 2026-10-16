//! Controlled coarse -> asynchronous fine -> retired gate. This is deliberately
//! separate from camera scheduling and from claims of physical VRAM equivalence.
use super::{
    Chunk, Fixture, Part, Result, Run, SurfaceMaterial, SurfaceUniform, WorldPart, cooked,
};
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    pbr::PreparedMaterial,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        erased_render_asset::ErasedRenderAssets,
        mesh::RenderMesh,
        render_asset::RenderAssets,
        renderer::{RenderDevice, RenderQueue, render_system},
        texture::GpuImage,
        view::window::screenshot::{Screenshot, ScreenshotCaptured},
    },
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
    time::Duration,
};
use theia_assets::{
    bundle_residency::{StagedBundle, WorldResidency},
    content_id,
    world_coverage::{ProtectedWorld, Region},
    world_demand::ChunkBounds,
};

#[derive(Default)]
pub(super) struct Owners {
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    images: Vec<Handle<Image>>,
    materials: Vec<Handle<SurfaceMaterial>>,
    logical_bytes: u64,
}
pub(super) fn spawn(
    chunk: &Chunk,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<SurfaceMaterial>,
) -> Owners {
    let mut owners = Owners {
        logical_bytes: cooked::logical_bytes(chunk),
        ..default()
    };
    for Part {
        positions,
        uvs,
        factor,
        texture,
    } in &chunk.parts
    {
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            texture.rgba.clone(),
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        let mut sampler = ImageSamplerDescriptor::nearest();
        sampler.address_mode_u = ImageAddressMode::Repeat;
        sampler.address_mode_v = ImageAddressMode::Repeat;
        image.sampler = ImageSampler::Descriptor(sampler);
        let image = images.add(image);
        let material = materials.add(SurfaceMaterial {
            surface: SurfaceUniform {
                factor: Vec4::from_array(*factor),
                options: Vec4::ZERO,
            },
            texture: image.clone(),
        });
        let mesh = meshes.add(super::mesh(positions.clone(), uvs.clone(), None));
        owners.entities.push(
            commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    WorldPart(chunk.id),
                    Visibility::Hidden,
                ))
                .id(),
        );
        owners.meshes.push(mesh);
        owners.images.push(image);
        owners.materials.push(material);
    }
    owners
}
#[derive(Clone, Default)]
struct Watch {
    meshes: Vec<AssetId<Mesh>>,
    images: Vec<AssetId<Image>>,
    materials: Vec<AssetId<SurfaceMaterial>>,
    remove: bool,
    ready: bool,
}
impl Watch {
    fn owners(owners: &[&Owners], remove: bool) -> Self {
        Self {
            meshes: owners
                .iter()
                .flat_map(|o| o.meshes.iter().map(Handle::id))
                .collect(),
            images: owners
                .iter()
                .flat_map(|o| o.images.iter().map(Handle::id))
                .collect(),
            materials: owners
                .iter()
                .flat_map(|o| o.materials.iter().map(Handle::id))
                .collect(),
            remove,
            ..default()
        }
    }
}
#[derive(Resource, Clone, Default)]
struct Bridge(Arc<Mutex<Watch>>);
struct Worker {
    receiver: Mutex<mpsc::Receiver<std::result::Result<cooked::Input, String>>>,
    join: Option<JoinHandle<()>>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
#[derive(Resource)]
pub(super) struct Probe {
    pub enabled: bool,
    region: usize,
    bootstrap: Vec<Chunk>,
    coarse: Vec<Owners>,
    fine: Option<Owners>,
    worker: Option<Worker>,
    staged: Option<StagedBundle>,
    paths: Vec<PathBuf>,
    bridge: Bridge,
    phase: &'static str,
    changed: usize,
    cancel: Option<usize>,
    budget: u64,
    cpu_budget: usize,
    base_cpu: usize,
    base_logical: u64,
    retiring_bytes: u64,
    history: Vec<serde_json::Value>,
    captures: Arc<Mutex<Vec<String>>>,
    captured: Vec<&'static str>,
    pub selected: [usize; 4],
    // Keep encoded ownership after worker/candidate handle owners on shutdown.
    cpu: Option<ProtectedWorld>,
}
impl Default for Probe {
    fn default() -> Self {
        Self {
            enabled: false,
            region: 0,
            cpu: None,
            bootstrap: Vec::new(),
            coarse: Vec::new(),
            fine: None,
            worker: None,
            staged: None,
            paths: Vec::new(),
            bridge: default(),
            phase: "disabled",
            changed: 0,
            cancel: None,
            budget: 0,
            cpu_budget: 0,
            base_cpu: 0,
            base_logical: 0,
            retiring_bytes: 0,
            history: Vec::new(),
            captures: default(),
            captured: Vec::new(),
            selected: [4, 5, 6, 7],
        }
    }
}
impl Probe {
    pub fn new(
        paths: Vec<PathBuf>,
        region: usize,
        cpu_budget: usize,
        budget: u64,
        cancel: Option<usize>,
    ) -> Result<Self> {
        if paths.len() != 8 {
            return Err(
                "Streaming probe requires eight source bundle paths; re-export fixture".into(),
            );
        }
        let mut store = WorldResidency::new(cpu_budget, 8)?;
        let mut staged = Vec::new();
        let mut coarse = Vec::new();
        let mut regions = Vec::new();
        // Match Theia's startup semantic prevalidation; only four coarse graphs
        // remain in the encoded store. Fine reload later reopens actual files.
        for (id, path) in paths.iter().enumerate() {
            let input = cooked::Input::load(path)?;
            let chunk = input.chunk(id)?;
            if id < 4 {
                let positions = chunk
                    .parts
                    .iter()
                    .flat_map(|p| p.positions.iter())
                    .collect::<Vec<_>>();
                regions.push(Region {
                    fine: ChunkBounds {
                        id: id as u64,
                        min: std::array::from_fn(|a| {
                            positions.iter().map(|p| p[a]).fold(f32::INFINITY, f32::min)
                        }),
                        max: std::array::from_fn(|a| {
                            positions
                                .iter()
                                .map(|p| p[a])
                                .fold(f32::NEG_INFINITY, f32::max)
                        }),
                    },
                    fallback: id as u64 + 4,
                });
            } else {
                staged.push((
                    id as u64,
                    store.stage(
                        &input.manifest,
                        content_id(&input.manifest),
                        std::array::from_fn(|i| input.roots[i].as_slice()),
                        &input.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                    )?,
                ));
                coarse.push(chunk);
            }
        }
        store.publish(staged, &[])?;
        let cpu = ProtectedWorld::new(store, &regions)?;
        let base_cpu = cpu.live_bytes();
        let base_logical = coarse.iter().map(cooked::logical_bytes).sum();
        if base_logical > budget {
            return Err("Logical upload budget cannot retain protected coarse coverage".into());
        }
        Ok(Self {
            enabled: true,
            region,
            cpu: Some(cpu),
            bootstrap: coarse,
            paths,
            cancel,
            budget,
            cpu_budget,
            base_cpu,
            base_logical,
            phase: "bootstrap",
            ..default()
        })
    }
    pub fn setup(
        &mut self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
        materials: &mut Assets<SurfaceMaterial>,
    ) {
        for chunk in std::mem::take(&mut self.bootstrap) {
            self.coarse
                .push(spawn(&chunk, commands, meshes, images, materials));
        }
        *self.bridge.0.lock().unwrap() =
            Watch::owners(&self.coarse.iter().collect::<Vec<_>>(), false);
    }
    fn event(&mut self, frame: usize, phase: &'static str) {
        println!("Streaming probe frame {frame}: {phase}");
        self.phase = phase;
        self.changed = frame;
        assert!(self.history.len() < 16);
        self.history.push(serde_json::json!({"frame":frame,"phase":phase,"cpu_unique_bytes":self.cpu.as_ref().unwrap().live_bytes(),"logical_asset_bytes":self.base_logical+self.retiring_bytes+self.fine.as_ref().map_or(0,|o|o.logical_bytes),"selected":self.selected}));
    }
    pub fn report(&self) -> serde_json::Value {
        if !self.enabled {
            return serde_json::Value::Null;
        }
        let cpu = self.cpu.as_ref().unwrap();
        let watch = self.bridge.0.lock().unwrap().clone();
        serde_json::json!({"phase":self.phase,"region":self.region,"history":self.history,"captures":*self.captures.lock().unwrap(),"cpu_unique_bytes":cpu.live_bytes(),"base_cpu_bytes":self.base_cpu,"base_logical_asset_bytes":self.base_logical,"logical_budget":self.budget,"cpu_budget":self.cpu_budget,"selected":self.selected,"physical_gpu_bytes":null,"gpu_ready":watch.ready,"watch_removal":watch.remove,"scope":"controlled one-fine cooked-file reload and retirement; encoded ownership charged by shared ProtectedWorld; adapted mesh/texel/uniform logical bytes exclude allocator slabs, work, proxy, ground, staging, driver and fixture metadata; not equivalent camera streaming"})
    }
    pub fn complete(&self) -> bool {
        !self.enabled
            || (matches!(
                self.phase,
                "retired" | "cancelled" | "deferred_cpu" | "deferred_logical"
            ) && self.captures.lock().unwrap().len() == self.captured.len()
                && self.captured.len() >= 2
                && self.cpu.as_ref().unwrap().live_bytes() == self.base_cpu)
    }
}
pub(super) fn install(app: &mut App) {
    let bridge = app.world().resource::<Probe>().bridge.clone();
    app.sub_app_mut(RenderApp)
        .insert_resource(bridge)
        .add_systems(
            Render,
            watch_gpu
                .in_set(RenderSystems::Cleanup)
                .after(render_system),
        );
}
fn watch_gpu(
    bridge: Res<Bridge>,
    meshes: Res<RenderAssets<RenderMesh>>,
    images: Res<RenderAssets<GpuImage>>,
    materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let mut watch = bridge.0.lock().unwrap();
    if watch.ready || watch.meshes.is_empty() {
        return;
    }
    let matches = watch
        .meshes
        .iter()
        .all(|id| meshes.get(*id).is_some() != watch.remove)
        && watch
            .images
            .iter()
            .all(|id| images.get(*id).is_some() != watch.remove)
        && watch
            .materials
            .iter()
            .all(|id| materials.get(*id).is_some() != watch.remove);
    if matches {
        // Flush pending queue writes and retire all earlier rendering before an
        // acknowledgement can publish or release encoded owners in main world.
        let submission = queue.submit([]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("Streaming GPU retirement");
        watch.ready = true;
    }
}
pub(super) fn tick(
    mut commands: Commands,
    mut probe: ResMut<Probe>,
    mut fixture: ResMut<Fixture>,
    run: Res<Run>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
) {
    if !probe.enabled || run.tick < run.pre_roll {
        return;
    }
    let frame = run.tick - run.pre_roll;
    let ready = probe.bridge.0.lock().unwrap().ready;
    if frame == 0 {
        assert!(
            ready,
            "Protected coarse GPU assets not ready after pre-roll"
        );
        probe.event(frame, "coarse");
    }
    if probe.phase == "loading" && probe.cancel.is_some_and(|f| frame >= f) {
        drop(probe.worker.take());
        probe.event(frame, "cancelled");
    }
    if probe.phase == "coarse" && frame >= 20 {
        let path = probe.paths[probe.region].clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let join = std::thread::spawn(move || {
            let result = cooked::Input::load(&path).map_err(|e| e.to_string());
            let _ = sender.send(result);
        });
        probe.worker = Some(Worker {
            receiver: Mutex::new(receiver),
            join: Some(join),
        });
        probe.event(frame, "loading");
    } else if probe.phase == "loading" {
        let result = probe
            .worker
            .as_ref()
            .unwrap()
            .receiver
            .lock()
            .unwrap()
            .try_recv();
        match result {
            Ok(result) => {
                let input = result.expect("Streaming cooked source failed");
                drop(probe.worker.take());
                let staged = probe.cpu.as_mut().unwrap().stage(
                    &input.manifest,
                    content_id(&input.manifest),
                    std::array::from_fn(|i| input.roots[i].as_slice()),
                    &input.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                );
                match staged {
                    Err(e) if e.0 == "live bundle byte budget exhausted" => {
                        probe.event(frame, "deferred_cpu")
                    }
                    Err(e) => panic!("Fine admission failed: {e}"),
                    Ok(staged) => {
                        let chunk = input.chunk(probe.region).expect("Fine conversion");
                        if probe.base_logical + cooked::logical_bytes(&chunk) > probe.budget {
                            drop(staged);
                            probe.event(frame, "deferred_logical");
                        } else {
                            let owners = spawn(
                                &chunk,
                                &mut commands,
                                &mut meshes,
                                &mut images,
                                &mut materials,
                            );
                            *probe.bridge.0.lock().unwrap() = Watch::owners(&[&owners], false);
                            probe.fine = Some(owners);
                            probe.staged = Some(staged);
                            probe.event(frame, "uploading");
                        }
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => panic!("Streaming worker disconnected"),
        }
    } else if probe.phase == "uploading" && ready && frame >= 40 && probe.cancel.is_none() {
        let region = probe.region;
        let staged = probe.staged.take().unwrap();
        probe
            .cpu
            .as_mut()
            .unwrap()
            .publish_fine(vec![(region as u64, staged)], &[])
            .expect("Fine publication");
        probe.selected[region] = region;
        probe.event(frame, "published");
    } else if probe.phase == "published" && frame >= 80
        || probe.phase == "uploading" && probe.cancel.is_some_and(|f| frame >= f)
    {
        let region = probe.region;
        probe.selected[region] = region + 4;
        // Keep candidate handles while a complete coarse frame and all earlier
        // graphics retire. Only then remove main-world assets/entities.
        *probe.bridge.0.lock().unwrap() = Watch::owners(&[probe.fine.as_ref().unwrap()], false);
        probe.event(frame, "retiring");
    } else if probe.phase == "retiring" && ready && frame > probe.changed {
        let owners = probe.fine.take().unwrap();
        probe.retiring_bytes = owners.logical_bytes;
        for entity in &owners.entities {
            commands.entity(*entity).despawn();
        }
        for h in &owners.meshes {
            meshes.remove(h.id());
        }
        for h in &owners.images {
            images.remove(h.id());
        }
        for h in &owners.materials {
            materials.remove(h.id());
        }
        *probe.bridge.0.lock().unwrap() = Watch::owners(&[&owners], true);
        // Bevy removes GPU render assets on last-handle Unused, not merely
        // main-world Removed. The ID-only watch must never retain these handles.
        drop(owners);
        probe.event(frame, "unloading");
    } else if probe.phase == "unloading" && ready {
        probe.retiring_bytes = 0;
        let region = probe.region;
        let phase = if probe.staged.take().is_some() {
            "cancelled"
        } else {
            probe
                .cpu
                .as_mut()
                .unwrap()
                .publish_fine(Vec::new(), &[region as u64])
                .expect("Fine encoded retirement");
            "retired"
        };
        probe.event(frame, phase);
    }
    if probe.phase == "loading" && probe.cancel.is_some_and(|f| frame >= f) {
        drop(probe.worker.take());
        probe.event(frame, "cancelled");
    }
    if frame < fixture.frames.len() {
        fixture.frames[frame].selected = probe.selected;
    }
    let label = if probe.phase == "coarse" && frame == 10 {
        Some("coarse")
    } else if probe.phase == "published" && frame == probe.changed + 6 {
        Some("fine")
    } else if matches!(
        probe.phase,
        "retired" | "cancelled" | "deferred_cpu" | "deferred_logical"
    ) && frame == probe.changed + 6
    {
        Some("final")
    } else {
        None
    };
    if let Some(label) = label {
        probe.captured.push(label);
        let output = run.output.join(label);
        std::fs::create_dir_all(&output).unwrap();
        let selected = probe.selected;
        let captures = probe.captures.clone();
        commands.spawn(Screenshot::primary_window()).observe(move|event:On<ScreenshotCaptured>|{
            std::fs::write(output.join("screenshot.raw"),event.image.data.as_ref().unwrap()).unwrap();
            std::fs::write(output.join("screenshot.json"),serde_json::to_vec_pretty(&serde_json::json!({"frame":frame,"selected":selected,"width":event.image.width(),"height":event.image.height(),"format":format!("{:?}",event.image.texture_descriptor.format),"overlay":false})).unwrap()).unwrap();captures.lock().unwrap().push(label.to_string());
        });
    }
}
