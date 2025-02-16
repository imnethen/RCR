struct uniform_data {
    c0_rays: u32,
    c0_spacing: f32,
    c0_raylength: f32,

    angular_scaling: u32,
    spatial_scaling: f32,

    preaveraging: u32,

    num_cascades: u32,
    cur_cascade: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: uniform_data;
@group(0) @binding(1)
var<storage, read> cascade_buffer: array<vec2u>;
@group(0) @binding(2)
var out_texture: texture_storage_2d<rgba16float, write>;

fn read_cascade(pos: u32) -> vec4f {
    let packed = cascade_buffer[pos];
    return vec4f(unpack2x16float(packed.x), unpack2x16float(packed.y));
}

// convert position from 2d to 1d
fn pos_2d1d(pos2d: vec2u, dims: vec2u) -> u32 {
    return pos2d.x + pos2d.y * dims.x;
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
    let pp = cascade_probe_spacing(cascade_index) * vec2f(probe_index);
    let offset = 0.5 * (pow(uniforms.spatial_scaling, f32(cascade_index)) - 1.) / (uniforms.spatial_scaling - 1.);
    return 0.5 + pp - offset * uniforms.c0_spacing;
}

fn probe_index_from_position(cascade_index: u32, probe_pos: vec2f) -> vec2u {
    let offset = 0.5 * (pow(uniforms.spatial_scaling, f32(cascade_index)) - 1.) / (uniforms.spatial_scaling - 1.);
    let pp = probe_pos - 0.5 + offset * uniforms.c0_spacing;
    return vec2u(pp / cascade_probe_spacing(cascade_index));
}

fn get_color(id2d: vec2u) -> vec4f {
    let pos = vec2f(id2d) + 0.5;

    let probe_index = probe_index_from_position(0u, pos);
    let probe_pos = probe_position_from_index(0u, probe_index);
    let spatial_resolution = cascade_spatial_resolution(0u);

    let d = pos - probe_pos;
    var weights = bilinear_weights(d / uniforms.c0_spacing);

    var result = vec4f(0.);

    for (var i = 0u; i < 4; i += 1u) {
        let offset = vec2u(i & 1, i >> 1);
        let prev_index1d = pos_2d1d(clamp(probe_index + offset, vec2u(0), spatial_resolution - 1), spatial_resolution);

        if uniforms.preaveraging == 1 {
            result += weights[i] * read_cascade(prev_index1d);
        } else {
            for (var j = 0u; j < uniforms.c0_rays; j += 1u) {
                let probe_index1d = prev_index1d + j * spatial_resolution.x * spatial_resolution.y;

                result += weights[i] * (1. / f32(uniforms.c0_rays)) * read_cascade(probe_index1d);
            }
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
