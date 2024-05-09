struct InstanceInput {
    @location(1) offset: vec2<f32>,
    @location(2) center: vec2<f32>,
}

// For aspect ratio and stuff
@group(0) @binding(0)
var<uniform> res: vec2<f32>;

@group(1) @binding(0)
var<uniform> radius: f32;

@group(2) @binding(0)
var<uniform> color: vec4<f32>;

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
    let aspect_ratio = f32(res.x) / f32(res.y);

    let position = model.position / vec3<f32>(aspect_ratio, 1.0, 1.0);
    //let position = model.position;
    let offset = instance.offset / vec2<f32>(aspect_ratio, 1.0);
    //let offset = instance.offset;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(offset, 0.0, 0.0) + vec4<f32>(position, 1.0);
    out.frag_coord = out.clip_position;
    out.circle_center = instance.center;
    out.tex_coords = model.tex_coords;
    return out;
}

// Fragment shader
fn adj_distance(aspect_ratio: f32, frag_coord: vec2<f32>, center: vec2<f32>) -> f32 {
    let adj_frag_coord = frag_coord * vec2<f32>(aspect_ratio, 1);
    let x_dist = pow(adj_frag_coord.x - center.x, 2.0);
    let y_dist = pow(adj_frag_coord.y - center.y, 2.0);
    return sqrt(x_dist + y_dist);
}

@group(3) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(3) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let aspect_ratio = f32(res.x) / f32(res.y);
    let center = in.circle_center; // vec2<f32>(1, aspect_ratio);

    let frag_coord = in.frag_coord.xy / in.frag_coord.w;

    let dist = adj_distance(aspect_ratio, frag_coord, center);
    if dist > (radius) {
        discard;
    }
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
