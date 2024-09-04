struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> model: mat4x4<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@stage(vertex)
fn main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.projection * camera.view * model * vec4<f32>(input.position, 1.0);
    return output;
}

struct FragmentInput {
    @builtin(position) position: vec4<f32>
}

@stage(fragment)
fn main(input: FragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}