const nonexistent_coord: f32 = -2e9;

@group(0) @binding(0)
var in_texture: texture_2d<f32>;
@group(1) @binding(0)
var out_texture: texture_storage_2d<r16float, write>;

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let pixel_pos = id.xy;
    let closest_pos = textureLoad(in_texture, pixel_pos, 0).xy;
    var dist = 1e9;
    if closest_pos.x != nonexistent_coord {
        dist = distance(closest_pos, vec2f(pixel_pos));
        dist = max(dist - 0.5 * sqrt(2.), 0.01);
    }
    textureStore(out_texture, id.xy, vec4f(dist, 0., 0., 0.));
}
