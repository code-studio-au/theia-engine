//! Inspect a content-addressed complete authored mip chain without Vulkan/native code.
use std::{error::Error, io::Read, path::Path};
use theia_assets::{content_id, texture::DEFAULT_BUDGET, texture_mips::TextureMipChain};
fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("texture_mips needs <sha256.ktx2>")?;
    let mut bytes = Vec::new();
    std::fs::File::open(&path)?
        .take(DEFAULT_BUDGET as u64 + 1)
        .read_to_end(&mut bytes)?;
    let hash = content_id(&bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect::<String>();
    if Path::new(&path).file_stem().and_then(|v| v.to_str()) != Some(&hash) {
        return Err("Mip filename/content digest mismatch".into());
    }
    let chain = TextureMipChain::decode(&bytes, None, DEFAULT_BUDGET)?;
    for (index, mip) in chain.levels.iter().enumerate() {
        println!(
            "Verified mip {index}: {}x{}, {} opaque RGBA8 sRGB bytes",
            mip.width,
            mip.height,
            mip.texels.len()
        );
    }
    println!("Writer: {}", chain.writer);
    println!(
        "Provenance: {}",
        chain.provenance.ok_or("Missing cooked provenance")?
    );
    Ok(())
}
