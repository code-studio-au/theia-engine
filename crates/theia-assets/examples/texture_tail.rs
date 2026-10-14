//! Inspect a bounded resident mip-tail plan without Vulkan or partial file I/O.
use std::{error::Error, io::Read, path::Path};
use theia_assets::{
    content_id, texture::DEFAULT_BUDGET, texture_mips::TextureMipChain, texture_residency::MipTail,
};
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("texture_tail needs <sha256.ktx2> <texel-budget> <requested-mip>")?;
    let budget = args.next().ok_or("missing texel budget")?.parse()?;
    let requested = args.next().ok_or("missing requested mip")?.parse()?;
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
    let tail = MipTail::fit(&chain, budget)?;
    let selection = tail.select(requested)?;
    println!(
        "{{\"first_level\":{},\"texel_bytes\":{},\"resident_extent\":{:?},\"source_level\":{},\"view_level\":{},\"coarsened\":{}}}",
        tail.first_level(),
        tail.texel_bytes(),
        tail.resident_extent(),
        selection.source_level,
        selection.view_level,
        selection.coarsened
    );
    Ok(())
}
