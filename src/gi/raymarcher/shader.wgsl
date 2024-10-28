const tau = 6.283185307179586;

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
    ray_count: u32
}

@group(0) @binding(0)
var<uniform> uniforms: uniform_data;

@group(1) @binding(0)
var in_texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

fn march_ray(start_pos: vec2f, dir: vec2f) -> vec4f {
    let texel = vec2f(1.) / vec2f(textureDimensions(in_texture));
    var pos = start_pos;

    for (var step = 0u; step < 128u; step += 1u) {
        let color = textureSample(in_texture, texture_sampler, pos * texel);
        if color.a > 0.01 {
            return color;
        }

        pos += dir * 10.;
    }

    return vec4f(0.);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let pixel_pos = in.tex_coord * vec2f(textureDimensions(in_texture));

    var result = vec4f(0.);

    for (var i = 0u; i < uniforms.ray_count; i += 1u) {
        let angle = f32(i) * tau / f32(uniforms.ray_count);
        let dir = vec2f(cos(angle), sin(angle));
        result += march_ray(pixel_pos, dir);
    }

    result /= f32(uniforms.ray_count);
    return vec4f(result.rgb, 1.);
    // return vec4f(pow(result.rgb, vec3f(1. / 2.2)), 1.);
}
