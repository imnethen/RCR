const tau = 6.283185307179586;

struct uniform_data {
    c0_rays: u32,
    c0_spacing: f32,
    c0_raylength: f32,

    angular_scaling: u32,
    spatial_scaling: f32,

    // 0 - offset, 1 - stacked
    probe_layout: u32,
    // 0 - vanilla, 1 - bilinear
    ringing_fix: u32,

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
var<storage, read> prev_cascade: array<vec2u>;
@group(2) @binding(1)
var<storage, read_write> out_cascade: array<vec2u>;

// TODO: figure out if its possible to make generic read/write functions for this
// the problem is that arguments are immutable and immutable arrays can only be indexed with constants
fn read_prev_cascade(pos: u32) -> vec4f {
    let packed = prev_cascade[pos];
    return vec4f(unpack2x16float(packed.x), unpack2x16float(packed.y));
}

fn store_to_out_cascade(pos: u32, value: vec4f) {
    let packed = vec2u(pack2x16float(value.rg), pack2x16float(value.ba));
    out_cascade[pos] = packed;
}

fn out_of_bounds(pos: vec2f, dims: vec2u) -> bool {
    return (pos.x < 0. || pos.y < 0. || pos.x >= f32(dims.x) || pos.y >= f32(dims.y));
}

fn march_ray(start_pos: vec2f, dir: vec2f, maxlen: f32) -> vec4f {
    let maxlensq = maxlen * maxlen;
    let in_texture_dims = textureDimensions(in_texture);
    let texel = vec2f(1.) / vec2f(in_texture_dims);
    var pos = start_pos;

    if out_of_bounds(pos, in_texture_dims) {
        return vec4f(0.);
    }

    for (var step = 0u; step < 1024u; step += 1u) {
        let dist = textureSampleLevel(sdf_texture, nearest_sampler, pos * texel, 0.).r;

        if dist < 1 {
            let color = textureSampleLevel(in_texture, nearest_sampler, pos * texel, 0.);
            if color.a > 0.99 {
                return color;
            }
        }

        pos += dir * dist;

        let from_start = pos - start_pos;
        if out_of_bounds(pos, in_texture_dims) || dot(from_start, from_start) > maxlensq {
            return vec4f(0.);
        }
    }

    return vec4f(0.);
}

@compute
@workgroup_size(128)
fn main(@builtin(global_invocation_id) id3d: vec3u) {
    let id = id3d.x;
    var result: vec4f;

    if uniforms.ringing_fix == 0 {
        result = rc(id);
    } else if uniforms.ringing_fix == 1 {
        result = rc_bilinear(id);
    }

    store_to_out_cascade(id, result);
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
    return 0.5 + pp - f32(1 - uniforms.probe_layout) * offset * uniforms.c0_spacing;
}

fn probe_index_from_position(cascade_index: u32, probe_pos: vec2f) -> vec2u {
    let offset = 0.5 * (pow(uniforms.spatial_scaling, f32(cascade_index)) - 1.) / (uniforms.spatial_scaling - 1.);
    let pp = probe_pos - 0.5 + f32(1 - uniforms.probe_layout) * offset * uniforms.c0_spacing;
    return vec2u(pp / cascade_probe_spacing(cascade_index));
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

fn ray_index_from_angle(cascade_index: u32, ray_angle: f32) -> f32 {
    return (ray_angle / tau) * f32(cascade_angular_resolution(cascade_index));
}

// ---

fn rc(id: u32) -> vec4f {
    let cascade = uniforms.cur_cascade;

    let num_rays = select(uniforms.c0_rays, uniforms.angular_scaling, cascade != 0);

    var result = vec4f(0);

    for (var i = 0u; i < num_rays; i += 1u) {
        let ray_index = f32(get_ray_index(cascade, id) * num_rays + i) + 0.5;
        let angle = ray_angle_from_index(uniforms.cur_cascade, ray_index);
        let dir = vec2f(cos(angle), sin(angle));

        let pos = probe_position(cascade, id) + dir * cascade_ray_offset(cascade);
        let ray_color = march_ray(pos, dir, cascade_ray_length(cascade));
        let ray_result = merge(id, ray_color, get_ray_index(cascade, id) * num_rays + i);

        result += ray_result;
    }

    result /= f32(num_rays);
    return result;
}

fn merge(id: u32, ray_color: vec4f, ray_index: u32) -> vec4f {
    let curcascade = uniforms.cur_cascade;

    if curcascade >= uniforms.num_cascades - 1 || ray_color.a >= 0.99 {
        return ray_color;
    }

    let probe_index = probe_index_2d(curcascade, id);
    let probe_pos = probe_position_from_index(curcascade, probe_index);

    let prev_ray_store_index = ray_index;
    let prev_probe_index = probe_index_from_position(curcascade + 1, probe_pos);
    let prev_spatial = cascade_spatial_resolution(curcascade + 1);

    let prev_probe_pos = probe_position_from_index(curcascade + 1, prev_probe_index);
    let d = probe_pos - prev_probe_pos;
    var weights = bilinear_weights(d / cascade_probe_spacing(curcascade + 1));

    var result = vec4f(0.);

    for (var i = 0u; i < 4; i += 1u) {
        if weights[i] < 0.01 {
            continue;
        }

        let offset = vec2u(i & 1, i >> 1);
        let merge_probe_index = clamp(prev_probe_index + offset, vec2u(0), prev_spatial - 1);

        let pos = merge_probe_index.x + merge_probe_index.y * prev_spatial.x + prev_ray_store_index * prev_spatial.x * prev_spatial.y;
        let probe_result = read_prev_cascade(pos);

        result += probe_result * weights[i];
    }

    result.a = 1.;
    return result;
}

fn rc_bilinear(id: u32) -> vec4f {
    let cascade = uniforms.cur_cascade;

    let num_rays: u32 = select(uniforms.c0_rays, uniforms.angular_scaling, cascade != 0);

    let probe_index = probe_index_2d(cascade, id);
    let probe_pos = probe_position_from_index(cascade, probe_index);

    let prev_probe_index = probe_index_from_position(cascade + 1, probe_pos);
    let prev_spatial = cascade_spatial_resolution(cascade + 1);

    let prev_probe_pos = probe_position_from_index(cascade + 1, prev_probe_index);
    let d = probe_pos - prev_probe_pos;
    var weights = bilinear_weights(d / cascade_probe_spacing(cascade + 1));

    var result = vec4f(0.);

    for (var i = 0u; i < num_rays; i += 1u) {
        let ray_index = f32(get_ray_index(cascade, id) * num_rays + i) + 0.5;
        let angle = ray_angle_from_index(uniforms.cur_cascade, ray_index);
        let ray_dir = vec2f(cos(angle), sin(angle));

        let prev_ray_index = u32(ray_index_from_angle(cascade + 1, angle) / f32(uniforms.angular_scaling));

        for (var j = 0u; j < 4u; j += 1u) {
            if weights[j] < 0.01 {
                continue;
            }

            let offset = vec2u(j & 1, j >> 1);
            let merge_probe_index = clamp(prev_probe_index + offset, vec2u(0), prev_spatial - 1);
            let merge_probe_pos = probe_position_from_index(cascade + 1, merge_probe_index);

            let merge_buffer_index = merge_probe_index.x + merge_probe_index.y * prev_spatial.x + prev_ray_index * prev_spatial.x * prev_spatial.y;

            var next_color: vec4f;
            if uniforms.cur_cascade >= uniforms.num_cascades - 1 {
                next_color = vec4f(0., 0., 0., 1.);
            } else {
                next_color = read_prev_cascade(merge_buffer_index);
            }

            let ray_start = probe_pos + ray_dir * cascade_ray_offset(cascade);
            let ray_end = merge_probe_pos + ray_dir * cascade_ray_offset(cascade + 1);

            let ray_color = march_ray(ray_start, normalize(ray_end - ray_start), distance(ray_end, ray_start));
            let probe_result = select(next_color, ray_color, ray_color.a >= 0.99);

            result += weights[j] * probe_result;
        }
    }

    result /= f32(num_rays);
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
