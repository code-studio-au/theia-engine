//! Independently validate a complete immutable textured mesh dependency set.
use std::{error::Error, io::Read, path::Path};
use theia_assets::{
    bundle::{BYTES, Bundle},
    content_id,
};
fn read(path: &Path, budget: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(budget as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > budget {
        return Err("Bundle/child exceeds encoded budget".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("bundle needs <sha256.theia-bundle>")?;
    let path = Path::new(&path);
    let data = read(path, BYTES)?;
    let hex = |id: [u8; 32]| id.iter().map(|v| format!("{v:02x}")).collect::<String>();
    if path.file_stem().and_then(|v| v.to_str()) != Some(&hex(content_id(&data))) {
        return Err("Bundle filename hash mismatch".into());
    }
    let bundle = Bundle::decode(&data, None)?;
    let extensions = ["theia-mesh", "theia-material", "theia-uv", "ktx2"];
    let mut children = Vec::new();
    for (dep, extension) in bundle.dependencies.iter().zip(extensions) {
        children.push(read(
            &path
                .parent()
                .unwrap()
                .join(format!("{}.{extension}", hex(dep.content_id))),
            dep.bytes as usize,
        )?);
    }
    let validated = bundle.validate(std::array::from_fn(|i| children[i].as_slice()))?;
    println!(
        "Verified complete opaque texture bundle: {} vertices, {} triangles, {} materials, {} UV rows, {}x{} texture",
        validated.mesh.positions.len(),
        validated.mesh.triangle_count(),
        validated.materials.base_colors.len(),
        validated.uvs.texcoords.len(),
        validated.texture.width,
        validated.texture.height
    );
    Ok(())
}
