//! Validate a material-bound texture table and all immutable KTX2 children.
use std::{error::Error, io::Read, path::Path};
use theia_assets::{Mesh, content_id, material::MaterialPage, texture_table::TextureTable};
fn read(path: &Path, budget: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(budget as u64 + 1)
        .read_to_end(&mut bytes)?;
    let hash = content_id(&bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect::<String>();
    if bytes.len() > budget || path.file_stem().and_then(|v| v.to_str()) != Some(&hash) {
        return Err("Texture set file budget/hash filename mismatch".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("texture_table needs <mesh> <materials> <texture-table>".into());
    }
    let mesh = Mesh::decode(
        &read(Path::new(&args[0]), theia_assets::DEFAULT_BUDGET)?,
        None,
        theia_assets::DEFAULT_BUDGET,
    )?;
    let materials = MaterialPage::decode(
        &read(Path::new(&args[1]), theia_assets::material::DEFAULT_BUDGET)?,
        None,
        &mesh,
        theia_assets::material::DEFAULT_BUDGET,
    )?;
    let table = TextureTable::decode(
        &read(
            Path::new(&args[2]),
            theia_assets::texture_table::DEFAULT_BUDGET,
        )?,
        None,
        &materials,
        theia_assets::texture_table::DEFAULT_BUDGET,
    )?;
    let mut bytes = Vec::new();
    for dep in &table.textures {
        let hex = dep
            .content_id
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>();
        bytes.push(read(
            &Path::new(&args[2]).parent().unwrap().join(hex + ".ktx2"),
            dep.bytes as usize,
        )?);
    }
    let slices: Vec<_> = bytes.iter().map(Vec::as_slice).collect();
    let textures = table.validate_textures(&slices)?;
    println!(
        "Verified complete resident texture table: {} materials, {} hash-sorted opaque textures, mappings {:?}",
        materials.base_colors.len(),
        textures.len(),
        table.material_textures
    );
    Ok(())
}
