//! Readonly bilinear sampling of a content-addressed authored mip chain.
use std::{error::Error, io::Read, path::Path};
use theia_assets::{content_id, texture::DEFAULT_BUDGET, texture_mips::TextureMipChain};
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("texture_filter needs <sha256.ktx2> <mip> <u> <v>")?;
    let mip: usize = args.next().ok_or("missing mip")?.parse()?;
    let uv = [
        args.next().ok_or("missing u")?.parse()?,
        args.next().ok_or("missing v")?.parse()?,
    ];
    if args.next().is_some() {
        return Err("unexpected trailing argument".into());
    }
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
    chain.provenance.ok_or("Missing cooked provenance")?;
    let level = chain.levels.get(mip).ok_or("Mip outside complete chain")?;
    let color = theia_assets::texture_filter::linear_repeat(level, uv)?;
    println!("{{\"mip_level\":{mip},\"linear_rgb\":{:?}}}", color);
    Ok(())
}
