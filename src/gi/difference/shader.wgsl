struct uniform_data {
    mode: u32,
    mult: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: uniform_data;
@group(0) @binding(1)
var temp_texture_1: texture_2d<f32>;
@group(0) @binding(2)
var temp_texture_2: texture_2d<f32>;

@group(1) @binding(0)
var out_texture: texture_storage_2d<rgba32float, write>;

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let pixel_pos = id.xy;

    let tex1px = textureLoad(temp_texture_1, pixel_pos, 0);
    let tex2px = textureLoad(temp_texture_2, pixel_pos, 0);

    var result: vec4f;

    if uniforms.mode == 0 {
        result = abs(tex1px - tex2px);
    } else if uniforms.mode == 1 {
        result = tex1px - tex2px;
    } else if uniforms.mode == 2 {
        result = tex2px - tex1px;
    } else if uniforms.mode == 3 {
        result = tex1px;
    } else if uniforms.mode == 4 {
        result = tex2px;
    } else {
        result = vec4f(0., 0., 1., 1.);
    }

    textureStore(out_texture, pixel_pos, vec4f(result.rgb * uniforms.mult, 1.));
}
