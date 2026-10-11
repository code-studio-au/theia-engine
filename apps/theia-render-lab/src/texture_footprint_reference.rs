//! Readonly CPU UV-gradient/footprint oracle over the qualified cooked scene.
use crate::{Result, asset, camera, texture};
use std::path::Path;
pub fn run(args: &mut impl Iterator<Item = String>) -> Result<()> {
    let mesh_path = args.next().ok_or("Footprint reference needs mesh path")?;
    let uv_path = args.next().ok_or("Footprint reference needs UV path")?;
    let texture_path = args
        .next()
        .ok_or("Footprint reference needs mip texture path")?;
    let mut values = [0u32; 5];
    for value in &mut values {
        *value = args
            .next()
            .ok_or("Footprint reference needs x y width height view")?
            .parse()?;
    }
    let [x, y, width, height, view] = values;
    let geometry = asset::load(Path::new(&mesh_path))?;
    let mesh = theia_assets::Mesh::decode(&geometry, None, theia_assets::DEFAULT_BUDGET)?;
    let bytes = asset::load_uv(Path::new(&uv_path), &mesh)?;
    let uvs =
        theia_assets::uv::UvPage::decode(&bytes, None, &mesh, theia_assets::uv::DEFAULT_BUDGET)?;
    let bytes = texture::load_mips(Path::new(&texture_path))?;
    let chain = theia_assets::texture_mips::TextureMipChain::decode(
        &bytes,
        None,
        theia_assets::texture::DEFAULT_BUDGET,
    )?;
    let pixel = camera::pixel(&mesh, x, y, width, height, view, true)?;
    let gradient = if pixel.covered {
        let rows = camera::view_rows(width, height, view)?;
        let clip = pixel
            .vertices
            .map(|vertex| camera::project(rows, mesh.positions[vertex as usize]));
        let uv = pixel.vertices.map(|vertex| uvs.texcoords[vertex as usize]);
        let differential = theia_assets::texture_footprint::differential(
            clip,
            uv,
            [f64::from(x) + 0.5, f64::from(y) + 0.5],
            [width, height],
        )?;
        let footprint = theia_assets::texture_footprint::choose_mip(
            differential,
            [chain.width, chain.height],
            chain.levels.len() as u32,
        )?;
        Some(
            serde_json::json!({"uv":differential.uv,"dx":differential.dx,"dy":differential.dy,
            "texels_per_pixel":footprint.texels_per_pixel,"mip_level":footprint.mip_level}),
        )
    } else {
        None
    };
    println!(
        "{}",
        serde_json::json!({"covered":pixel.covered,"identity":pixel.identity,"gradient":gradient,
        "texture_dimensions":[chain.width,chain.height],"mip_count":chain.levels.len(),
        "gpu_selection":false,"policy":"ceil(log2(max Jacobian column length in base texels)), clamped"})
    );
    Ok(())
}
