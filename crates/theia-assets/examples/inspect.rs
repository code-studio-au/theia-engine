use std::{error::Error, fs, io::Read};
use theia_assets::{DEFAULT_BUDGET, Mesh};
fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or("inspect needs a mesh path")?;
    if fs::metadata(&path)?.len() > DEFAULT_BUDGET as u64 {
        return Err("encoded asset budget exceeded".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(DEFAULT_BUDGET as u64 + 1)
        .read_to_end(&mut bytes)?;
    let mesh = Mesh::decode(&bytes, None, DEFAULT_BUDGET)?;
    println!(
        "Verified immutable mesh: {} bytes, {} vertices, {} meshlets, {} triangles",
        bytes.len(),
        mesh.positions.len(),
        mesh.meshlets.len(),
        mesh.triangle_count()
    );
    println!("Producer: {}", mesh.provenance);
    Ok(())
}
