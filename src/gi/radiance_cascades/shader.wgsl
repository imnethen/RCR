const tau = 6.283185307179586;

struct uniform_data {
    c0_rays: u32,
    c0_spacing: f32,
    c0_raylength: f32,

    angular_scaling: u32,
    spatial_scaling: f32,

    num_cascades: u32,
    cur_cascade: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: uniform_data;
@group(0) @binding(1)
var nearest_sampler: sampler;
@group(0) @binding(2)
var linear_sampler: sampler;
@group(0) @binding(3)
var sdf_texture: texture_2d<f32>;

@group(1) @binding(0)
var in_texture: texture_2d<f32>;

@group(2) @binding(0)
var prev_cascade: texture_2d<f32>;
@group(2) @binding(1)
var out_texture: texture_storage_2d<rgba32float, write>;

fn to_tex(pos: vec2f, texel: vec2f) -> vec2f {
    return (pos + vec2f(0.5)) * texel;
}

fn out_of_bounds(pos: vec2f, dims: vec2u) -> bool {
    return (pos.x < 0. || pos.y < 0. || pos.x >= f32(dims.x) || pos.y >= f32(dims.y));
}

fn march_ray(start_pos: vec2f, dir: vec2f) -> vec4f {
    let in_texture_dims = textureDimensions(in_texture);
    let texel = vec2f(1.) / vec2f(in_texture_dims);
    var pos = start_pos;

    for (var step = 0u; step < 1024u; step += 1u) {
        let color = textureSampleLevel(in_texture, nearest_sampler, to_tex(pos, texel), 0.);
        if color.a > 0.99 {
            return color;
        }

        let dist = textureSampleLevel(sdf_texture, nearest_sampler, to_tex(pos, texel), 0.).r;
        pos += dir * dist * 0.9;

        if out_of_bounds(pos, in_texture_dims) {
            return vec4f(0.);
        }
    }

    return vec4f(0.);
}

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id2d: vec3u, @builtin(num_workgroups) nw: vec3u) {
    let id = vec2u(id2d.x + id2d.y * nw.x * 16, 0);
    let in_texture_dims = textureDimensions(in_texture);
    let out_texture_dims = textureDimensions(out_texture);
    let in_pixel_pos = vec2u(id.x % in_texture_dims.x, id.x / in_texture_dims.x);
    let out_pixel_pos = vec2u(id.x % out_texture_dims.x, id.x / out_texture_dims.x);

    var result = vec4f(0.);

    // FIXME
    let ray_count = 64u;
    for (var i = 0u; i < ray_count; i += 1u) {
        let angle = f32(i) * tau / f32(ray_count);
        let dir = vec2f(cos(angle), sin(angle));
        result += march_ray(vec2f(in_pixel_pos), dir);
    }

    result /= f32(ray_count);
    textureStore(out_texture, out_pixel_pos, vec4f(result.rgb, 1.));
}
