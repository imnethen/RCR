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
fn fs_main(in: VertexOutput) -> @location(0) vec2f {
    // in_texture is the same size as the jfa temp textures because thats how the temp textures are created
    let pixel_pos = vec2u(in.tex_coord * vec2f(textureDimensions(in_texture)));

    let pixel_color = textureLoad(in_texture, pixel_pos, 0);
    if (pixel_color.a < 0.001) {
        return vec2f(nonexistent_coord);
    } else {
        return vec2f(pixel_pos);
    }
}
