//! Validate independently cooked opaque material bindings against immutable mesh bytes.
use std::{error::Error, io::Read, path::Path};
use theia_assets::{Mesh, content_id, material::MaterialPage};
fn read(path: &str, budget: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(budget as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > budget
        || Path::new(path).file_stem().and_then(|v| v.to_str())
            != Some(
                &content_id(&bytes)
                    .iter()
                    .map(|v| format!("{v:02x}"))
                    .collect::<String>(),
            )
    {
        return Err("File exceeds budget or content-addressed filename mismatch".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: material <hash.theia-mesh> <hash.theia-material>".into());
    }
    let mesh = Mesh::decode(
        &read(&args[0], theia_assets::DEFAULT_BUDGET)?,
        None,
        theia_assets::DEFAULT_BUDGET,
    )?;
    let page = MaterialPage::decode(
        &read(&args[1], theia_assets::material::DEFAULT_BUDGET)?,
        None,
        &mesh,
        theia_assets::material::DEFAULT_BUDGET,
    )?;
    for (id, meshlet) in mesh.meshlets.iter().enumerate() {
        for triangle in 0..meshlet.triangle_count {
            println!(
                "meshlet {id}, triangle {triangle}: {:?}",
                page.color(id as u32, triangle).unwrap()
            );
        }
    }
    println!(
        "Material binding validated: {} opaque factors / {} triangles",
        page.base_colors.len(),
        page.triangle_materials.len()
    );
    Ok(())
}
