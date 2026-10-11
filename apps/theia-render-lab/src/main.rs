mod asset;
mod authority;
mod authority_demo;
mod bundle_streaming;
mod camera;
mod gpu;
mod indirect;
mod lod_reference;
mod network;
mod pipeline;
mod spatial;
mod streaming;
mod surface_reference;
mod texture;
mod visibility;
mod wsi;

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    window::{Window, WindowId},
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenderMode {
    Authority,
    Indirect,
    Spatial,
    Visibility,
    Asset,
    AssetCamera,
    AssetCull,
    AssetLod,
    AssetResolve,
    DirectTask,
    MeshOnly,
    Clear,
}

#[derive(Default)]
struct Options {
    network_address: Option<std::net::SocketAddr>,
    network_certificate: Option<std::path::PathBuf>,
    network_faults: bool,
    network_repair_test: bool,
    authority_alpha: Option<f32>,
    authority_demo: bool,
    authority_script: bool,
    no_validation: bool,
    gpu: Option<String>,
    frames: Option<u64>,
    smoke: bool,
    wsi_smoke: bool,
    mode: Option<RenderMode>,
    work_items: Option<u32>,
    shader_dir: Option<std::path::PathBuf>,
    asset: Option<std::path::PathBuf>,
    fallback: Option<std::path::PathBuf>,
    asset_budget: Option<usize>,
    cell: Option<std::path::PathBuf>,
    resident_budget: Option<u64>,
    camera: bool,
    frustum: bool,
    material: bool,
    materials: Option<std::path::PathBuf>,
    uvs: Option<std::path::PathBuf>,
    texture: Option<std::path::PathBuf>,
    texture_budget: Option<u64>,
    texture_table: Option<std::path::PathBuf>,
    bundle: Option<std::path::PathBuf>,
    stream_bundle: Option<std::path::PathBuf>,
    bundle_byte_budget: Option<usize>,
    bundle_gpu_budget: Option<u64>,
    lod: Option<(std::path::PathBuf, std::path::PathBuf)>,
    lod_pixels: Option<f32>,
    lod_budget: Option<u64>,
}
impl Options {
    fn render_mode(&self) -> RenderMode {
        if self.lod.is_some() {
            RenderMode::AssetLod
        } else if self.material {
            RenderMode::AssetResolve
        } else if self.frustum {
            RenderMode::AssetCull
        } else if self.camera {
            RenderMode::AssetCamera
        } else {
            self.mode.unwrap_or(RenderMode::Indirect)
        }
    }
    fn parse() -> Result<Self> {
        let mut result = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--network-client" => {
                    result.network_address = Some(
                        args.next()
                            .ok_or("--network-client needs loopback address")?
                            .parse()?,
                    );
                    if result.mode.replace(RenderMode::Authority).is_some() {
                        return Err("Network client is a separate render mode".into());
                    }
                }
                "--network-faults" => result.network_faults = true,
                "--network-repair-test" => result.network_repair_test = true,
                "--network-certificate" => {
                    result.network_certificate = Some(
                        args.next()
                            .ok_or("--network-certificate needs public DER file")?
                            .into(),
                    )
                }
                "--authority-script-reference" => {
                    if args.next().is_some() {
                        return Err("Unexpected script reference argument".into());
                    }
                    println!("{}", authority_demo::script_reference()?);
                    std::process::exit(0);
                }
                "--authority-script" => result.authority_script = true,
                "--authority-alpha" => {
                    result.authority_alpha = Some(
                        args.next()
                            .ok_or("--authority-alpha needs fraction")?
                            .parse()?,
                    )
                }
                "--authority-reference" => {
                    let alpha = args
                        .next()
                        .ok_or("--authority-reference needs fraction")?
                        .parse()?;
                    if args.next().is_some() {
                        return Err("Unexpected authority reference argument".into());
                    }
                    println!("{}", authority::fixture(alpha)?.reference);
                    std::process::exit(0);
                }
                "--frustum-ids" => {
                    let path = args.next().ok_or("--frustum-ids needs mesh")?;
                    let width = args.next().ok_or("--frustum-ids needs width")?.parse()?;
                    let height = args.next().ok_or("--frustum-ids needs height")?.parse()?;
                    let view = args.next().ok_or("--frustum-ids needs view")?.parse()?;
                    let mesh = theia_assets::Mesh::decode(
                        &asset::load(std::path::Path::new(&path))?,
                        None,
                        theia_assets::DEFAULT_BUDGET,
                    )?;
                    println!(
                        "{}",
                        serde_json::json!({"ids":camera::visible(&mesh,camera::view_rows(width,height,view)?)?})
                    );
                    std::process::exit(0);
                }
                "--lod-test" => {
                    result.lod = Some((
                        args.next().ok_or("--lod-test needs fine mesh")?.into(),
                        args.next().ok_or("--lod-test needs coarse mesh")?.into(),
                    ))
                }
                "--lod-budget" => {
                    result.lod_budget =
                        Some(args.next().ok_or("--lod-budget needs bytes")?.parse()?)
                }
                "--lod-pixels" => {
                    result.lod_pixels = Some(
                        args.next()
                            .ok_or("--lod-pixels needs a pixel budget")?
                            .parse()?,
                    )
                }
                "--lod-reference" => {
                    lod_reference::run(&mut args)?;
                    std::process::exit(0);
                }
                "--stream-bundle" => {
                    result.stream_bundle = Some(
                        args.next()
                            .ok_or("--stream-bundle needs replacement manifest")?
                            .into(),
                    )
                }
                "--bundle-byte-budget" => {
                    result.bundle_byte_budget = Some(
                        args.next()
                            .ok_or("--bundle-byte-budget needs bytes")?
                            .parse()?,
                    )
                }
                "--bundle-gpu-budget" => {
                    result.bundle_gpu_budget = Some(
                        args.next()
                            .ok_or("--bundle-gpu-budget needs bytes")?
                            .parse()?,
                    )
                }
                "--bundle" => {
                    result.bundle = Some(
                        args.next()
                            .ok_or("--bundle needs a content-addressed manifest")?
                            .into(),
                    )
                }
                "--camera-test" => result.camera = true,
                "--frustum-test" => result.frustum = true,
                "--material-test" => result.material = true,
                "--texture-table" => {
                    result.material = true;
                    result.texture_table = Some(
                        args.next()
                            .ok_or("--texture-table needs a content-addressed table")?
                            .into(),
                    );
                }
                "--texture-test" => {
                    result.material = true;
                    result.texture = Some(
                        args.next()
                            .ok_or("--texture-test needs a content-addressed KTX2")?
                            .into(),
                    );
                }
                "--texture-budget" => {
                    result.texture_budget =
                        Some(args.next().ok_or("--texture-budget needs bytes")?.parse()?);
                }
                "--uv-test" => {
                    result.material = true;
                    result.uvs = Some(
                        args.next()
                            .ok_or("--uv-test needs a content-addressed UV page")?
                            .into(),
                    );
                }
                "--materials" => {
                    result.material = true;
                    result.materials = Some(
                        args.next()
                            .ok_or("--materials needs a content-addressed material sidecar")?
                            .into(),
                    );
                }
                "--camera-reference"
                | "--frustum-reference"
                | "--material-reference"
                | "--authored-reference"
                | "--uv-reference"
                | "--texture-reference"
                | "--texture-table-reference" => {
                    use std::io::Read;
                    let path = args.next().ok_or(
                        "--camera-reference needs mesh path and x y width height scenario",
                    )?;
                    let authored =
                        if arg == "--authored-reference"
                            || (arg == "--texture-reference" || arg == "--texture-table-reference")
                        {
                            Some(args.next().ok_or(
                                "--authored-reference needs material sidecar after mesh path",
                            )?)
                        } else {
                            None
                        };
                    let uv_path = if arg == "--uv-reference"
                        || (arg == "--texture-reference" || arg == "--texture-table-reference")
                    {
                        Some(
                            args.next()
                                .ok_or("--uv-reference needs UV page after mesh path")?,
                        )
                    } else {
                        None
                    };
                    let texture_path =
                        if arg == "--texture-reference" || arg == "--texture-table-reference" {
                            Some(args.next().ok_or(
                                "--texture-reference needs KTX2 after mesh/material/UV paths",
                            )?)
                        } else {
                            None
                        };
                    let mut values = [0u32; 5];
                    for value in &mut values {
                        *value = args
                            .next()
                            .ok_or("--camera-reference needs x y width height scenario")?
                            .parse()?;
                    }
                    let mut bytes = Vec::new();
                    std::fs::File::open(path)?
                        .take(theia_assets::DEFAULT_BUDGET as u64 + 1)
                        .read_to_end(&mut bytes)?;
                    let mesh =
                        theia_assets::Mesh::decode(&bytes, None, theia_assets::DEFAULT_BUDGET)?;
                    let [x, y, width, height, scenario] = values;
                    let material = arg == "--material-reference";
                    let frustum = arg != "--camera-reference";
                    let mut pixel = camera::pixel(&mesh, x, y, width, height, scenario, frustum)?;
                    if material && pixel.identity[0] != 0 && (pixel.identity[0] - 1) & 1 != 0 {
                        pixel.color[0] *= 89. / 255.;
                        pixel.color[1] *= 153. / 255.;
                    }
                    let mut material_page = None;
                    if let Some(path) = authored {
                        let data = asset::load_material(std::path::Path::new(&path), &mesh)?;
                        let page = theia_assets::material::MaterialPage::decode(
                            &data,
                            None,
                            &mesh,
                            theia_assets::material::DEFAULT_BUDGET,
                        )?;
                        if pixel.identity[0] != 0 {
                            let color = page
                                .color(pixel.identity[0] - 1, pixel.identity[1])
                                .ok_or("Invalid CPU material identity")?;
                            pixel.color.copy_from_slice(&color[..3]);
                        }
                        material_page = Some(page);
                    }
                    let factor = pixel.color;
                    let mut uv_value = None;
                    let mut selected_material = None;
                    let mut selected_texture = None;
                    if let Some(path) = uv_path {
                        let data = asset::load_uv(std::path::Path::new(&path), &mesh)?;
                        let page = theia_assets::uv::UvPage::decode(
                            &data,
                            None,
                            &mesh,
                            theia_assets::uv::DEFAULT_BUDGET,
                        )?;
                        if pixel.covered {
                            let uv: [f32; 2] = std::array::from_fn(|axis| {
                                (0..3)
                                    .map(|i| {
                                        page.texcoords[pixel.vertices[i] as usize][axis]
                                            * pixel.weights[i]
                                    })
                                    .sum()
                            });
                            pixel.color = [
                                (uv[0] * 0.4 + 0.1).clamp(0., 1.),
                                (uv[1] * 0.4 + 0.1).clamp(0., 1.),
                                0.5,
                            ];
                            if let Some(path) = &texture_path {
                                if arg == "--texture-table-reference" {
                                    let materials = material_page
                                        .as_ref()
                                        .ok_or("Texture reference requires material page")?;
                                    let set =
                                        texture::load_set(std::path::Path::new(path), materials)?;
                                    let table = theia_assets::texture_table::TextureTable::decode(
                                        &set.table,
                                        None,
                                        materials,
                                        theia_assets::texture_table::DEFAULT_BUDGET,
                                    )?;
                                    let material = materials.triangle_materials[materials
                                        .triangle_offsets
                                        [(pixel.identity[0] - 1) as usize]
                                        as usize
                                        + pixel.identity[1] as usize];
                                    let index = table
                                        .texture_for(material)
                                        .ok_or("Invalid CPU texture material")?;
                                    let tex = theia_assets::texture::Texture::decode(
                                        &set.images[index as usize],
                                        None,
                                        theia_assets::texture::DEFAULT_BUDGET,
                                    )?;
                                    let sampled = texture::reference_sample(&tex, uv);
                                    pixel.color =
                                        std::array::from_fn(|axis| sampled[axis] * factor[axis]);
                                    selected_material = Some(material);
                                    selected_texture = Some(index);
                                } else {
                                    let bytes = texture::load(std::path::Path::new(path))?;
                                    let tex = theia_assets::texture::Texture::decode(
                                        &bytes,
                                        None,
                                        theia_assets::texture::DEFAULT_BUDGET,
                                    )?;
                                    let sampled = texture::reference_sample(&tex, uv);
                                    pixel.color =
                                        std::array::from_fn(|axis| sampled[axis] * factor[axis]);
                                }
                            }
                            uv_value = Some(uv);
                        }
                    }
                    let ids = if frustum {
                        camera::visible(&mesh, camera::view_rows(width, height, scenario)?)?
                    } else {
                        Vec::new()
                    };
                    println!(
                        "{}",
                        serde_json::json!({"color":pixel.color,"depth":pixel.depth,"covered":pixel.covered,"identity":pixel.identity,"ids":ids,"uv":uv_value,"material_id":selected_material,"texture_index":selected_texture})
                    );
                    std::process::exit(0);
                }
                "--no-validation" => result.no_validation = true,
                "--gpu" => result.gpu = Some(args.next().ok_or("--gpu needs a name substring")?),
                "--frames" => {
                    let count = args
                        .next()
                        .ok_or("--frames needs a positive integer")?
                        .parse()?;
                    if count == 0 {
                        return Err("--frames must be positive".into());
                    }
                    result.frames = Some(count);
                }
                "--mesh-only" | "--clear-only" | "--direct-task" | "--spatial-test"
                | "--visibility-test" | "--authority-test" | "--authority-demo" => {
                    let mode = match arg.as_str() {
                        "--authority-demo" => {
                            result.authority_demo = true;
                            RenderMode::Authority
                        }
                        "--authority-test" => RenderMode::Authority,
                        "--mesh-only" => RenderMode::MeshOnly,
                        "--clear-only" => RenderMode::Clear,
                        "--spatial-test" => RenderMode::Spatial,
                        "--visibility-test" => RenderMode::Visibility,
                        _ => RenderMode::DirectTask,
                    };
                    if result.mode.replace(mode).is_some() {
                        return Err("Regression modes are mutually exclusive".into());
                    }
                }
                "--fallback" => {
                    result.fallback =
                        Some(args.next().ok_or("--fallback needs a cooked mesh")?.into())
                }
                "--asset-budget" => {
                    let bytes: usize = args
                        .next()
                        .ok_or("--asset-budget needs positive encoded bytes")?
                        .parse()?;
                    if bytes == 0 || bytes > theia_assets::DEFAULT_BUDGET {
                        return Err("Asset budget must be 1..64 MiB encoded bytes".into());
                    }
                    result.asset_budget = Some(bytes);
                }
                "--asset" => {
                    if result.mode.replace(RenderMode::Asset).is_some() {
                        return Err("Render modes are mutually exclusive".into());
                    }
                    result.asset = Some(
                        args.next()
                            .ok_or("--asset needs a content-addressed cooked mesh")?
                            .into(),
                    );
                }
                "--cell" => {
                    if result.mode.replace(RenderMode::Asset).is_some() {
                        return Err("Render modes are mutually exclusive".into());
                    }
                    result.cell = Some(
                        args.next()
                            .ok_or("--cell needs a hash-named cell manifest")?
                            .into(),
                    );
                }
                "--resident-budget" => {
                    let bytes: u64 = args
                        .next()
                        .ok_or("--resident-budget needs physical geometry bytes")?
                        .parse()?;
                    if bytes == 0 {
                        return Err("Resident budget must be positive".into());
                    }
                    result.resident_budget = Some(bytes);
                }
                "--work-items" => {
                    let count = args.next().ok_or("--work-items needs 0, 1 or 2")?.parse()?;
                    if count > 2 {
                        return Err("--work-items must be 0, 1 or 2".into());
                    }
                    result.work_items = Some(count);
                }
                "--visibility-reference" => {
                    let mut values = [0u32; 5];
                    for value in &mut values {
                        *value = args
                            .next()
                            .ok_or("--visibility-reference needs x y width height count")?
                            .parse()?;
                    }
                    let [x, y, width, height, count] = values;
                    if width == 0 || height == 0 || x >= width || y >= height || count > 2 {
                        return Err("Invalid visibility reference pixel/extent/count".into());
                    }
                    surface_reference::print_pixel(x, y, width, height, count);
                    std::process::exit(0);
                }
                "--spatial-reference" => {
                    let scenario = args
                        .next()
                        .ok_or("--spatial-reference needs 0, 1 or 2")?
                        .parse()?;
                    if scenario > 2 {
                        return Err("Spatial reference scenario must be 0, 1 or 2".into());
                    }
                    spatial::print_reference(scenario);
                    std::process::exit(0);
                }
                "--shader-dir" => {
                    result.shader_dir =
                        Some(args.next().ok_or("--shader-dir needs a directory")?.into())
                }
                "--smoke-test" => result.smoke = true,
                "--wsi-smoke-test" => {
                    result.smoke = true;
                    result.wsi_smoke = true;
                }
                "--help" => {
                    println!(
                        "Theia Render Lab: GPU task/mesh qualification\n  --network-client <loopback address> --network-certificate <public DER>  live readonly GPU viewer\n  --network-faults  bounded encrypted post-bootstrap packet faults\n  --network-repair-test  qualification ACK withholding until full repair\n  --authority-demo  WASD/arrows movement, Space attack, Escape quit\n  --authority-script  bounded demo intent sequence for qualification\n  --authority-test  fixed server-owned two-tick actor presentation\n  --authority-alpha <0..1>  GPU interpolation fraction (default .5)\n  --authority-reference <0..1>  readonly CPU fixture JSON without Vulkan\n  --gpu <name substring>\n  --frames <positive count>\n  --smoke-test  120 frames, resize/minimize/restore with validation\n  --wsi-smoke-test  smoke plus labeled acquire/present recovery injection\n  --shader-dir <directory>  compiled SPIR-V directory\n  --bundle <file>  admit complete immutable geometry/material/UV/texture set\n  --lod-test <fine coarse>  GPU resident heightfield LOD selection\n  --lod-budget <bytes>  combined physical geometry cap (default 4 MiB)\n  --lod-pixels <pixels>  projected-error budget (default 6)\n  --lod-reference <fine coarse width height view pixels>  CPU projected vertical-error selection\n  --stream-bundle <file>  alternate two complete v2 bundles with bounded I/O/uploads\n  --bundle-byte-budget <bytes>  unique live encoded budget (default 64 MiB)\n  --bundle-gpu-budget <bytes>  active+pending bundle buffers/images (default 32 MiB)\n  --texture-table <file> --uv-test <file> --materials <file>  dense material texture selection\n  --texture-table-reference <mesh material uv table x y width height view>  CPU indexed sampling JSON\n  --texture-test <file> --uv-test <file> --materials <file>  resident opaque sRGB texture\n  --texture-budget <bytes>  physical image allocation cap (default 4 MiB)\n  --texture-reference <mesh material uv texture x y width height view>  CPU sampling JSON\n  --uv-test <file>  fixed cooked TEXCOORD_0 perspective reconstruction\n  --uv-reference <mesh uv x y width height view>  CPU UV/color JSON\n  --materials <file>  opaque unlit sidecar for fixed --asset\n  --authored-reference <mesh material x y width height view>  CPU material JSON\n  --material-test  cooked identity/depth and diagnostic material resolve\n  --material-reference <file x y width height view>  CPU identity/color/depth JSON\n  --frustum-test  GPU conservative meshlet selection for --asset/--cell\n  --frustum-ids <file width height view>  CPU IDs without raster/clipping oracle\n  --frustum-reference <file x y width height view>  CPU pixel and IDs JSON\n  --camera-test  perspective and D32 depth for --asset/--cell\n  --camera-reference <file x y width height scenario>  CPU pixel JSON\n  --cell <file>  M4 two-page asynchronous residency qualification\n  --resident-budget <bytes>  cell geometry allocation budget (default 16 MiB)\n  --asset <file>  M4 cooked immutable meshlet viewer\n  --fallback <file> --asset-budget <encoded bytes>  explicit capacity fallback\n  --visibility-test  M3 identity/depth and material resolve\n  --visibility-reference <x y width height count>  CPU pixel reference JSON\n  --spatial-test  M2 grid/height culling with GPU-generated resident bounds\n  --spatial-reference <0|1|2>  print CPU reference JSON without Vulkan\n  --direct-task  run the qualified direct task amplification path\n  --work-items <0|1|2>  fixed qualification input for compute\n  --mesh-only  run the qualified single-triangle mesh path\n  --clear-only  run the qualified clear-screen path without shaders\n  --no-validation  explicitly disable validation"
                    );
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}").into()),
            }
        }
        if let Some((fine, coarse)) = &result.lod {
            if result.asset.is_some()
                || result.cell.is_some()
                || result.fallback.is_some()
                || result.asset_budget.is_some()
                || result.mode.is_some()
                || result.camera
                || result.frustum
                || result.material
                || result.materials.is_some()
                || result.uvs.is_some()
                || result.texture.is_some()
                || result.texture_table.is_some()
                || result.bundle.is_some()
            {
                return Err("--lod-test owns the two geometry pages; do not combine other render/asset modes".into());
            }
            if result.lod_pixels.is_some_and(|v| !v.is_finite() || v < 0.) {
                return Err("LOD pixel budget must be finite and nonnegative".into());
            }
            lod_reference::load(fine, coarse)?;
            result.asset = Some(fine.clone());
            result.mode = Some(RenderMode::Asset);
        } else if result.lod_pixels.is_some() || result.lod_budget.is_some() {
            return Err("LOD pixels/budget require --lod-test".into());
        }
        if let Some(path) = &result.bundle {
            if result.asset.is_some()
                || result.cell.is_some()
                || result.fallback.is_some()
                || result.asset_budget.is_some()
                || result.materials.is_some()
                || result.uvs.is_some()
                || result.texture.is_some()
                || result.texture_table.is_some()
                || result.mode.is_some()
            {
                return Err("--bundle owns all four asset roles; do not combine explicit asset/cell/fallback/material/UV/texture paths".into());
            }
            let bundle = texture::load_bundle(path)?;
            let [mesh, materials, uvs, texture] = bundle.paths;
            result.asset = Some(mesh);
            result.materials = Some(materials);
            result.uvs = Some(uvs);
            if bundle.indexed {
                result.texture_table = Some(texture);
            } else {
                result.texture = Some(texture);
            }
            result.material = true;
            result.mode = Some(RenderMode::Asset);
        }
        if let Some(path) = &result.stream_bundle {
            if result.bundle.is_none() || result.texture_table.is_none() {
                return Err("--stream-bundle requires an initial v2 --bundle".into());
            }
            if !texture::load_bundle(path)?.indexed {
                return Err("Replacement bundle must use v2 texture table".into());
            }
        } else if result.bundle_byte_budget.is_some() || result.bundle_gpu_budget.is_some() {
            return Err("Bundle residency budgets require --stream-bundle".into());
        }
        if (result.camera || result.frustum || result.material)
            && result.mode != Some(RenderMode::Asset)
        {
            return Err(
                "--camera-test/--frustum-test/--material-test require --asset or --cell".into(),
            );
        }
        if result.uvs.is_some()
            && (result.asset.is_none()
                || result.cell.is_some()
                || result.fallback.is_some()
                || (result.materials.is_some()
                    && result.texture.is_none()
                    && result.texture_table.is_none()))
        {
            return Err(
                "--uv-test requires a fixed --asset without --cell/--fallback/--materials".into(),
            );
        }
        if result.texture.is_some()
            && (result.asset.is_none()
                || result.uvs.is_none()
                || result.materials.is_none()
                || result.cell.is_some()
                || result.fallback.is_some())
        {
            return Err("--texture-test requires fixed --asset plus --uv-test and --materials; no cell/fallback".into());
        }
        if let Some(path) = &result.texture_table {
            if result.texture.is_some()
                || result.asset.is_none()
                || result.materials.is_none()
                || result.uvs.is_none()
                || result.cell.is_some()
                || result.fallback.is_some()
            {
                return Err("--texture-table needs fixed --asset/--materials/--uv-test without single texture/cell/fallback".into());
            }
            if result.bundle.is_none() {
                texture::admit_table(
                    path,
                    result.asset.as_deref().unwrap(),
                    result.materials.as_deref().unwrap(),
                    result.uvs.as_deref().unwrap(),
                )?;
            }
        }
        if result.texture_budget.is_some()
            && result.texture.is_none()
            && result.texture_table.is_none()
        {
            return Err("--texture-budget requires --texture-test or --texture-table".into());
        }
        if result.materials.is_some()
            && (result.asset.is_none() || result.cell.is_some() || result.fallback.is_some())
        {
            return Err("--materials requires a fixed --asset without --cell/--fallback".into());
        }
        if result.resident_budget.is_some() && result.cell.is_none() {
            return Err("--resident-budget requires --cell".into());
        }
        if result.asset.is_none() && (result.fallback.is_some() || result.asset_budget.is_some()) {
            return Err("--fallback/--asset-budget require --asset".into());
        }
        if result.work_items.is_some()
            && (!matches!(
                result.render_mode(),
                RenderMode::Indirect
                    | RenderMode::Spatial
                    | RenderMode::Visibility
                    | RenderMode::Asset
                    | RenderMode::AssetCamera
                    | RenderMode::AssetCull
                    | RenderMode::AssetLod
                    | RenderMode::AssetResolve
            ) || result.smoke)
        {
            return Err(
                "--work-items requires indirect, spatial, visibility or asset mode without --smoke-test (which cycles inputs)"
                    .into(),
            );
        }
        if let Some(network_address) = result.network_address {
            if result.network_certificate.is_none()
                || result.authority_alpha.is_some()
                || result.authority_demo
                || result.authority_script
            {
                return Err("Network client needs its public certificate and owns input/time; do not combine local authority flags".into());
            }
            if !network_address.ip().is_loopback() {
                return Err("Network client fixture must target loopback".into());
            }
        } else if result.network_certificate.is_some()
            || result.network_faults
            || result.network_repair_test
        {
            return Err("Network certificate/fault/repair options require --network-client".into());
        }
        if result.authority_script && !result.authority_demo {
            return Err("--authority-script requires --authority-demo".into());
        }
        if result.authority_demo && result.authority_alpha.is_some() {
            return Err(
                "--authority-demo owns its presentation clock; alpha is only for --authority-test"
                    .into(),
            );
        }
        if result.render_mode() == RenderMode::Authority && result.network_address.is_none() {
            authority::fixture(result.authority_alpha.unwrap_or(0.5))?;
        } else if result.authority_alpha.is_some() {
            return Err("--authority-alpha requires --authority-test".into());
        }
        if result.smoke {
            if result.no_validation || result.frames.is_some() {
                return Err(
                    "--smoke-test requires validation and cannot combine with --frames".into(),
                );
            }
            result.frames = Some(120);
        }
        Ok(result)
    }
}
struct App {
    options: Options,
    validation: Arc<gpu::Validation>,
    window: Option<Arc<Window>>,
    renderer: Option<gpu::Renderer>,
    frames: u64,
    restore: Option<Instant>,
    failure: Option<String>,
    occluded: bool,
    authority_demo: Option<authority_demo::Demo>,
    authority_started: Instant,
    network: Option<network::Viewer>,
    network_title: Option<String>,
}
impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl ToString) {
        self.failure = Some(error.to_string());
        event_loop.exit();
    }
    fn shutdown(&mut self) {
        self.renderer.take();
        self.window.take();
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<_> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title(if self.options.network_address.is_some() {
                            "Theia network viewer — Connecting"
                        } else if self.options.authority_demo {
                            "Theia — local authority demo (WASD/arrows, Space attack)"
                        } else if self.options.render_mode() == RenderMode::Authority {
                            "Theia Render Lab — authoritative snapshot fixture"
                        } else {
                            "Theia Render Lab — M0-D mesh triangle"
                        })
                        .with_inner_size(PhysicalSize::new(960, 540)),
                )?,
            );
            let mut renderer =
                gpu::Renderer::new(window.clone(), &self.options, self.validation.clone())?;
            if let Some(demo) = &self.authority_demo {
                renderer.authority_push(demo.push()?)?;
            }
            if let Some(network) = &self.network {
                renderer.authority_push(network.push()?.0)?;
            }
            Ok((window, renderer))
        })();
        match result {
            Ok((window, renderer)) => {
                self.window = Some(window);
                self.renderer = Some(renderer);
            }
            Err(error) => self.fail(event_loop, error),
        }
    }
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.shutdown();
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|window| window.id() != id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. }
                if event.logical_key == Key::Named(NamedKey::Escape) =>
            {
                event_loop.exit()
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                let binding = match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyA) => Some(0),
                    PhysicalKey::Code(KeyCode::KeyD) => Some(1),
                    PhysicalKey::Code(KeyCode::KeyW) => Some(2),
                    PhysicalKey::Code(KeyCode::KeyS) => Some(3),
                    PhysicalKey::Code(KeyCode::ArrowLeft) => Some(4),
                    PhysicalKey::Code(KeyCode::ArrowRight) => Some(5),
                    PhysicalKey::Code(KeyCode::ArrowUp) => Some(6),
                    PhysicalKey::Code(KeyCode::ArrowDown) => Some(7),
                    _ => None,
                };
                if let Some(binding) = binding {
                    if let Some(demo) = &mut self.authority_demo {
                        demo.direction(binding, pressed);
                    }
                    if let Some(network) = &self.network {
                        network.direction(binding, pressed);
                    }
                } else if event.physical_key == PhysicalKey::Code(KeyCode::Space)
                    && pressed
                    && !event.repeat
                {
                    if let Some(demo) = &mut self.authority_demo {
                        demo.attack();
                    }
                    if let Some(network) = &self.network {
                        network.attack();
                    }
                }
            }
            WindowEvent::Focused(false) => {
                if let Some(demo) = &mut self.authority_demo {
                    demo.release_input();
                }
                if let Some(network) = &self.network {
                    network.release_input();
                }
            }
            WindowEvent::Resized(size) => {
                println!("Window resized: {}x{}", size.width, size.height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.recreate = true;
                }
            }
            WindowEvent::Occluded(value) => self.occluded = value,
            WindowEvent::RedrawRequested => {
                if self.restore.is_some() || self.occluded {
                    return;
                }
                let size = self.window.as_ref().unwrap().inner_size();
                let result = self.renderer.as_mut().unwrap().draw(size);
                match result {
                    Ok(true) => self.frames += 1,
                    Ok(false) => return,
                    Err(error) => {
                        self.fail(event_loop, error);
                        return;
                    }
                }
                if self.options.smoke {
                    let window = self.window.as_ref().unwrap();
                    match self.frames {
                        20 => {
                            let _ = window.request_inner_size(PhysicalSize::new(640, 360));
                        }
                        40 => {
                            let _ = window.request_inner_size(PhysicalSize::new(1280, 720));
                        }
                        60 => {
                            // Exercise zero-size handling explicitly: X11 need not report 0x0 on minimize.
                            match self
                                .renderer
                                .as_mut()
                                .unwrap()
                                .draw(PhysicalSize::new(0, 0))
                            {
                                Ok(false) => println!("Smoke: zero-size draw deferred"),
                                _ => {
                                    self.fail(event_loop, "Zero-size draw was not deferred");
                                    return;
                                }
                            }
                            window.set_minimized(true);
                            self.restore = Some(Instant::now() + Duration::from_millis(300));
                            println!("Smoke: requested minimize");
                        }
                        _ => {}
                    }
                }
                if self
                    .options
                    .frames
                    .is_some_and(|limit| self.frames >= limit)
                {
                    if let Err(error) = self.renderer.as_ref().unwrap().qualification_complete() {
                        self.fail(event_loop, error);
                        return;
                    }
                    if let Some(demo) = &self.authority_demo
                        && let Err(error) = demo.verify()
                    {
                        self.fail(event_loop, error);
                        return;
                    }
                    if let Some(network) = &self.network
                        && let Err(error) = network.verify()
                    {
                        self.fail(event_loop, error);
                        return;
                    }
                    println!("Presented {} frames", self.frames);
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(demo) = &mut self.authority_demo {
            let result = demo
                .advance(self.authority_started.elapsed())
                .and_then(|()| demo.push());
            match result {
                Ok(bytes) => {
                    if let Some(renderer) = &mut self.renderer
                        && let Err(error) = renderer.authority_push(bytes)
                    {
                        self.fail(event_loop, error);
                        return;
                    }
                }
                Err(error) => {
                    self.fail(event_loop, error);
                    return;
                }
            }
        }
        if let Some(network) = &self.network {
            match network.push() {
                Ok((bytes, title)) => {
                    if let Some(renderer) = &mut self.renderer
                        && let Err(error) = renderer.authority_push(bytes)
                    {
                        self.fail(event_loop, error);
                        return;
                    }
                    if self.network_title.as_ref() != Some(&title) {
                        if let Some(window) = &self.window {
                            window.set_title(&format!("Theia network viewer — {title}"));
                        }
                        self.network_title = Some(title);
                    }
                }
                Err(error) => {
                    self.fail(event_loop, error);
                    return;
                }
            }
        }
        let authority_deadline = self
            .authority_demo
            .as_ref()
            .and_then(|demo| self.authority_started.checked_add(demo.next_tick()));
        if let Some(deadline) = self.restore {
            if Instant::now() < deadline {
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    authority_deadline.map_or(deadline, |tick| tick.min(deadline)),
                ));
                return;
            }
            self.restore = None;
            self.occluded = false;
            if let Some(window) = &self.window {
                window.set_minimized(false);
                let _ = window.request_inner_size(PhysicalSize::new(800, 600));
                println!("Smoke: requested restore");
            }
            if let Some(renderer) = &mut self.renderer {
                renderer.recreate = true;
            }
        }
        event_loop
            .set_control_flow(authority_deadline.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
        if !self.occluded
            && let Some(window) = &self.window
        {
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                window.request_redraw();
            }
        }
    }
    fn exiting(&mut self, _: &ActiveEventLoop) {
        self.shutdown();
    }
}
fn main() -> Result<()> {
    let options = Options::parse()?;
    let validation = Arc::new(gpu::Validation::default());
    let authority_demo = options
        .authority_demo
        .then(|| authority_demo::Demo::new(options.authority_script))
        .transpose()?;
    let network = options
        .network_address
        .map(|address| {
            network::Viewer::new(
                address,
                options.network_certificate.as_deref().unwrap(),
                options.network_faults,
                options.network_repair_test,
            )
        })
        .transpose()?;
    let mut app = App {
        options,
        validation: validation.clone(),
        window: None,
        renderer: None,
        frames: 0,
        restore: None,
        failure: None,
        occluded: false,
        authority_demo,
        authority_started: Instant::now(),
        network,
        network_title: None,
    };
    let result = EventLoop::new()?.run_app(&mut app);
    app.shutdown();
    let (warnings, errors) = validation.counts();
    println!("Validation totals (including teardown): {warnings} warnings, {errors} errors");
    result?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    if errors > 0 {
        return Err("Vulkan validation reported errors".into());
    }
    Ok(())
}
