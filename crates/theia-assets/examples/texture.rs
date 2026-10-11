//! Independently inspect a content-addressed bounded KTX2 texture.
use std::{error::Error, io::Read, path::Path};
use theia_assets::{
    content_id,
    texture::{DEFAULT_BUDGET, Texture},
};
fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("texture needs <sha256.ktx2>")?;
    let mut bytes = Vec::new();
    std::fs::File::open(&path)?
        .take(DEFAULT_BUDGET as u64 + 1)
        .read_to_end(&mut bytes)?;
    let hash = content_id(&bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect::<String>();
    if Path::new(&path).file_stem().and_then(|v| v.to_str()) != Some(&hash) {
        return Err("Texture filename/content hash mismatch".into());
    }
    let texture = Texture::decode(&bytes, None, DEFAULT_BUDGET)?;
    println!(
        "Verified KTX2: {}x{}, {} opaque RGBA8 sRGB texel bytes; writer {}",
        texture.width,
        texture.height,
        texture.texels.len(),
        texture.writer
    );
    println!(
        "Provenance: {}",
        texture.provenance.ok_or("Missing cooked provenance")?
    );
    Ok(())
}
