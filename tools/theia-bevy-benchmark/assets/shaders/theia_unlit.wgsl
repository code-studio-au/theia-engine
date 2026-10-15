#import bevy_pbr::{forward_io::VertexOutput, mesh_view_bindings::view}
struct Surface { factor: vec4<f32>, options: vec4<f32> }
#ifdef BINDLESS
#import bevy_render::bindless::{bindless_textures_2d, bindless_samplers_filtering}
#import bevy_pbr::mesh_bindings::mesh
struct SurfaceIndices { surface: u32, texture: u32, sampler: u32 }
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage> surface_indices: array<SurfaceIndices>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var<storage> surfaces: array<Surface>;
#else
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> surface: Surface;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var color_sampler: sampler;
#endif
@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
#ifdef BINDLESS
    let slot = mesh[input.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    let indices = surface_indices[slot];
    let surface = surfaces[indices.surface];
    var color = textureSampleLevel(bindless_textures_2d[indices.texture], bindless_samplers_filtering[indices.sampler], input.uv, 0.0) * surface.factor;
#else
    var color = textureSampleLevel(color_texture, color_sampler, input.uv, 0.0) * surface.factor;
#endif
#ifdef VERTEX_COLORS
    color *= input.color;
#endif
    if surface.options.x > 0.0 {
        let distance = -(view.view_from_world * input.world_position).z;
        color = vec4<f32>(mix(color.rgb, vec3<f32>(0.12,0.23,0.35), clamp(distance/180.0,0.0,0.7)), 1.0);
    }
    return color;
}
