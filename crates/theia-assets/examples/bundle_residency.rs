//! CPU qualification using real hash-named complete bundles. No GPU work occurs.
use std::{collections::BTreeMap, error::Error, io::Read, path::Path};
use theia_assets::{
    bundle::{BYTES, Bundle, BundleProfile},
    bundle_residency::{Residency, WorldResidency},
    content_id,
};

struct Input {
    manifest: Vec<u8>,
    roots: Vec<Vec<u8>>,
    leaves: Vec<Vec<u8>>,
}
fn hex(id: [u8; 32]) -> String {
    id.iter().map(|v| format!("{v:02x}")).collect()
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("resource exceeds encoded budget".into());
    }
    Ok(bytes)
}
impl Input {
    fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let manifest = read(path, BYTES)?;
        if path.file_stem().and_then(|s| s.to_str()) != Some(&hex(content_id(&manifest))) {
            return Err("bundle filename hash mismatch".into());
        }
        let bundle = Bundle::decode(&manifest, None)?;
        let parent = path.parent().ok_or("missing bundle directory")?;
        let extensions = [
            "theia-mesh",
            "theia-material",
            "theia-uv",
            if bundle.profile == BundleProfile::SingleTexture {
                "ktx2"
            } else {
                "theia-textures"
            },
        ];
        let mut roots = Vec::new();
        for (dep, extension) in bundle.dependencies.iter().zip(extensions) {
            roots.push(read(
                &parent.join(format!("{}.{extension}", hex(dep.content_id))),
                dep.bytes as usize,
            )?);
        }
        let mut leaves = Vec::new();
        if bundle.profile == BundleProfile::TextureTable {
            let admitted =
                bundle.validate_table_roots(std::array::from_fn(|i| roots[i].as_slice()))?;
            for dep in admitted.table.textures {
                leaves.push(read(
                    &parent.join(hex(dep.content_id) + ".ktx2"),
                    dep.bytes as usize,
                )?);
            }
        }
        Ok(Self {
            manifest,
            roots,
            leaves,
        })
    }
    fn stage_world(
        &self,
        store: &mut WorldResidency,
    ) -> Result<theia_assets::bundle_residency::StagedBundle, Box<dyn Error>> {
        Ok(store.stage(
            &self.manifest,
            content_id(&self.manifest),
            std::array::from_fn(|i| self.roots[i].as_slice()),
            &self.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        )?)
    }
    fn sizes(&self) -> BTreeMap<[u8; 32], usize> {
        std::iter::once(&self.manifest)
            .chain(&self.roots)
            .chain(&self.leaves)
            .map(|v| (content_id(v), v.len()))
            .collect()
    }
    fn stage(
        &self,
        store: &mut Residency,
    ) -> Result<theia_assets::bundle_residency::StagedBundle, Box<dyn Error>> {
        Ok(store.stage(
            &self.manifest,
            content_id(&self.manifest),
            std::array::from_fn(|i| self.roots[i].as_slice()),
            &self.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        )?)
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<_> = std::env::args().skip(1).collect();
    if paths.first().is_some_and(|v| v == "--world") {
        return qualify_world(&paths[1..]);
    }
    if paths.len() != 2 {
        return Err("bundle_residency needs <first bundle> <second bundle>".into());
    }
    let a = Input::load(Path::new(&paths[0]))?;
    let b = Input::load(Path::new(&paths[1]))?;
    let asizes = a.sizes();
    let bsizes = b.sizes();
    let abytes: usize = asizes.values().sum();
    let bbytes: usize = bsizes.values().sum();
    let mut union = asizes.clone();
    union.extend(&bsizes);
    let total: usize = union.values().sum();
    if total <= abytes.max(bbytes) {
        return Err("qualification needs two non-contained complete sets".into());
    }
    let mut store = Residency::new(total)?;
    let staged = a.stage(&mut store)?;
    let duplicate = a.stage(&mut store)?;
    assert_eq!(store.live_bytes(), abytes);
    drop(duplicate);
    let first = store.publish(Some(staged))?;
    let staged = b.stage(&mut store)?;
    assert_eq!(store.live_bytes(), total);
    let second = store.publish(Some(staged))?;
    for (id, size) in &asizes {
        assert_eq!(first.resource(id).unwrap().len(), *size);
    }
    for (id, size) in &bsizes {
        assert_eq!(second.resource(id).unwrap().len(), *size);
    }
    assert_eq!(second.epoch(), 2);
    drop(first);
    assert_eq!(store.live_bytes(), bbytes);
    store.publish(None)?;
    assert_eq!(store.live_bytes(), bbytes);
    drop(second);
    assert_eq!(store.live_bytes(), 0);
    let mut bounded = Residency::new(abytes.max(bbytes))?;
    let staged = a.stage(&mut bounded)?;
    let reader = bounded.publish(Some(staged))?;
    assert!(b.stage(&mut bounded).is_err());
    assert_eq!(bounded.snapshot().epoch(), 1);
    assert_eq!(bounded.live_bytes(), abytes);
    bounded.publish(None)?;
    assert!(b.stage(&mut bounded).is_err());
    drop(reader);
    let staged = b.stage(&mut bounded)?;
    assert_eq!(bounded.publish(Some(staged))?.epoch(), 3);
    println!(
        "Verified complete CPU bundle residency: first={abytes}, second={bbytes}, union={total}, shared={}, resources={}/{}, retained-reader rejection and last-owner reuse",
        abytes + bbytes - total,
        asizes.len(),
        bsizes.len()
    );
    Ok(())
}

fn qualify_world(paths: &[String]) -> Result<(), Box<dyn Error>> {
    if paths.len() < 2 || paths.len() > theia_assets::bundle_residency::MAX_WORLD_CHUNKS {
        return Err("--world needs 2..4096 complete bundle paths".into());
    }
    let inputs = paths
        .iter()
        .map(|p| Input::load(Path::new(p)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut union = BTreeMap::new();
    let mut separate = 0usize;
    for input in &inputs {
        let sizes = input.sizes();
        separate += sizes.values().sum::<usize>();
        union.extend(sizes);
    }
    let total = union.values().sum::<usize>();
    let mut store = WorldResidency::new(total, inputs.len())?;
    let staged = inputs
        .iter()
        .enumerate()
        .map(|(i, input)| input.stage_world(&mut store).map(|s| (i as u64, s)))
        .collect::<Result<Vec<_>, _>>()?;
    let reader = store.publish(staged, &[])?;
    assert_eq!(reader.chunk_count(), inputs.len());
    assert_eq!(reader.epoch(), 1);
    assert_eq!(store.live_bytes(), total);
    for (index, input) in inputs.iter().enumerate() {
        let chunk = reader.chunk(index as u64).unwrap();
        assert_eq!(chunk.bundle_id(), Some(content_id(&input.manifest)));
        for (id, length) in input.sizes() {
            assert_eq!(chunk.resource(&id).unwrap().len(), length);
        }
    }
    let reduced = store.publish(vec![], &[0])?;
    assert!(reduced.chunk(0).is_none());
    for i in 1..inputs.len() {
        assert_eq!(reduced.chunk(i as u64).unwrap().epoch(), 1);
    }
    let replacement = inputs[1].stage_world(&mut store)?;
    let replaced = store.publish(vec![(0, replacement)], &[])?;
    assert_eq!(
        replaced.chunk(0).unwrap().bundle_id(),
        reader.chunk(1).unwrap().bundle_id()
    );
    assert_eq!(replaced.chunk(0).unwrap().epoch(), 3);
    assert_eq!(store.live_bytes(), total);
    drop(reduced);
    drop(replaced);
    let ids = reader.chunk_ids().collect::<Vec<_>>();
    store.publish(vec![], &ids)?;
    assert_eq!(store.snapshot().chunk_count(), 0);
    assert_eq!(store.live_bytes(), total);
    drop(reader);
    assert_eq!(store.live_bytes(), 0);
    // An independent byte-union oracle predicts the first short-budget failure.
    let mut short = WorldResidency::new(total - 1, inputs.len())?;
    let mut expected = BTreeMap::new();
    let mut retained = Vec::new();
    let mut rejected = false;
    for input in &inputs {
        expected.extend(input.sizes());
        let stage = input.stage_world(&mut short);
        if expected.values().sum::<usize>() > total - 1 {
            assert!(stage.is_err());
            rejected = true;
            break;
        }
        retained.push(stage?);
    }
    assert!(rejected);
    assert_eq!(short.snapshot().chunk_count(), 0);
    drop(retained);
    assert_eq!(short.live_bytes(), 0);
    println!(
        "Verified independent complete CPU world bundles: chunks={}, unique_bytes={total}, separate_bytes={separate}, shared_bytes={}, resources={}, exact/short budgets and retained-reader eviction",
        inputs.len(),
        separate - total,
        union.len()
    );
    Ok(())
}
