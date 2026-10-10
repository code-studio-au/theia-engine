//! Offline bounded static geometry and optional unlit factor normalization.
use base64::Engine;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    io::Read,
    path::{Component, Path},
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_VERTICES: usize = 8192;
const MAX_INDICES: usize = 6144;
type Matrix = [[f32; 4]; 4];
const IDENTITY: Matrix = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0., 0., 0., 1.],
];
fn hex(bytes: &[u8]) -> String {
    theia_assets::content_id(bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}
fn read(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err("Source file exceeds 64 MiB limit".into());
    }
    Ok(bytes)
}
fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|column| {
        std::array::from_fn(|row| (0..4).map(|k| a[k][row] * b[column][k]).sum())
    })
}
fn determinant(m: Matrix) -> f32 {
    m[0][0] * (m[1][1] * m[2][2] - m[2][1] * m[1][2])
        - m[1][0] * (m[0][1] * m[2][2] - m[2][1] * m[0][2])
        + m[2][0] * (m[0][1] * m[1][2] - m[1][1] * m[0][2])
}
struct Scene {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
    nodes: Vec<Value>,
    visited: BTreeSet<usize>,
    unlit: bool,
    base_colors: Vec<[f32; 4]>,
    material_indices: BTreeMap<usize, u32>,
    triangle_materials: Vec<u32>,
}
impl Scene {
    fn material(&mut self, material: gltf::Material<'_>) -> Result<u32> {
        let index = material
            .index()
            .ok_or("Unlit import requires an explicit authored material")?;
        let pbr = material.pbr_metallic_roughness();
        let factor = pbr.base_color_factor();
        if !material.unlit()
            || !material.double_sided()
            || material.alpha_mode() != gltf::material::AlphaMode::Opaque
            || pbr.base_color_texture().is_some()
            || factor[3] != 1.
            || factor
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || material.extensions().is_some_and(|v| !v.is_empty())
            || pbr.extensions().is_some_and(|v| !v.is_empty())
        {
            return Err("Unlit import requires double-sided opaque factor-only KHR_materials_unlit; textures/other extensions unsupported".into());
        }
        if let Some(&id) = self.material_indices.get(&index) {
            return Ok(id);
        }
        if self.base_colors.len() >= 256 {
            return Err("Unlit material count exceeds 256".into());
        }
        let id = self.base_colors.len() as u32;
        self.base_colors.push(factor);
        self.material_indices.insert(index, id);
        Ok(id)
    }
    fn visit(
        &mut self,
        node: gltf::Node<'_>,
        parent: Matrix,
        buffers: &[Vec<u8>],
        depth: usize,
    ) -> Result<()> {
        if depth > 128 || self.visited.len() >= 4096 || !self.visited.insert(node.index()) {
            return Err("Scene exceeds bounded tree limits or repeats a node".into());
        }
        if node.skin().is_some() || node.weights().is_some() {
            return Err("Skin/morph nodes are unsupported by static position importer".into());
        }
        if node.extensions().is_some_and(|v| !v.is_empty()) {
            return Err("Node extensions are unsupported".into());
        }
        let local = node.transform();
        if let gltf::scene::Transform::Decomposed { rotation, .. } = &local {
            let norm: f32 = rotation.iter().map(|value| value * value).sum();
            if !norm.is_finite() || (norm - 1.).abs() > 1e-5 {
                return Err("Node rotation is not a unit quaternion".into());
            }
        }
        let world = multiply(parent, local.matrix());
        if world.iter().flatten().any(|v| !v.is_finite())
            || [world[0][3], world[1][3], world[2][3], world[3][3]] != [0., 0., 0., 1.]
        {
            return Err("Nonfinite or nonaffine node transform".into());
        }
        let determinant = determinant(world);
        if !determinant.is_finite() || determinant.abs() < 1e-10 {
            return Err("Singular node transform".into());
        }
        if let Some(mesh) = node.mesh() {
            if mesh.extensions().is_some_and(|v| !v.is_empty()) {
                return Err("Mesh extensions are unsupported".into());
            }
            if mesh.weights().is_some() {
                return Err("Morph meshes are unsupported".into());
            }
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles
                    || primitive.morph_targets().next().is_some()
                    || (!self.unlit && primitive.material().index().is_some())
                    || primitive.extensions().is_some_and(|v| !v.is_empty())
                    || primitive
                        .attributes()
                        .any(|(kind, _)| kind != gltf::Semantic::Positions)
                {
                    return Err(
                        "Only position-only triangle geometry in the selected material mode is supported".into(),
                    );
                }
                let material = if self.unlit {
                    Some(self.material(primitive.material())?)
                } else {
                    None
                };
                let count = primitive
                    .get(&gltf::Semantic::Positions)
                    .ok_or("Missing POSITION accessor")?
                    .count();
                if count == 0
                    || count > MAX_VERTICES - self.positions.len()
                    || primitive
                        .indices()
                        .is_some_and(|indices| indices.count() > MAX_INDICES - self.indices.len())
                {
                    return Err("Imported accessor count exceeds geometry limits".into());
                }
                let reader = primitive.reader(|buffer| Some(buffers[buffer.index()].as_slice()));
                let positions: Vec<_> = reader
                    .read_positions()
                    .ok_or("Missing float3 POSITION data")?
                    .collect();
                if positions.is_empty() || self.positions.len() + positions.len() > MAX_VERTICES {
                    return Err("Imported vertex limit exceeded".into());
                }
                let mut indices: Vec<u32> = reader
                    .read_indices()
                    .map(|values| values.into_u32().collect())
                    .unwrap_or_else(|| (0..positions.len() as u32).collect());
                if indices.is_empty()
                    || !indices.len().is_multiple_of(3)
                    || self.indices.len() + indices.len() > MAX_INDICES
                    || indices.iter().any(|v| *v as usize >= positions.len())
                {
                    return Err("Invalid or excessive imported triangle indices".into());
                }
                let base = self.positions.len() as u32;
                let first_index = self.indices.len();
                for p in positions {
                    let transformed: [f32; 3] = std::array::from_fn(|row| {
                        world[0][row] * p[0]
                            + world[1][row] * p[1]
                            + world[2][row] * p[2]
                            + world[3][row]
                    });
                    let normalized = [transformed[0], -transformed[2], transformed[1]];
                    if normalized.iter().any(|v| !v.is_finite()) {
                        return Err("Nonfinite transformed position".into());
                    }
                    self.positions.push(normalized);
                }
                if determinant < 0. {
                    for triangle in indices.as_chunks_mut::<3>().0 {
                        triangle.swap(1, 2);
                    }
                }
                self.indices
                    .extend(indices.iter().map(|index| base + index));
                if let Some(id) = material {
                    self.triangle_materials
                        .extend(std::iter::repeat_n(id, indices.len() / 3));
                }
                let mut provenance = json!({"node_index":node.index(),"name":node.name(),"mesh_index":mesh.index(),"primitive_index":primitive.index(),
                    "vertex_offset":base,"vertex_count":self.positions.len()-base as usize,"index_offset":first_index,"index_count":indices.len(),"reflected":determinant<0.});
                if let Some(id) = material {
                    provenance["normalized_material_id"] = json!(id);
                    provenance["source_material_index"] = json!(primitive.material().index());
                }
                self.nodes.push(provenance);
            }
        }
        for child in node.children() {
            self.visit(child, world, buffers, depth + 1)?;
        }
        Ok(())
    }
}
fn normalize(path: &Path) -> Result<Value> {
    normalize_mode(path, false)
}
fn normalize_mode(path: &Path, unlit: bool) -> Result<Value> {
    let source = read(path)?;
    let gltf = gltf::Gltf::from_slice(&source)?;
    if gltf
        .document
        .extensions_used()
        .any(|extension| !unlit || extension != "KHR_materials_unlit")
        || gltf
            .document
            .extensions_required()
            .any(|extension| !unlit || extension != "KHR_materials_unlit")
        || gltf.document.animations().next().is_some()
        || gltf.document.cameras().next().is_some()
    {
        return Err(
            "Unsupported extension, animation or authored camera in the static prototype".into(),
        );
    }
    if unlit && gltf.materials().len() > 256 {
        return Err("Unlit source material count exceeds 256".into());
    }
    let directory = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()?;
    let mut buffers = Vec::new();
    let mut dependencies = Vec::new();
    let mut total = 0usize;
    for buffer in gltf.buffers() {
        if buffer.length() > MAX_BYTES {
            return Err("Declared buffer exceeds 64 MiB limit".into());
        }
        let is_bin = matches!(buffer.source(), gltf::buffer::Source::Bin);
        let data = match buffer.source() {
            gltf::buffer::Source::Bin => gltf
                .blob
                .as_ref()
                .ok_or("GLB buffer has no binary chunk")?
                .clone(),
            gltf::buffer::Source::Uri(uri) => {
                if let Some(encoded) = uri
                    .strip_prefix("data:application/octet-stream;base64,")
                    .or_else(|| uri.strip_prefix("data:application/gltf-buffer;base64,"))
                {
                    if encoded.len() > MAX_BYTES * 2 {
                        return Err("Embedded buffer exceeds limit".into());
                    }
                    base64::engine::general_purpose::STANDARD.decode(encoded)?
                } else {
                    let relative = Path::new(uri);
                    if uri.contains([':', '%', '?', '#', '\\'])
                        || !relative
                            .components()
                            .all(|v| matches!(v, Component::Normal(_) | Component::CurDir))
                    {
                        return Err("Buffer URI must be an unescaped relative local path".into());
                    }
                    let resolved = directory.join(relative).canonicalize()?;
                    if !resolved.starts_with(&directory) {
                        return Err("Buffer resolves outside source directory".into());
                    }
                    read(&resolved)?
                }
            }
        };
        if data.len() < buffer.length()
            || data.len() > buffer.length() + if is_bin { 3 } else { 0 }
            || data.len() > MAX_BYTES
            || data[buffer.length()..].iter().any(|v| *v != 0)
        {
            return Err("Buffer length/padding mismatch".into());
        }
        total = total
            .checked_add(data.len())
            .ok_or("Buffer size overflow")?;
        if total > MAX_BYTES {
            return Err("Total buffer bytes exceed 64 MiB".into());
        }
        dependencies
            .push(json!({"buffer_index":buffer.index(),"bytes":data.len(),"sha256":hex(&data)}));
        buffers.push(data);
    }
    for accessor in gltf.accessors() {
        if accessor.sparse().is_some() || accessor.normalized() {
            return Err("Sparse/normalized accessors are unsupported".into());
        }
        let view = accessor.view().ok_or("Accessor has no buffer view")?;
        let view_end = view
            .offset()
            .checked_add(view.length())
            .ok_or("Buffer-view overflow")?;
        if view_end > buffers[view.buffer().index()].len() || view_end > view.buffer().length() {
            return Err("Buffer view exceeds actual bytes".into());
        }
        let stride = view.stride().unwrap_or(accessor.size());
        let end = accessor
            .count()
            .checked_sub(1)
            .and_then(|count| count.checked_mul(stride))
            .and_then(|bytes| bytes.checked_add(accessor.offset()))
            .and_then(|bytes| bytes.checked_add(accessor.size()))
            .ok_or("Accessor size overflow")?;
        if stride < accessor.size() || end > view.length() {
            return Err("Accessor exceeds view/stride".into());
        }
    }
    let scene = if let Some(scene) = gltf.default_scene() {
        scene
    } else {
        let mut scenes = gltf.scenes();
        let scene = scenes.next().ok_or("No authored scene")?;
        if scenes.next().is_some() {
            return Err("Multiple scenes require explicit default scene".into());
        }
        scene
    };
    if scene.extensions().is_some_and(|v| !v.is_empty()) {
        return Err("Scene extensions are unsupported".into());
    }
    let mut normalized = Scene {
        positions: Vec::new(),
        indices: Vec::new(),
        nodes: Vec::new(),
        visited: BTreeSet::new(),
        unlit,
        base_colors: Vec::new(),
        material_indices: BTreeMap::new(),
        triangle_materials: Vec::new(),
    };
    for node in scene.nodes() {
        normalized.visit(node, IDENTITY, &buffers, 0)?;
    }
    if normalized.indices.is_empty() {
        return Err("Selected scene has no supported geometry".into());
    }
    for triangle in normalized.indices.as_chunks::<3>().0 {
        let [a, b, c] = triangle.map(|index| normalized.positions[index as usize]);
        let ab: [f32; 3] = std::array::from_fn(|axis| b[axis] - a[axis]);
        let ac: [f32; 3] = std::array::from_fn(|axis| c[axis] - a[axis]);
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let area_squared: f32 = cross.into_iter().map(|value| value * value).sum();
        if !area_squared.is_finite() || area_squared <= 0. {
            return Err("Degenerate or excessive imported triangle area".into());
        }
    }
    let mut result = json!({"schema":1,"positions":normalized.positions,"indices":normalized.indices,
        "provenance":{"producer":format!("theia-gltf-import-{}/gltf-1.4.1",env!("CARGO_PKG_VERSION")),"source_sha256":hex(&source),"buffers":dependencies,
            "compiler":env!("THEIA_IMPORT_RUSTC"),"target_os":std::env::consts::OS,"target_arch":std::env::consts::ARCH,
            "debug_assertions":cfg!(debug_assertions),"normalizer_binary_sha256":hex(&read(&std::env::current_exe()?)?),
            "normalizer_source_sha256":hex(include_bytes!("main.rs")),"cargo_lock_sha256":hex(include_bytes!("../Cargo.lock")),"cargo_manifest_sha256":hex(include_bytes!("../Cargo.toml")),
            "scene_index":scene.index(),"nodes":normalized.nodes,"coordinates":"RH Z-up meters; glTF (x,y,z) -> (x,-z,y)","reflections":"swap triangle indices 1/2 for negative world determinant"}});
    if unlit {
        result["schema"] = json!(2);
        result["base_colors"] = json!(normalized.base_colors);
        result["triangle_materials"] = json!(normalized.triangle_materials);
        result["provenance"]["shading"] =
            json!("opaque double-sided KHR_materials_unlit factors; no textures/vertex attributes");
    }
    Ok(result)
}
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let input = args
        .next()
        .ok_or("usage: theia-gltf-import <source.gltf/glb> <normalized.json> [--unlit-factors]")?;
    let output = args.next().ok_or("normalized output path required")?;
    let unlit = match args.next().as_deref() {
        None => false,
        Some("--unlit-factors") => true,
        _ => return Err("Only optional --unlit-factors is supported".into()),
    };
    if args.next().is_some() {
        return Err("Too many arguments".into());
    }
    let normalized = if unlit {
        normalize_mode(Path::new(&input), true)?
    } else {
        normalize(Path::new(&input))?
    };
    std::fs::write(&output, serde_json::to_vec(&normalized)?)?;
    println!(
        "Normalized {} vertices / {} triangles into {output}",
        normalized["positions"].as_array().unwrap().len(),
        normalized["indices"].as_array().unwrap().len() / 3
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Value, Vec<u8>) {
        let mut bytes = Vec::new();
        for p in [[-0.4f32, 0.5, 0.4], [0.4, 0.5, 0.4], [0., 0.5, -0.4]] {
            for value in p {
                bytes.extend(value.to_le_bytes());
            }
        }
        for i in [0u16, 1, 2] {
            bytes.extend(i.to_le_bytes());
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        (
            json!({"asset":{"version":"2.0"},"buffers":[{"uri":format!("data:application/octet-stream;base64,{encoded}"),"byteLength":bytes.len()}],
            "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-0.4,0.5,-0.4],"max":[0.4,0.5,0.4]},
                {"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0}),
            bytes,
        )
    }
    fn run(document: &Value) -> Result<Value> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("scene.gltf");
        std::fs::write(&path, serde_json::to_vec(document)?)?;
        normalize(&path)
    }
    #[test]
    fn nested_rotation_translation_and_reflection_bake_into_z_up() {
        let (mut document, _) = fixture();
        let q = 0.5f64.sqrt();
        document["nodes"] = json!([{"children":[1],"translation":[0.1,0.05,0.],"rotation":[0.,q,0.,q]},
            {"mesh":0,"translation":[0.,0.,0.2],"scale":[-1.,1.,1.]}]);
        let result = run(&document).unwrap();
        for (actual, expected) in result["positions"].as_array().unwrap().iter().zip([
            [0.7, 0.4, 0.55],
            [0.7, -0.4, 0.55],
            [-0.1, 0., 0.55],
        ]) {
            for axis in 0..3 {
                assert!((actual[axis].as_f64().unwrap() - expected[axis]).abs() < 1e-6);
            }
        }
        assert_eq!(result["indices"], json!([0, 2, 1]));
        assert_eq!(result["provenance"]["nodes"][0]["reflected"], true);
    }
    #[test]
    fn separate_instances_get_distinct_dense_ranges() {
        let (mut document, _) = fixture();
        document["nodes"] = json!([{"mesh":0},{"mesh":0,"translation":[0.6,0.,0.]}]);
        document["scenes"][0]["nodes"] = json!([0, 1]);
        let result = run(&document).unwrap();
        assert_eq!(result["indices"], json!([0, 1, 2, 3, 4, 5]));
        for i in 0..3 {
            assert!(
                (result["positions"][i + 3][0].as_f64().unwrap()
                    - result["positions"][i][0].as_f64().unwrap()
                    - 0.6)
                    .abs()
                    < 1e-6
            );
        }
        assert_eq!(result["provenance"]["nodes"][1]["vertex_offset"], 3);
    }
    #[test]
    fn invalid_ranges_features_tree_and_uris_fail_before_decode() {
        let (original, _) = fixture();
        let mut cases = Vec::new();
        let mut v = original.clone();
        v["bufferViews"][0]["byteOffset"] = json!(20);
        cases.push(v);
        let mut v = original.clone();
        v["accessors"][0]["byteOffset"] = json!(32);
        cases.push(v);
        let mut v = original.clone();
        v["buffers"][0]["uri"] = json!("../outside.bin");
        cases.push(v);
        let mut v = original.clone();
        v["nodes"][0]["children"] = json!([0]);
        cases.push(v);
        let mut v = original.clone();
        v["nodes"][0]["scale"] = json!([0., 1., 1.]);
        cases.push(v);
        let mut v = original.clone();
        v["nodes"][0]["rotation"] = json!([0., 0., 0., 2.]);
        cases.push(v);
        let mut v = original.clone();
        v["materials"] = json!([{}]);
        v["meshes"][0]["primitives"][0]["material"] = json!(0);
        cases.push(v);
        let mut v = original.clone();
        v["meshes"][0]["primitives"][0]["mode"] = json!(1);
        cases.push(v);
        let mut v = original.clone();
        v["scenes"][0]["nodes"] = json!([0, 0]);
        cases.push(v);
        for (index, case) in cases.iter().enumerate() {
            assert!(run(case).is_err(), "case {index}");
        }
    }
    #[test]
    fn unlit_factors_follow_source_primitives_through_instance_reflections() {
        let (mut document, _) = fixture();
        document["extensionsUsed"] = json!(["KHR_materials_unlit"]);
        document["extensionsRequired"] = json!(["KHR_materials_unlit"]);
        document["materials"] = json!([
            {"doubleSided":true,"extensions":{"KHR_materials_unlit":{}},"pbrMetallicRoughness":{"baseColorFactor":[0.8,0.1,0.05,1.0]}},
            {"doubleSided":true,"extensions":{"KHR_materials_unlit":{}},"pbrMetallicRoughness":{"baseColorFactor":[0.05,0.65,0.2,1.0]}}
        ]);
        document["meshes"][0]["primitives"][0]["material"] = json!(1);
        document["meshes"]
            .as_array_mut()
            .unwrap()
            .push(json!({"primitives":[{"attributes":{"POSITION":0},"indices":1,"material":0}]}));
        document["nodes"] =
            json!([{"mesh":0},{"mesh":1,"scale":[-1.,1.,1.]},{"mesh":0,"translation":[0.4,0.,0.]}]);
        document["scenes"][0]["nodes"] = json!([0, 1, 2]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scene.gltf");
        std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
        assert!(normalize(&path).is_err());
        let result = normalize_mode(&path, true).unwrap();
        assert_eq!(result["schema"], json!(2));
        assert_eq!(result["triangle_materials"], json!([0, 1, 0]));
        let green = result["base_colors"][0][1].as_f64().unwrap();
        assert!((green - 0.65).abs() < 1e-6);
        assert_eq!(result["indices"], json!([0, 1, 2, 3, 5, 4, 6, 7, 8]));
        assert_eq!(
            result["provenance"]["nodes"][1]["source_material_index"],
            json!(0)
        );
    }
    #[test]
    fn unlit_mode_rejects_unsupported_alpha_faces_textures_attributes_and_extensions() {
        let (mut source, _) = fixture();
        source["extensionsUsed"] = json!(["KHR_materials_unlit"]);
        source["materials"] = json!([{"doubleSided":true,"extensions":{"KHR_materials_unlit":{}},"pbrMetallicRoughness":{"baseColorFactor":[0.8,0.1,0.05,1.0]}}]);
        source["meshes"][0]["primitives"][0]["material"] = json!(0);
        let mut cases = Vec::new();
        let mut v = source.clone();
        v["materials"][0]["alphaMode"] = json!("BLEND");
        cases.push(v);
        let mut v = source.clone();
        v["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"][3] = json!(0.5);
        cases.push(v);
        let mut v = source.clone();
        v["materials"][0]["doubleSided"] = json!(false);
        cases.push(v);
        let mut v = source.clone();
        v["materials"][0]
            .as_object_mut()
            .unwrap()
            .remove("extensions");
        cases.push(v);
        let mut v = source.clone();
        v["meshes"][0]["primitives"][0]
            .as_object_mut()
            .unwrap()
            .remove("material");
        cases.push(v);
        let mut v = source.clone();
        v["images"] = json!([{"uri":"unloaded.png"}]);
        v["textures"] = json!([{"source":0}]);
        v["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index":0});
        cases.push(v);
        let mut v = source.clone();
        v["meshes"][0]["primitives"][0]["attributes"]["NORMAL"] = json!(0);
        cases.push(v);
        let mut v = source.clone();
        v["extensionsRequired"] = json!(["KHR_materials_transmission"]);
        cases.push(v);
        let mut v = source.clone();
        v["nodes"][0]["extensions"] = json!({"KHR_materials_unlit":{}});
        cases.push(v);
        let mut v = source.clone();
        v["materials"] =
            json!(std::iter::repeat_n(source["materials"][0].clone(), 257).collect::<Vec<_>>());
        cases.push(v);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scene.gltf");
        for (i, case) in cases.iter().enumerate() {
            std::fs::write(&path, serde_json::to_vec(case).unwrap()).unwrap();
            assert!(normalize_mode(&path, true).is_err(), "case {i}");
        }
    }
    #[test]
    fn external_buffers_and_padded_glb_preserve_normalized_geometry() {
        let (mut document, bytes) = fixture();
        let expected = run(&document).unwrap();
        let directory = tempfile::tempdir().unwrap();
        document["buffers"][0]["uri"] = json!("mesh.bin");
        std::fs::write(directory.path().join("mesh.bin"), &bytes).unwrap();
        let external = directory.path().join("scene.gltf");
        std::fs::write(&external, serde_json::to_vec(&document).unwrap()).unwrap();
        let loaded = normalize(&external).unwrap();
        assert_eq!(loaded["positions"], expected["positions"]);
        assert_eq!(loaded["indices"], expected["indices"]);
        document["buffers"][0]
            .as_object_mut()
            .unwrap()
            .remove("uri");
        let mut json = serde_json::to_vec(&document).unwrap();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let mut bin = bytes;
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let mut glb = b"glTF".to_vec();
        glb.extend(2u32.to_le_bytes());
        glb.extend((12u32 + 8 + json.len() as u32 + 8 + bin.len() as u32).to_le_bytes());
        glb.extend((json.len() as u32).to_le_bytes());
        glb.extend(b"JSON");
        glb.extend(json);
        glb.extend((bin.len() as u32).to_le_bytes());
        glb.extend(b"BIN\0");
        glb.extend(bin);
        let path = directory.path().join("scene.glb");
        std::fs::write(&path, glb).unwrap();
        let loaded = normalize(&path).unwrap();
        assert_eq!(loaded["positions"], expected["positions"]);
        assert_eq!(loaded["indices"], expected["indices"]);
    }
}
