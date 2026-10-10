//! File-backed CPU lifetime qualification; no GPU upload/completion claims.
use std::{
    error::Error,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use theia_assets::{
    content_id,
    residency::{Manifest, Residency},
};
fn read(path: &Path, limit: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("file exceeds bounded read size".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn Error>> {
    let path = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: residency <cell-file>")?,
    );
    let bytes = read(&path, 24 + 4096 * 40)?;
    let id = content_id(&bytes);
    let name: String = id.iter().map(|v| format!("{v:02x}")).collect();
    if path.file_stem().and_then(|v| v.to_str()) != Some(name.as_str()) {
        return Err("manifest filename differs from content address".into());
    }
    let manifest = Manifest::decode(&bytes, id)?;
    let mut pages: Vec<_> = manifest.pages().iter().collect();
    pages.sort_by_key(|(_, size)| **size);
    if pages.len() != 2 {
        return Err("qualification requires two distinct mesh pages".into());
    }
    let (small_id, small_size) = pages[0];
    let (large_id, large_size) = pages[1];
    let directory = path.parent().ok_or("missing page directory")?;
    let load = |id: &[u8; 32], limit: usize| {
        let name: String = id.iter().map(|v| format!("{v:02x}")).collect();
        read(&directory.join(format!("{name}.theia-mesh")), limit)
    };
    let large = load(large_id, *large_size)?;
    let small = load(small_id, *small_size)?;
    let mut store = Residency::new(*large_size)?;
    let staged = store.stage(&manifest, *large_id, &large)?;
    let retained = store.publish(Some(staged), &[])?;
    store.publish(None, &[*large_id])?;
    assert!(store.stage(&manifest, *small_id, &small).is_err());
    assert_eq!(retained.page(large_id), Some(large.as_slice()));
    println!(
        "Cell {}: retained epoch {} protects {} bytes after eviction",
        manifest.cell(),
        retained.epoch(),
        store.live_bytes()
    );
    drop(retained);
    assert_eq!(store.live_bytes(), 0);
    let staged = store.stage(&manifest, *small_id, &small)?;
    let published = store.publish(Some(staged), &[])?;
    assert_eq!(published.page(small_id), Some(small.as_slice()));
    assert_eq!(store.live_bytes(), *small_size);
    println!(
        "Released reader; epoch {} publishes {} bytes within {}-byte encoded-page budget",
        published.epoch(),
        store.live_bytes(),
        large_size
    );
    Ok(())
}
