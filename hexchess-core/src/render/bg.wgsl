// For aspect ratio and stuff
struct Offset {
    data: vec2<f32>,
    padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> offset: Offset;

struct Size {
    data: f32,
    padding: f32,
    padding2: vec2<f32>,
}

@group(1) @binding(0)
var<uniform> grid_size: Size;

struct Res {
    data: vec2<f32>,
    padding: vec2<f32>,
}

// For aspect ratio and stuff
@group(3) @binding(0)
var<uniform> res: Res;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(3) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(2) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let offset = offset.data;
    let grid_size = grid_size.data;
    let res = res.data;
    let o = offset;
    let x = in.tex_coords.x;
    let y = in.tex_coords.y;
    let a = res.x / res.y;
    // Compute the the background texture coordinates
    let tex_coords = (vec2<f32>(0.5 + ((2 * x * a) - a) / 2, y) + o) / grid_size;

    // Fade out the texture when very zoomed out
    let alpha_mod = smoothstep(0.01, 0.02, grid_size);
    return textureSample(t_diffuse, s_diffuse, tex_coords) * vec4<f32>(1, 1, 1, alpha_mod);
}
