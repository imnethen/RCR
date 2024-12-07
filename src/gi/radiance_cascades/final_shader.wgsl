struct uniform_data {
    c0_rays: u32,
    c0_spacing: f32,
    c0_raylength: f32,

    angular_scaling: u32,
    spatial_scaling: f32,

    num_cascades: u32,
    cur_cascade: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: uniform_data;
@group(0) @binding(1)
var temp_texture: texture_2d<f32>;
@group(0) @binding(2)
var out_texture: texture_storage_2d<rgba32float, write>;

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let temp_texture_dims = textureDimensions(temp_texture);
    let out_texture_dims = textureDimensions(out_texture);

    let out_pos = id.xy;
    let id1d = id.x + id.y * out_texture_dims.x;
    let temp_pos = vec2u(id1d % temp_texture_dims.x, id1d / temp_texture_dims.x);

    let color = textureLoad(temp_texture, temp_pos, 0);

    textureStore(out_texture, out_pos, color);
}
