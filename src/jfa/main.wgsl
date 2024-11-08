struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) tex_coord: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    var poses = array(
        vec2f(0., 1.),
        vec2f(1., 1.),
        vec2f(0., 0.),
        vec2f(1., 0.),
    );

    let pos = vec2f(poses[vid].x * 2. - 1., -poses[vid].y * 2. + 1.);
    return VertexOutput(vec4f(pos, 0, 1), poses[vid]);
}

struct uniform_data {
    stepsize: u32
}

@group(0) @binding(0)
var<uniform> uniforms: uniform_data;
@group(0) @binding(1)
var in_texture: texture_2d<i32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec2i {
    let in_dims = textureDimensions(in_texture);
    let pixel_pos = vec2u(in.tex_coord * vec2f(in_dims));

    var best_dist: f32 = 1e9;
    var best_pos: vec2i = vec2i(-1);

    for (var x: i32 = -1; x <= 1; x += 1) {
        for (var y: i32 = -1; y <= 1; y += 1) {
            let pos = vec2i(pixel_pos) + i32(uniforms.stepsize) * vec2i(x, y);
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

    return best_pos;
}

fn out_of_bounds(pos: vec2i, dims: vec2u) -> bool {
    return (pos.x < 0 || pos.y < 0 || pos.x >= i32(dims.x) || pos.y >= i32(dims.y));
}
