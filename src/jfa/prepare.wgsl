const nonexistent_coord: f32 = -2e9;

@group(0) @binding(0)
var in_texture: texture_2d<f32>;
@group(1) @binding(1)
var out_texture: texture_storage_2d<rg16float, write>;

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let color = textureLoad(in_texture, id.xy, 0);
    if color.a < 0.001 {
        textureStore(out_texture, id.xy, vec4f(vec2f(nonexistent_coord), 0, 0));
    } else {
        textureStore(out_texture, id.xy, vec4f(vec2f(id.xy), 0, 0));
    }
}
