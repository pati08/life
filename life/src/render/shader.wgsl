struct InstanceInput {
    @location(1) offset: vec2<f32>,
    @location(2) center: vec2<f32>,
}

struct Res {
    data: vec2<f32>,
    padding: vec2<f32>,
}

// For aspect ratio and stuff
@group(0) @binding(0)
var<uniform> res: Res;

struct Rad {
    data: f32,
    padding: f32,
    padding2: vec2<f32>,
}

@group(1) @binding(0)
var<uniform> radius: Rad;

@group(2) @binding(0)
var<uniform> color: vec4<f32>;

struct Pan {
    data: vec2<f32>,
    padding: vec2<f32>,
}

@group(4) @binding(0)
var<uniform> pan: Pan;

// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(3) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_coord: vec4<f32>,
    @location(4) circle_center: vec2<f32>,
    @location(3) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let res = res.data;
    //let radius = radius.data;
    let pan = pan.data;
    let aspect_ratio = f32(res.x) / f32(res.y);

    let position = (model.position) / vec3<f32>(aspect_ratio, 1.0, 1.0);
    //let position = model.position;
    let offset = (instance.offset - (vec2<f32>((pan.x * 2), -((pan.y * 2))))) / vec2<f32>(aspect_ratio, 1.0);
    //let offset = instance.offset;

    var out: VertexOutput;
    //let pan_mod = vec2<f32>(-pan.x, pan.y) / vec2<f32>(aspect_ratio, 1.0);
    out.clip_position = vec4<f32>(offset, 0.0, 0.0) + vec4<f32>(position, 1.0);// + vec4<f32>(pan_mod, 0.0, 0.0);
    out.frag_coord = out.clip_position;
    out.circle_center = instance.center;
    out.tex_coords = model.tex_coords;
    return out;
}

// Fragment shader
@group(3) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(3) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //let res = res.data;
    let radius = radius.data;
    //let pan = pan.data;
    // At exremely far zooms, interpolate between the texture and a solid color
    let factor = smoothstep(0.01, 0.02, radius);
    return factor * textureSample(t_diffuse, s_diffuse, in.tex_coords) + (color * (1 - factor));
}
