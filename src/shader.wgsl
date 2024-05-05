struct InstanceInput {
    @location(5) offset: vec2<f32>,
    @location(6) center: vec2<f32>,
}

// For aspect ratio and stuff
@group(0) @binding(0)
var<uniform> resolution: vec2<f32>;

// Vertex shader
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(6) circle_center: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(instance.offset, 0.0, 0.0) + vec4<f32>(model.position, 1.0);
    out.circle_center = instance.center;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
