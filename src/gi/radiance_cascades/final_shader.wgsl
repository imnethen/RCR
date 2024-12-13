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
var temp_texture: texture_2d<f32>;
@group(0) @binding(2)
var out_texture: texture_storage_2d<rgba32float, write>;

// convert position from 2d to 1d
fn pos_2d1d(pos2d: vec2u, dims: vec2u) -> u32 {
    return pos2d.x + pos2d.y * dims.x;
}

// convert position from 1d to 2d
fn pos_1d2d(pos1d: u32, dims: vec2u) -> vec2u {
    return vec2u(pos1d % dims.x, pos1d / dims.x);
}

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3u) {
    // let temp_texture_dims = textureDimensions(temp_texture);
    let out_texture_dims = textureDimensions(out_texture);

    let out_pos = id.xy;
    // let id1d = id.x + id.y * out_texture_dims.x;
    // let temp_pos = vec2u(id1d % temp_texture_dims.x, id1d / temp_texture_dims.x);

    // let color = textureLoad(temp_texture, temp_pos, 0);

    // textureStore(out_texture, out_pos, color);

    textureStore(out_texture, out_pos, get_evil_color(pos_2d1d(id.xy, out_texture_dims)));
}

// everything after this is evil and bad and will be deleted

// cascade info
fn get_cascade_angular_resolution(cascade_index: u32) -> u32 {
    return u32(pow(4., f32(cascade_index)));
}

fn get_cascade_probe_spacing(cascade_index: u32) -> f32 {
    let sindex = pow(2., f32(cascade_index));
    return uniforms.c0_spacing * sindex;
}

fn get_cascade_spatial_resolution(cascade_index: u32) -> vec2u {
    let in_dims = vec2f(textureDimensions(out_texture));
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
    return get_cascade_probe_spacing(cascade_index) * vec2f(index);
}

fn get_ray_index(cascade_index: u32, id: u32) -> u32 {
    let angres = get_cascade_angular_resolution(cascade_index);
    return id / angres;
}

fn get_evil_color(id1d: u32) -> vec4f {
    let temp_texture_dims = textureDimensions(temp_texture);
    // if in and out are different sizes then im dead i think but they shouldnt be probably i think
    let inout_texture_dims = textureDimensions(out_texture);

    let pid = get_probe_index_2d(0u, id1d);
    let spatial = get_cascade_spatial_resolution(0u);

    var poses = array(
        pid,
        pid + spatial * vec2u(0, 1),
        pid + spatial * vec2u(0, 2),
        pid + spatial * vec2u(0, 3),
    );

    var result = vec4f(0.);

    for (var i = 0u; i < 4u; i += 1u) {
        result += textureLoad(temp_texture, poses[i], 0);
    }

    result /= 4.;
    result.a = 1.;
    return result;
}
