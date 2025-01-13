struct uniform_data {
    color: vec3f,
    shape: u32, // 0 = square, 1 = circle
    pos: vec2u,
    radius: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: uniform_data;

@group(1) @binding(0)
var out_texture: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3u) {
    if (uniforms.shape == 0) {
        let pixel_pos = id.xy + uniforms.pos;
        textureStore(out_texture, pixel_pos, vec4f(uniforms.color, 1.));
    } else {
        let pixel_pos = id.xy + uniforms.pos - vec2u(uniforms.radius);
        let center = vec2f(uniforms.pos);
        let dist = distance(vec2f(pixel_pos), center);
        if (dist <= uniforms.radius) {
            textureStore(out_texture, pixel_pos, vec4f(uniforms.color, 1.));
        }
    }
}
