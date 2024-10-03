struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct SkyOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec3<f32>,
};

struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    projection_inv: mat4x4<f32>,
    position: vec3<f32>,
}

@group(0) @binding(0) 
var<uniform> camera: Camera;

@vertex
fn vs_sky(input: VertexInput) -> SkyOutput {
    var output: SkyOutput;

    let position = camera.projection * camera.view * vec4<f32>(input.position, 1.0);
    output.position = position.xyww;
    output.uv = input.position;
    return output;
}

@group(0) @binding(1) var texture: texture_cube<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

@fragment
fn fs_sky(input: SkyOutput) -> @location(0) vec4<f32> {
    return textureSample(texture, tex_sampler, input.uv);
}