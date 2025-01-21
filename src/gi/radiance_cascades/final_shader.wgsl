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
    let out_pos = id.xy;
    textureStore(out_texture, out_pos, get_color(id.xy));
}

fn cascade_probe_spacing(cascade_index: u32) -> f32 {
    let mult = pow(f32(uniforms.spatial_scaling), f32(cascade_index));
    return uniforms.c0_spacing * mult;
}

fn cascade_spatial_resolution(cascade_index: u32) -> vec2u {
    let in_dims = vec2f(textureDimensions(out_texture));
    let spacing = cascade_probe_spacing(cascade_index);
    return vec2u(ceil(in_dims / spacing)) + 1;
}

fn probe_position_from_index(cascade_index: u32, probe_index: vec2u) -> vec2f {
    return cascade_probe_spacing(cascade_index) * (vec2f(probe_index) - 0.5) + uniforms.c0_spacing;
}

fn probe_index_from_position(cascade_index: u32, probe_pos: vec2f) -> vec2u {
    let index = (probe_pos - uniforms.c0_spacing) / cascade_probe_spacing(cascade_index) + 0.5;
    return vec2u(index);
}

fn get_color(id2d: vec2u) -> vec4f {
    let temp_texture_dims = textureDimensions(temp_texture);
    let pos = vec2f(id2d) + 0.5;

    let probe_index = probe_index_from_position(0u, pos);
    let probe_pos = probe_position_from_index(0u, probe_index);
    let spatial_resolution = cascade_spatial_resolution(0u);

    let d = pos - probe_pos;
    var weights = bilinear_weights(d / uniforms.c0_spacing);

    var result = vec4f(0.);

    // TODO: variable names
    for (var i = 0u; i < 4; i += 1u) {
        let offset = vec2u(i & 1, i >> 1);
        let pindex1d = pos_2d1d(clamp(probe_index + offset, vec2u(0), spatial_resolution - 1), spatial_resolution);

        for (var j = 0u; j < uniforms.c0_rays; j += 1u) {
            let prindex1d = pindex1d + j * spatial_resolution.x * spatial_resolution.y;
            let prindex2d = pos_1d2d(prindex1d, temp_texture_dims);

            result += weights[i] * (1. / f32(uniforms.c0_rays)) * textureLoad(temp_texture, prindex2d, 0);
        }
    }

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
