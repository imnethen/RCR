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
    color: vec3f,
    radius: f32,
    texture_size: vec2f,
    pos: vec2f,
}

@group(0) @binding(0)
var<uniform> uniforms: uniform_data;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let pixel_pos = in.tex_coord * uniforms.texture_size;
    let dist = distance(pixel_pos, uniforms.pos);

    if (dist <= uniforms.radius) {
        return vec4f(uniforms.color, 1.);
    } else {
        return vec4f(0.);
    }
}
