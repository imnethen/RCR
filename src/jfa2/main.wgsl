var<push_constant> stepsize: u32;

@group(0) @binding(0)
var in_texture: texture_2d<i32>;
@group(0) @binding(1)
var out_texture: texture_storage_2d<rg32sint, write>;

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let in_dims = textureDimensions(in_texture);
    // let pixel_pos = vec2u(in.tex_coord * vec2f(in_dims));
    let pixel_pos = id.xy;

    var best_dist: f32 = 1e9;
    var best_pos: vec2i = vec2i(-1);

    for (var x: i32 = -1; x <= 1; x += 1) {
        for (var y: i32 = -1; y <= 1; y += 1) {
            let pos = vec2i(pixel_pos) + i32(stepsize) * vec2i(x, y);
            if out_of_bounds(pos, in_dims) {
                continue;
            }
            let closest = textureLoad(in_texture, pos, 0).xy;
            if closest.x == -1 {
                continue;
            }

            let dist = distance(vec2f(pixel_pos), vec2f(closest));
            if dist < best_dist {
                best_dist = dist;
                best_pos = closest;
            }
        }
    }

    textureStore(out_texture, pixel_pos, vec4i(best_pos, 0, 0));
}

fn out_of_bounds(pos: vec2i, dims: vec2u) -> bool {
    return (pos.x < 0 || pos.y < 0 || pos.x >= i32(dims.x) || pos.y >= i32(dims.y));
}
