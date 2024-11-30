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

const nonexistent_coord: f32 = -2e9;

@group(0) @binding(0)
var in_texture: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let pixel_pos = vec2u(in.tex_coord * vec2f(textureDimensions(in_texture)));
    let closest_pos = textureLoad(in_texture, pixel_pos, 0).xy;
    var dist = 1e9;
    if closest_pos.x != nonexistent_coord {
        dist = distance(closest_pos, vec2f(pixel_pos));
        dist = max(dist - 0.5 * sqrt(2.), 0.01);
    }
    return vec4f(dist, vec3f(0.));
}
