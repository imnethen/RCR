struct uniform_data {
    color: vec3f,
    pos: vec2u,
}

@group(0) @binding(0)
var<uniform> uniforms: uniform_data;

@group(1) @binding(0)
var out_texture: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let pixel_pos = id.xy + uniforms.pos;
    textureStore(out_texture, pixel_pos, vec4f(uniforms.color, 1.));
}
