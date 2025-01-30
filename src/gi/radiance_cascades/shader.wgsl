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
var out_texture: texture_storage_2d<rgba16float, write>;

// convert position from 2d to 1d
fn pos_2d1d(pos2d: vec2u, dims: vec2u) -> u32 {
    return pos2d.x + pos2d.y * dims.x;
}

// convert position from 1d to 2d
fn pos_1d2d(pos1d: u32, dims: vec2u) -> vec2u {
    return vec2u(pos1d % dims.x, pos1d / dims.x);
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
        let color = textureSampleLevel(in_texture, nearest_sampler, pos * texel, 0.);
        if color.a > 0.99 {
            return color;
        }

        let dist = textureSampleLevel(sdf_texture, nearest_sampler, pos * texel, 0.).r;
        // TODO: check if the *0.9 is necessary
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

    result = rc(id1d);
    result = merge(id1d, result);

    textureStore(out_texture, out_pixel_pos, result);
}

// everything after this and before the next one is good and will not be deleted
fn cascade_angular_resolution(cascade_index: u32) -> u32 {
    let mult = u32(0.5 + pow(f32(uniforms.angular_scaling), f32(cascade_index)));
    return uniforms.c0_rays * mult;
}

fn cascade_probe_spacing(cascade_index: u32) -> f32 {
    let mult = pow(f32(uniforms.spatial_scaling), f32(cascade_index));
    return uniforms.c0_spacing * mult;
}

fn cascade_spatial_resolution(cascade_index: u32) -> vec2u {
    let in_dims = vec2f(textureDimensions(in_texture));
    let spacing = cascade_probe_spacing(cascade_index);
    return vec2u(ceil(in_dims / spacing)) + 1;
}

fn cascade_ray_length(cascade_index: u32) -> f32 {
    let mult = pow(f32(uniforms.angular_scaling), f32(cascade_index));
    return uniforms.c0_raylength * mult;
}

fn cascade_ray_offset(cascade_index: u32) -> f32 {
    let scaling = f32(uniforms.angular_scaling);
    return uniforms.c0_raylength * (pow(scaling, f32(cascade_index)) - 1.) / (scaling - 1.);
}

// ---

fn probe_index_2d(cascade_index: u32, id: u32) -> vec2u {
    let spatial_resolution = cascade_spatial_resolution(cascade_index);
    let x = id % spatial_resolution.x;
    let y = (id % (spatial_resolution.x * spatial_resolution.y)) / spatial_resolution.x;
    return vec2u(x, y);
}

fn probe_position_from_index(cascade_index: u32, probe_index: vec2u) -> vec2f {
    let pp = cascade_probe_spacing(cascade_index) * vec2f(probe_index);
    let offset = 0.5 * (pow(uniforms.spatial_scaling, f32(cascade_index)) - 1.) / (uniforms.spatial_scaling - 1.);
    return 0.5 + pp - offset * uniforms.c0_spacing;
}

fn probe_index_from_position(cascade_index: u32, probe_pos: vec2f) -> vec2u {
    let resolution = cascade_spatial_resolution(cascade_index);

    var res: vec2u;
    {
        var l = 0u;
        var r = resolution.x + 1u;

        while (l + 1 < r) {
            let m = (l + r) / 2u;
            let pm = probe_position_from_index(cascade_index, vec2u(m, 0)).x;
            if (pm <= probe_pos.x) {
                l = m;
            } else {
                r = m;
            }
        }

        res.x = l;
    }
    {
        var l = 0u;
        var r = resolution.y + 1u;

        while (l + 1 < r) {
            let m = (l + r) / 2u;
            let pm = probe_position_from_index(cascade_index, vec2u(m, 0)).x;
            if (pm <= probe_pos.y) {
                l = m;
            } else {
                r = m;
            }
        }

        res.y = l;
    }

    return res;
}

fn probe_position(cascade_index: u32, id: u32) -> vec2f {
    let index = probe_index_2d(cascade_index, id);
    return probe_position_from_index(cascade_index, index);
}

fn get_ray_index(cascade_index: u32, id: u32) -> u32 {
    let spares = cascade_spatial_resolution(cascade_index);
    return id / (spares.x * spares.y);
}

fn ray_angle_from_index(cascade_index: u32, ray_index: f32) -> f32 {
    return ray_index * tau / f32(cascade_angular_resolution(cascade_index));
}

// ---

fn rc(id: u32) -> vec4f {
    let cascade = uniforms.cur_cascade;

    let ray_index = f32(get_ray_index(cascade, id)) + 0.5;
    let angle = ray_angle_from_index(uniforms.cur_cascade, ray_index);
    let dir = vec2f(cos(angle), sin(angle));

    let pos = probe_position(cascade, id) + dir * cascade_ray_offset(cascade);
    let ray_color = march_ray(pos, dir, cascade_ray_length(cascade));
    return ray_color;
}

fn merge(id: u32, ray_color: vec4f) -> vec4f {
    let curcascade = uniforms.cur_cascade;

    if curcascade >= uniforms.num_cascades - 1 || ray_color.a >= 0.99 {
        return ray_color;
    }

    let dims = textureDimensions(prev_cascade);

    let probe_index = probe_index_2d(curcascade, id);
    let probe_pos = probe_position_from_index(curcascade, probe_index);
    let ray_index = get_ray_index(curcascade, id);

    let prev_ray_index = ray_index * uniforms.angular_scaling;
    let prev_probe_index = probe_index_from_position(curcascade + 1, probe_pos);
    let prev_spatial = cascade_spatial_resolution(curcascade + 1);

    let prev_probe_pos = probe_position_from_index(curcascade + 1, prev_probe_index);
    let d = probe_pos - prev_probe_pos;
    var weights = bilinear_weights(d / cascade_probe_spacing(curcascade + 1));

    var result = vec4f(0.);

    for (var i = 0u; i < 4; i += 1u) {
        let offset = vec2u(i & 1, i >> 1);
        // TODO: check if clamp is necessary, i dont think it is
        let merge_probe_index = clamp(prev_probe_index + offset, vec2u(0), prev_spatial - 1);

        var probe_result = vec4f(0.);

        for (var j = 0u; j < uniforms.angular_scaling; j += 1u) {
            let merge_ray_index = prev_ray_index + j;
            let pos = merge_probe_index.x + merge_probe_index.y * prev_spatial.x + merge_ray_index * prev_spatial.x * prev_spatial.y;
            probe_result += textureLoad(prev_cascade, pos_1d2d(pos, dims), 0);
        }

        result += probe_result * weights[i];
    }

    result /= f32(uniforms.angular_scaling);
    result.a = 1.;
    return result;
}

fn bilinear_weights(pos: vec2f) -> array<f32, 4> {
    return array(
        (1. - pos.x) * (1. - pos.y),
        pos.x * (1. - pos.y),
        (1. - pos.x) * pos.y,
        pos.x * pos.y,
    );
}
