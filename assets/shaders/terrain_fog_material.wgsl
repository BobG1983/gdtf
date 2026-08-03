// Terrain fog-of-war material + multi-level storey darken.
//
// Renders one atlas terrain tile on a unit-rect Mesh2d, with two independent per-instance
// knobs:
//   `saturation` — 1.0 = full colour (a squad-VISIBLE cell), 0.0 = full greyscale at the
//     SAME luminance (a squad-EXPLORED / "was visible" cell — colour-loss as the memory cue).
//   `brightness` — 1.0 = full brightness (the active view storey), < 1.0 = dimmed (a lower,
//     drawn-but-non-active storey in the UFO:EU / OpenXcom multi-level display).
// The fragment samples the tile, computes its BT.709 luminance, mixes toward grey by
// (1 - saturation), THEN scales the result by `brightness` — so the fog colour-loss and the
// storey-depth darken COMPOSE (a lower EXPLORED tile is greyscaled AND dimmed). UNSEEN cells
// are hidden by the presenter via Visibility, not here.
//
// Mirrors bevy_sprite_render's sprite_material.wgsl vertex shape (the same mesh2d
// functions + VERTEX_* shader-defs the Mesh2d pipeline sets from the mesh attributes), so
// the unit Rectangle mesh feeds it correctly. The atlas tile is selected CPU-side: the
// uv_transform uniform already bakes the atlas rect, so the fragment just samples through
// it.

#import bevy_sprite::{
    mesh2d_functions as mesh_functions,
    mesh2d_vertex_output::VertexOutput,
}

struct TerrainFogMaterial {
    // Maps the unit-rect UV to the atlas tile's UV rect.
    uv_transform: mat3x3<f32>,
    // The quad size in world units (scales the unit rect in the vertex stage).
    vertex_scale: vec2<f32>,
    // 1.0 = full colour, 0.0 = full greyscale (the fog colour-loss axis).
    saturation: f32,
    // 1.0 = full brightness (active storey), < 1.0 = dimmed lower drawn storey.
    // Fills the trailing pad byte; the scalars pack exactly to 16 bytes.
    brightness: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: TerrainFogMaterial;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var tile_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var tile_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_POSITIONS
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let position = vec4<f32>(vertex.position * vec3<f32>(material.vertex_scale, 1.0), 1.0);
    out.world_position = mesh_functions::mesh2d_position_local_to_world(world_from_local, position);
    out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
#endif

#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh2d_normal_local_to_world(vertex.normal, vertex.instance_index);
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh2d_tangent_local_to_world(world_from_local, vertex.tangent);
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

    return out;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = (material.uv_transform * vec3<f32>(mesh.uv, 1.0)).xy;
    let sampled = textureSample(tile_texture, tile_sampler, uv);

    // Mask: discard the transparent atlas margin (rather than blend a halo).
    if sampled.a < 0.5 {
        discard;
    }

    // BT.709 luminance — the perceptual grey at the SAME brightness as the tile.
    let luma = dot(sampled.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let grey = vec3<f32>(luma, luma, luma);
    // saturation 1.0 -> full colour; 0.0 -> full greyscale (the fog colour-loss axis).
    let coloured = mix(grey, sampled.rgb, clamp(material.saturation, 0.0, 1.0));
    // THEN scale by the storey-depth brightness (1.0 active / < 1.0 lower drawn
    // storey) — a SEPARATE axis that COMPOSES on top of the saturation mix, never replacing
    // it (a lower EXPLORED tile is both greyscaled and dimmed).
    let rgb = coloured * clamp(material.brightness, 0.0, 1.0);

    return vec4<f32>(rgb, sampled.a);
}
