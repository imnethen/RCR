const nonexistent_coord: f32 = -2e9;

var<push_constant> stepsize: u32;

@group(0) @binding(0)
var in_texture: texture_2d<f32>;
@group(0) @binding(1)
var out_texture: texture_storage_2d<rg32float, write>;

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let in_dims = textureDimensions(in_texture);
    let pixel_pos = id.xy;

    var best_dist: f32 = 2e9;
    var best_pos: vec2f = vec2f(nonexistent_coord);

    for (var x: i32 = -1; x <= 1; x += 1) {
        for (var y: i32 = -1; y <= 1; y += 1) {
            let pos = vec2i(pixel_pos) + i32(stepsize) * vec2i(x, y);
            if out_of_bounds(pos, in_dims) {
                continue;
            }
            let closest: vec2f = textureLoad(in_texture, pos, 0).xy;

            let diff = closest - vec2f(pixel_pos);
            let dist = (diff.x * diff.x) + (diff.y * diff.y);
            if dist < best_dist {
                best_dist = dist;
                best_pos = closest;
            }
        }
    }

    textureStore(out_texture, pixel_pos, vec4f(best_pos, 0., 0.));
}

fn out_of_bounds(pos: vec2i, dims: vec2u) -> bool {
    return (pos.x < 0 || pos.y < 0 || pos.x >= i32(dims.x) || pos.y >= i32(dims.y));
}
