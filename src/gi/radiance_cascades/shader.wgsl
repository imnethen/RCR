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

// convert position from 2d to 1d
fn pos_2d1d(pos2d: vec2u, dims: vec2u) -> u32 {
    return pos2d.x + pos2d.y * dims.x;
}

// convert position from 1d to 2d
fn pos_1d2d(pos1d: u32, dims: vec2u) -> vec2u {
    return vec2u(pos1d % dims.x, pos1d / dims.x);
}

fn to_tex(pos: vec2f, texel: vec2f) -> vec2f {
    return (pos + vec2f(0.5)) * texel;
}

fn out_of_bounds(pos: vec2f, dims: vec2u) -> bool {
    return (pos.x < 0. || pos.y < 0. || pos.x >= f32(dims.x) || pos.y >= f32(dims.y));
}

fn march_ray(start_pos: vec2f, dir: vec2f, maxlen: f32) -> vec4f {
    let maxlensq = maxlen * maxlen;
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

        let from_start = pos - start_pos;
        if out_of_bounds(pos, in_texture_dims) || from_start.x * from_start.x + from_start.y * from_start.y > maxlensq {
            return vec4f(0.);
        }
    }

    return vec4f(0.);
}

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id2d: vec3u, @builtin(num_workgroups) nw: vec3u) {
    let id1d = pos_2d1d(id2d.xy, nw.xy * 16);
    let in_texture_dims = textureDimensions(in_texture);
    let out_texture_dims = textureDimensions(out_texture);
    let in_pixel_pos = pos_1d2d(id1d, in_texture_dims);
    let out_pixel_pos = pos_1d2d(id1d, out_texture_dims);

    var result = vec4f(0.);

    result = bad_and_evil_rc_that_ignores_config_and_is_bad(id1d);
    result = evil_merge(id1d, result);

    textureStore(out_texture, out_pixel_pos, vec4f(result.rgb, 1.));
}

// everything after this is evil and bad and will be deleted

// cascade info
fn get_cascade_angular_resolution(cascade_index: u32) -> u32 {
    return u32(pow(4., f32(cascade_index + 1)));
}

fn get_cascade_probe_spacing(cascade_index: u32) -> f32 {
    let sindex = pow(2., f32(cascade_index));
    return uniforms.c0_spacing * sindex;
}

fn get_cascade_spatial_resolution(cascade_index: u32) -> vec2u {
    let in_dims = vec2f(textureDimensions(in_texture));
    let spacing = get_cascade_probe_spacing(cascade_index);
    return vec2u(ceil(in_dims / spacing));
}

fn get_cascade_ray_offset(cascade_index: u32) -> f32 {
    let tindex = pow(4., f32(cascade_index));
    return (uniforms.c0_raylength * (1. - tindex)) / (1. - 4.);
}

fn get_cascade_ray_length(cascade_index: u32) -> f32 {
    let tindex = pow(4., f32(cascade_index));
    return uniforms.c0_raylength * tindex;
}

// -----------------------------
// probe info

// direction first
fn get_probe_index_2d(cascade_index: u32, id: u32) -> vec2u {
    // id = d * wh + y * w + x
    // x = id - dwh - yw = (id % wh) - yw = (id % wh) % w
    // y = (id - dwh - x) / w = (id % wh) / w

    let spatial_resolution = get_cascade_spatial_resolution(cascade_index);
    let x = id % spatial_resolution.x;
    let y = (id % (spatial_resolution.x * spatial_resolution.y)) / spatial_resolution.x;
    return vec2u(x, y);
}

fn get_probe_position(cascade_index: u32, id: u32) -> vec2f {
    let index = get_probe_index_2d(cascade_index, id);
    return get_cascade_probe_spacing(cascade_index) * vec2f(index) + get_cascade_probe_spacing(0u) * 0.5 * pow(2., f32(uniforms.cur_cascade));
}

fn get_ray_index(cascade_index: u32, id: u32) -> u32 {
    let spares = get_cascade_spatial_resolution(cascade_index);
    return id / (spares.x * spares.y);
}
// -----------------------------

fn ray_index_to_angle(cascade_index: u32, ray_index: f32) -> f32 {
    return ray_index * tau / f32(get_cascade_angular_resolution(uniforms.cur_cascade));
}

fn bad_and_evil_rc_that_ignores_config_and_is_bad(id1d: u32) -> vec4f {
    var result = vec4f(0.);

    let angle = ray_index_to_angle(uniforms.cur_cascade, f32(get_ray_index(uniforms.cur_cascade, id1d)) + 0.5);
    let ray_dir = vec2f(cos(angle), sin(angle));
    let ray_color = march_ray(get_probe_position(uniforms.cur_cascade, id1d) - vec2f(0.5) + ray_dir * get_cascade_ray_offset(uniforms.cur_cascade), ray_dir, get_cascade_ray_length(uniforms.cur_cascade));
    result = ray_color;
    return result;
}

fn evil_merge(id1d: u32, ray_color: vec4f) -> vec4f {
    if (uniforms.cur_cascade == uniforms.num_cascades - 1) {
        return ray_color;
    }
    if (ray_color.a > 0.99) {
        return ray_color;
    }

    let dims = textureDimensions(prev_cascade);

    let probe_index = get_probe_index_2d(uniforms.cur_cascade, id1d);
    let ray_index = get_ray_index(uniforms.cur_cascade, id1d);

    let prev_ray_index = ray_index * 4;
    let prev_probe_index = (probe_index - 1) / 2;
    let prev_spatial = get_cascade_spatial_resolution(uniforms.cur_cascade + 1);

    var offsets = array(
        vec2u(0, 0),
        vec2u(1, 0),
        vec2u(0, 1),
        vec2u(1, 1),
    );
    var weights = get_bilenear_weights(vec2f(0.25) * vec2f(
        2. * f32(1 - probe_index.x % 2) + 1.,
        2. * f32(1 - probe_index.y % 2) + 1.,
    ));

    var result = vec4f(0.);

    for (var i = 0u; i < 4; i += 1u) {
        let pindex = clamp(prev_probe_index + offsets[i], vec2u(0, 0), prev_spatial - 1);

        for (var j = 0u; j < 4; j += 1u) {
            let rindex = prev_ray_index + j;
            let pos = pindex.x + pindex.y * prev_spatial.x + rindex * prev_spatial.x * prev_spatial.y;
            result += weights[i] * textureLoad(prev_cascade, pos_1d2d(pos, dims), 0);
        }
    }

    result *= 0.25;
    result.a = 1.;
    return result;
}

fn get_bilenear_weights(pos: vec2f) -> array<f32, 4> {
    return array(
        (1. - pos.x) * (1. - pos.y),
        pos.x * (1. - pos.y),
        (1. - pos.x) * pos.y,
        pos.x * pos.y,
    );
}
