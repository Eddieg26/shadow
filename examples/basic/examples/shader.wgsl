struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    projection_inv: mat4x4<f32>,
    position: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.projection * camera.view * model * vec4<f32>(input.position, 1.0);
    output.uv = input.uv;
    return output;
}

@group(2) @binding(0) var texture: texture_2d<f32>;
@group(2) @binding(1) var tex_sampler: sampler;


@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(texture, tex_sampler, input.uv);
}