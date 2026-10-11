//! CPU projected-error reference for the qualified source-linked heightfield pair.
use crate::{Result, asset, camera};
pub fn run(args: &mut impl Iterator<Item = String>) -> Result<()> {
    let fine_path = args.next().ok_or("LOD reference needs fine mesh")?;
    let coarse_path = args.next().ok_or("LOD reference needs coarse mesh")?;
    let width = args.next().ok_or("LOD reference needs width")?.parse()?;
    let height = args.next().ok_or("LOD reference needs height")?.parse()?;
    let view = args.next().ok_or("LOD reference needs view")?.parse()?;
    let budget = args
        .next()
        .ok_or("LOD reference needs pixel budget")?
        .parse()?;
    let decode = |path: &str| -> Result<theia_assets::Mesh> {
        Ok(theia_assets::Mesh::decode(
            &asset::load(std::path::Path::new(path))?,
            None,
            theia_assets::DEFAULT_BUDGET,
        )?)
    };
    let fine = decode(&fine_path)?;
    let coarse = decode(&coarse_path)?;
    let source: serde_json::Value = serde_json::from_str(&fine.provenance)?;
    let metadata: serde_json::Value = serde_json::from_str(&coarse.provenance)?;
    let proof = &metadata["lod_qualification"];
    let triangles = |mesh: &theia_assets::Mesh| {
        mesh.meshlets
            .iter()
            .map(|v| u64::from(v.triangle_count))
            .sum::<u64>()
    };
    if !source["source_sha256"]
        .as_str()
        .is_some_and(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        || metadata["fallback_for_source_sha256"] != source["source_sha256"]
        || proof["original_triangles"].as_u64() != Some(triangles(&fine))
        || proof["simplified_triangles"].as_u64() != Some(triangles(&coarse))
        || proof["exact_boundary_preserved"].as_bool() != Some(true)
        || proof["continuous_profile"].as_str()
            != Some(
                "exact represented positions; continuous rectangular XY heightfields; vertical error only",
            )
        || fine.positions.len() != coarse.positions.len()
        || fine
            .positions
            .iter()
            .flatten()
            .zip(coarse.positions.iter().flatten())
            .any(|(a, b)| a.to_bits() != b.to_bits())
    {
        return Err("LOD reference requires source-linked continuous heightfield pages with exact common positions/counts".into());
    }
    let upper = proof["continuous_vertical_error_upper_bound"]
        .as_f64()
        .ok_or("LOD page has no declared continuous error bound")?;
    let error = theia_assets::lod::error_f32(upper)?;
    let mut bounds = [[f32::INFINITY; 3], [f32::NEG_INFINITY; 3]];
    for p in &fine.positions {
        for axis in 0..3 {
            bounds[0][axis] = bounds[0][axis].min(p[axis]);
            bounds[1][axis] = bounds[1][axis].max(p[axis]);
        }
    }
    let selection = theia_assets::lod::select(
        bounds,
        error,
        camera::view_rows(width, height, view)?,
        [width, height],
        budget,
    )?;
    println!(
        "{}",
        serde_json::json!({"fine_id":fine.content_id.iter().map(|v|format!("{v:02x}")).collect::<String>(),"coarse_id":coarse.content_id.iter().map(|v|format!("{v:02x}")).collect::<String>(),
    "geometry_bindings_verified":true,"proof_recomputed":false,"represented_vertical_error":error,
    "bound_pixels":selection.bound_pixels,"pixel_budget":budget,"level":format!("{:?}",selection.level),"reason":selection.reason.map(|v|format!("{v:?}"))})
    );
    Ok(())
}
