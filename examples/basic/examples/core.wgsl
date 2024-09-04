#define_import_path shadow::core

struct Vertex {
    @location(0) position: vec3<f32>,

    @location(1) uv0: vec2<f32>,

    @location(2) uv1: vec2<f32>,

    @location(3) normal: vec3<f32>,

    @location(4) tangent: vec4<f32>,

    @location(5) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec3<f32>,

    @location(1) uv0: vec2<f32>,

    @location(2) uv1: vec2<f32>,

    @location(3) normal: vec3<f32>,

    @location(4) tangent: vec4<f32>,

    @location(5) color: vec4<f32>,
}

type FragmentInput = VertexOutput;

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}


struct Globals {
    time: f32,
    delta_time: f32,
    frame: u32,
    _padding: u32,
}

struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,

}

struct Object {
    instance: u32,
    model: mat4x4<f32>,
}

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}