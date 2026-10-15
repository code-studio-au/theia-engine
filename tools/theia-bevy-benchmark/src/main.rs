//! Isolated native comparison executable. No Bevy dependency enters Theia runtime.
use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    mesh::{MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        Render, RenderApp, RenderPlugin, RenderSystems,
        pipelined_rendering::PipelinedRenderingPlugin,
        render_resource::{
            AsBindGroup, CommandEncoderDescriptor, Extent3d, LoadOp, Operations,
            RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, ShaderType,
            SpecializedMeshPipelineError, StoreOp, TextureDimension, TextureFormat,
        },
        renderer::{RenderAdapterInfo, RenderDevice, RenderQueue, render_system},
        settings::{Backends, RenderCreation, WgpuSettings},
        view::window::{
            ExtractedWindows,
            screenshot::{Screenshot, ScreenshotCaptured},
        },
    },
    shader::ShaderRef,
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{num::NonZeroU32, path::PathBuf, time::Instant};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
// Share the engine's qualified readonly workload/camera functions. Vulkan
// recorder code in the module is unused; no Theia renderer runs in this process.
#[allow(dead_code)]
#[path = "../../../apps/theia-render-lab/src/benchmark.rs"]
mod benchmark;
#[allow(dead_code)]
#[path = "../../../apps/theia-render-lab/src/camera.rs"]
mod camera;

#[derive(Deserialize)]
struct TextureData {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}
#[derive(Deserialize)]
struct Part {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    factor: [f32; 4],
    texture: TextureData,
}
#[derive(Deserialize)]
struct Chunk {
    id: usize,
    region: usize,
    parts: Vec<Part>,
    ground: Vec<[[f32; 3]; 3]>,
}
#[derive(Deserialize)]
struct Frame {
    frame: usize,
    time: f32,
    selected: [usize; 4],
    eye: [f32; 3],
    rows: [[f32; 4]; 4],
}
#[derive(Deserialize, Resource)]
struct Fixture {
    schema: u32,
    workload: String,
    players: u32,
    extent: [u32; 2],
    warmup: usize,
    frames: Vec<Frame>,
    chunks: Vec<Chunk>,
    source_report_sha256: String,
    source_manifest_sha256: String,
}
#[derive(Resource)]
struct Run {
    output: PathBuf,
    pre_roll: usize,
    tick: usize,
    snapshot: Option<usize>,
    started: Instant,
    last: Instant,
    update_started: Instant,
    samples: Vec<Value>,
    captured: bool,
    metadata: Value,
}
#[derive(Component)]
struct Proxy(u32);
#[derive(Component)]
struct WorldPart(usize);
#[derive(Resource)]
struct ProxyMeshes {
    near: [Handle<Mesh>; 5],
    far: [Handle<Mesh>; 5],
}
#[derive(Resource)]
struct Ground(Vec<GroundSurface>);
struct GroundSurface {
    triangles: Vec<[[f32; 3]; 3]>,
    cells: Vec<Vec<usize>>,
    origin: [f32; 2],
}
impl GroundSurface {
    fn new(chunk: &Chunk) -> Self {
        let origin = [
            (chunk.region % 2) as f32 * 64. - 64.,
            (chunk.region / 2) as f32 * 64. - 64.,
        ];
        let cells = (0..64)
            .map(|cell| {
                let lo = [
                    origin[0] + (cell % 8) as f32 * 8.,
                    origin[1] + (cell / 8) as f32 * 8.,
                ];
                chunk
                    .ground
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| {
                        (0..2).all(|axis| {
                            t.iter().any(|p| p[axis] > lo[axis])
                                && t.iter().any(|p| p[axis] < lo[axis] + 8.)
                        })
                    })
                    .map(|(id, _)| id)
                    .collect()
            })
            .collect();
        Self {
            triangles: chunk.ground.clone(),
            cells,
            origin,
        }
    }
    fn height(&self, p: [f32; 2]) -> f32 {
        let cell: [usize; 2] = std::array::from_fn(|axis| {
            (((p[axis] - self.origin[axis]) / 8.).floor() as usize).min(7)
        });
        let cross = |a: [f32; 2], b: [f32; 2]| a[0] * b[1] - a[1] * b[0];
        for &index in &self.cells[cell[1] * 8 + cell[0]] {
            let [a, b, c] = self.triangles[index];
            let ab = [b[0] - a[0], b[1] - a[1]];
            let ac = [c[0] - a[0], c[1] - a[1]];
            let ap = [p[0] - a[0], p[1] - a[1]];
            let denominator = cross(ab, ac);
            let u = cross(ap, ac) / denominator;
            let v = cross(ab, ap) / denominator;
            if u >= -1e-6 && v >= -1e-6 && u + v <= 1. + 1e-6 {
                return a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2]);
            }
        }
        panic!("Qualified fixture has no ground at {p:?}")
    }
}
#[derive(Clone, ShaderType)]
struct SurfaceUniform {
    factor: Vec4,
    options: Vec4,
}
#[derive(Asset, TypePath, AsBindGroup, Clone)]
#[data(0, SurfaceUniform, binding_array(10))]
#[bindless]
struct SurfaceMaterial {
    surface: SurfaceUniform,
    #[texture(1)]
    #[sampler(2)]
    texture: Handle<Image>,
}
impl From<&SurfaceMaterial> for SurfaceUniform {
    fn from(material: &SurfaceMaterial) -> Self {
        material.surface.clone()
    }
}
impl Material for SurfaceMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/theia_unlit.wgsl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> std::result::Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
fn mesh(positions: Vec<[f32; 3]>, uvs: Vec<[f32; 2]>, colors: Option<Vec<[f32; 4]>>) -> Mesh {
    let count = positions.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; count]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    if let Some(colors) = colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh
}
const CUBE: [[usize; 3]; 12] = [
    [0, 2, 1],
    [1, 2, 3],
    [4, 5, 6],
    [5, 7, 6],
    [0, 1, 4],
    [1, 5, 4],
    [2, 6, 3],
    [3, 6, 7],
    [0, 4, 2],
    [2, 4, 6],
    [1, 3, 5],
    [3, 7, 5],
];
fn proxy_mesh(color: usize, near: bool) -> Mesh {
    let fraction = color as f32 / 4.;
    let base = [0.15, 0.3, 0.8].map(|v| v * (1. - fraction));
    let other = [0.8, 0.15, 0.05];
    let base: [f32; 3] = std::array::from_fn(|i| base[i] + other[i] * fraction);
    let corner = |i: usize| {
        [
            if i & 1 != 0 { 0.3 } else { -0.3 },
            if i & 2 != 0 { 0.3 } else { -0.3 },
            if i & 4 != 0 { 0.8 } else { -0.8 },
        ]
    };
    let mut positions = CUBE
        .iter()
        .flat_map(|tri| tri.iter().map(|i| corner(*i)))
        .collect::<Vec<_>>();
    let mut colors = (0..12)
        .flat_map(|primitive| {
            let light = [0.35, 1., 0.55, 0.8, 0.5, 0.75][primitive / 2];
            std::iter::repeat_n([base[0] * light, base[1] * light, base[2] * light, 1.], 3)
        })
        .collect::<Vec<_>>();
    if near {
        let roof = [
            [-0.3, -0.3, 0.8],
            [0.3, -0.3, 0.8],
            [0.3, 0.3, 0.8],
            [-0.3, 0.3, 0.8],
        ];
        for side in 0..4 {
            positions.extend([roof[side], roof[(side + 1) % 4], [0., 0., 1.6]]);
            colors.extend([[base[0] * 0.7, base[1] * 0.7, base[2] * 0.7, 1.]; 3]);
        }
    }
    let count = positions.len();
    mesh(positions, vec![[0., 0.]; count], Some(colors))
}
fn pose(fixture: &Fixture, ground: &Ground, frame: usize, id: u32) -> ([f32; 3], bool) {
    let f = &fixture.frames[frame];
    let (mut center, extent, _) = benchmark::object(id, fixture.players, f.time);
    let region = usize::from(center[0] >= 0.) + 2 * usize::from(center[1] >= 0.);
    center[2] = ground.0[f.selected[region]].height([center[0], center[1]]) + extent[2];
    let near = center
        .iter()
        .zip(f.eye)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        < 35. * 35.;
    (center, near)
}
fn setup(
    mut commands: Commands,
    fixture: Res<Fixture>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
    gpu: (Res<RenderAdapterInfo>, Res<RenderDevice>),
    mut run: ResMut<Run>,
) {
    let (adapter, device) = gpu;
    run.metadata = json!({"device":adapter.name,"backend":format!("{:?}",adapter.backend),"vendor_id":adapter.vendor,"device_id":adapter.device,"driver":adapter.driver,"driver_info":adapter.driver_info,"bindless_material_supported":SurfaceMaterial::bindless_supported(&device),"wgpu_instance_layers_env":std::env::var("VK_INSTANCE_LAYERS").ok()});
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 65f32.to_radians(),
            near: 0.1,
            far: 250.,
            ..default()
        }),
        Tonemapping::None,
        DebandDither::Disabled,
        Msaa::Off,
    ));
    for chunk in &fixture.chunks {
        for part in &chunk.parts {
            let mut image = Image::new(
                Extent3d {
                    width: part.texture.width,
                    height: part.texture.height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                part.texture.rgba.clone(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            let mut sampler = ImageSamplerDescriptor::nearest();
            sampler.address_mode_u = ImageAddressMode::Repeat;
            sampler.address_mode_v = ImageAddressMode::Repeat;
            image.sampler = ImageSampler::Descriptor(sampler);
            let texture = images.add(image);
            let material = materials.add(SurfaceMaterial {
                surface: SurfaceUniform {
                    factor: Vec4::from_array(part.factor),
                    options: Vec4::ZERO,
                },
                texture,
            });
            commands.spawn((
                Mesh3d(meshes.add(mesh(part.positions.clone(), part.uvs.clone(), None))),
                MeshMaterial3d(material),
                WorldPart(chunk.id),
                Visibility::Hidden,
            ));
        }
    }
    let near = std::array::from_fn(|i| meshes.add(proxy_mesh(i, true)));
    let far = std::array::from_fn(|i| meshes.add(proxy_mesh(i, false)));
    let texture = images.add(Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255; 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ));
    let material = materials.add(SurfaceMaterial {
        surface: SurfaceUniform {
            factor: Vec4::ONE,
            options: Vec4::X,
        },
        texture,
    });
    for i in 0..fixture.players {
        let id = i + benchmark::STATIC_OBJECTS;
        commands.spawn((
            Mesh3d(far[(id % 5) as usize].clone()),
            MeshMaterial3d(material.clone()),
            Proxy(id),
        ));
    }
    commands.insert_resource(ProxyMeshes { near, far });
}
fn begin(fixture: Res<Fixture>, mut run: ResMut<Run>) {
    let now = Instant::now();
    let interval = (now - run.last).as_secs_f64() * 1000.;
    run.last = now;
    run.update_started = now;
    if run.tick >= run.pre_roll && run.tick - run.pre_roll < fixture.frames.len() {
        let index = run.tick - run.pre_roll;
        run.samples
            .push(json!({"frame":index,"frame_interval_ms":interval}));
    }
}
fn update(
    fixture: Res<Fixture>,
    ground: Res<Ground>,
    meshes: Res<ProxyMeshes>,
    run: Res<Run>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut parts: Query<(&WorldPart, &mut Visibility)>,
    mut proxies: Query<(&Proxy, &mut Transform, &mut Mesh3d), Without<Camera3d>>,
) {
    let index = run.snapshot.unwrap_or(
        run.tick
            .saturating_sub(run.pre_roll)
            .min(fixture.frames.len() - 1),
    );
    let f = &fixture.frames[index];
    let (eye, target) = benchmark::rig(f.time, 60.);
    *camera.single_mut().unwrap() = Transform::from_translation(Vec3::from_array(eye))
        .looking_at(Vec3::from_array(target), Vec3::Z);
    for (part, mut visibility) in &mut parts {
        let selected = if f.selected.contains(&part.0) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != selected {
            *visibility = selected;
        }
    }
    proxies
        .par_iter_mut()
        .for_each(|(proxy, mut transform, mut mesh)| {
            let (center, near) = pose(&fixture, &ground, index, proxy.0);
            transform.translation = Vec3::from_array(center);
            let desired = if near {
                &meshes.near[(proxy.0 % 5) as usize]
            } else {
                &meshes.far[(proxy.0 % 5) as usize]
            };
            if mesh.0 != *desired {
                mesh.0 = desired.clone();
            }
        });
}
fn finish(
    mut commands: Commands,
    fixture: Res<Fixture>,
    mut run: ResMut<Run>,
    mut exit: MessageWriter<AppExit>,
) {
    let update_ms = run.update_started.elapsed().as_secs_f64() * 1000.;
    if run.tick >= run.pre_roll && run.tick - run.pre_roll < fixture.frames.len() {
        let index = run.tick - run.pre_roll;
        if let Some(sample) = run.samples.last_mut() {
            sample["cpu_main_schedule_ms"] = json!(update_ms);
            sample["scene_time"] = json!(fixture.frames[index].time);
            sample["selected"] = json!(fixture.frames[index].selected);
        }
    }
    if let Some(frame) = run.snapshot {
        if run.tick == run.pre_roll {
            let output = run.output.clone();
            commands.spawn(Screenshot::primary_window()).observe(move |event:On<ScreenshotCaptured>,mut run:ResMut<Run>| {
                std::fs::write(output.join("screenshot.raw"),event.image.data.as_ref().unwrap()).unwrap();
                std::fs::write(output.join("screenshot.json"),serde_json::to_vec_pretty(&json!({"frame":frame,"width":event.image.width(),"height":event.image.height(),"format":format!("{:?}",event.image.texture_descriptor.format)})).unwrap()).unwrap();
                run.captured=true;
            });
        }
        if run.captured {
            exit.write(AppExit::Success);
        }
        assert!(run.tick < run.pre_roll + 600, "Screenshot did not complete");
    } else if run.tick == run.pre_roll + fixture.frames.len() {
        let measured = &run.samples[fixture.warmup..];
        let mut values = measured
            .iter()
            .map(|s| s["frame_interval_ms"].as_f64().unwrap())
            .collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        let quantile = |p: f64| values[((values.len() - 1) as f64 * p).ceil() as usize];
        let report = json!({"schema":1,"engine":"bevy-0.19.1","workload":fixture.workload,"players":fixture.players,"frames":fixture.frames.len(),"warmup_frames":fixture.warmup,"pre_roll_frames":run.pre_roll,"extent":fixture.extent,"presentation":"FIFO; requested maximum frame latency 1; pipelined rendering disabled","materials":"opaque unlit; source textures/linear factors; proxy face colors and view-depth fog","streaming":false,"asset_residency":"all eight source graphs preloaded; recorded selection replay","motion":"CPU ECS; shared Theia readonly motion; CPU selected-terrain query","gpu_timestamps":false,"cpu_metric":"First-to-Last main schedule only; excludes later render extraction/submission","wall_seconds":run.started.elapsed().as_secs_f64(),"source_report_sha256":fixture.source_report_sha256,"source_manifest_sha256":fixture.source_manifest_sha256,"summary_frame_interval_ms":{"mean":values.iter().sum::<f64>()/values.len() as f64,"p50":quantile(0.5),"p95":quantile(0.95),"p99":quantile(0.99),"max":values.last()},"samples":run.samples,"metadata":run.metadata});
        std::fs::write(
            run.output.join("report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        exit.write(AppExit::Success);
    }
    run.tick += 1;
}
/// Bevy 0.19.1 can present its initial acquired surface before a camera view
/// has rendered. Explicitly initialize that image; later frames skip this pass.
/// Qualification found both PRESENT layout and acquire-semaphore diagnostics
/// without it. This startup work is outside all recorded benchmark samples.
fn initialize_first_surface(
    windows: Res<ExtractedWindows>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    for window in windows.windows.values().filter(|w| w.needs_initial_present) {
        let Some(view) = window.swap_chain_texture_view.as_ref() else {
            continue;
        };
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("comparison initial surface clear"),
        });
        {
            let attachments = [Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(LinearRgba::new(0.025, 0.07, 0.12, 1.).into()),
                    store: StoreOp::Store,
                },
            })];
            let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("comparison initial clear"),
                color_attachments: &attachments,
                ..default()
            });
        }
        queue.submit([encoder.finish()]);
    }
}
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut path = None;
    let mut output = None;
    let mut snapshot = None;
    let mut reference = None;
    let mut pre_roll = 120;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fixture" => path = Some(PathBuf::from(args.next().ok_or("Missing fixture")?)),
            "--output" => output = Some(PathBuf::from(args.next().ok_or("Missing output")?)),
            "--snapshot" => {
                snapshot = Some(
                    args.next()
                        .ok_or("Missing snapshot frame")?
                        .parse::<usize>()?,
                )
            }
            "--reference" => {
                reference = Some(
                    args.next()
                        .ok_or("Missing reference frame")?
                        .parse::<usize>()?,
                )
            }
            "--pre-roll" => pre_roll = args.next().ok_or("Missing pre-roll")?.parse::<usize>()?,
            _ => return Err(format!("Unknown argument: {arg}").into()),
        }
    }
    let path = path.ok_or("--fixture is required")?;
    let bytes = std::fs::read(path)?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Fixture exceeds 32 MiB".into());
    }
    let fixture: Fixture = serde_json::from_slice(&bytes)?;
    if fixture.schema != 1
        || fixture.players > 10000
        || fixture.chunks.len() != 8
        || fixture.frames.len() < 3
        || fixture.warmup + 2 >= fixture.frames.len()
        || fixture.extent.iter().any(|v| !(64..=8192).contains(v))
        || !(2..=1200).contains(&pre_roll)
    {
        return Err("Invalid comparison fixture/configuration".into());
    }
    for (i, c) in fixture.chunks.iter().enumerate() {
        if c.id != i || c.region != i % 4 || c.ground.is_empty() || c.parts.is_empty() {
            return Err("Invalid region ordering/content".into());
        }
    }
    for (i, f) in fixture.frames.iter().enumerate() {
        if f.frame != i
            || !f.time.is_finite()
            || f.selected
                .iter()
                .enumerate()
                .any(|(i, id)| *id != i && *id != i + 4)
        {
            return Err("Invalid route selection/time".into());
        }
    }
    if snapshot
        .or(reference)
        .is_some_and(|v| v >= fixture.frames.len())
    {
        return Err("Frame outside fixture".into());
    }
    let ground = Ground(fixture.chunks.iter().map(GroundSurface::new).collect());
    if let Some(frame) = reference {
        let objects = (0..fixture.players)
            .map(|i| {
                let id = i + benchmark::STATIC_OBJECTS;
                let (center, near) = pose(&fixture, &ground, frame, id);
                json!({"id":id,"center":center,"extent":[0.3,0.3,0.8],"near":near})
            })
            .collect::<Vec<_>>();
        println!(
            "{}",
            json!({"frame":frame,"time":fixture.frames[frame].time,"eye":fixture.frames[frame].eye,"rows":fixture.frames[frame].rows,"selected":fixture.frames[frame].selected,"objects":objects})
        );
        return Ok(());
    }
    let output = output.ok_or("--output is required")?;
    std::fs::create_dir_all(&output)?;
    for name in ["report.json", "screenshot.raw", "screenshot.json"] {
        let p = output.join(name);
        if p.exists() {
            std::fs::remove_file(p)?;
        }
    }
    let extent = fixture.extent;
    let frames = fixture.frames.len();
    let now = Instant::now();
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::linear_rgb(0.025, 0.07, 0.12)))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .insert_resource(fixture)
        .insert_resource(ground)
        .insert_resource(Run {
            output,
            pre_roll,
            tick: 0,
            snapshot,
            started: now,
            last: now,
            update_started: now,
            samples: Vec::with_capacity(frames),
            captured: false,
            metadata: Value::Null,
        })
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                        backends: Some(Backends::VULKAN),
                        ..default()
                    })),
                    synchronous_pipeline_compilation: true,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Theia benchmark — Bevy reference".into(),
                        resolution: WindowResolution::new(extent[0], extent[1])
                            .with_scale_factor_override(1.),
                        present_mode: PresentMode::Fifo,
                        desired_maximum_frame_latency: NonZeroU32::new(1),
                        ..default()
                    }),
                    ..default()
                })
                .disable::<PipelinedRenderingPlugin>(),
        )
        .add_plugins(MaterialPlugin::<SurfaceMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(First, begin)
        .add_systems(Update, update)
        .add_systems(Last, finish);
    app.sub_app_mut(RenderApp).add_systems(
        Render,
        initialize_first_surface
            .in_set(RenderSystems::Render)
            .before(render_system),
    );
    app.run();
    Ok(())
}
