use super::{
    constants::{
        CAMERA, CAMERA_STRUCT, FRAGMENT_INPUT, FRAGMENT_INPUT_STRUCT, MATERIAL, MATERIAL_STRUCT,
        SURFACE, SURFACE_STRUCT, VERTEX_INPUT, VERTEX_INPUT_STRUCT, VERTEX_OUTPUT_STRUCT,
    },
    BuiltinValue, ShaderAttribute, ShaderField, ShaderInput, ShaderProperty, ShaderResource,
    ShaderValue, SurfaceAttribute, VertexInput,
};
use crate::material::{shader::constants::{OBJECT, OBJECT_STRUCT, VERTEX_OUTPUT}, BlendMode, ShaderModel};
use crate::resources::mesh::MeshAttributeKind;
use std::borrow::Cow;

pub fn define_struct_field(
    name: &str,
    property: ShaderProperty,
    attribute: Option<&ShaderAttribute>,
) -> Option<String> {
    let attribute = attribute.map(|a| a.to_string()).unwrap_or_default();
    let property = match property {
        ShaderProperty::Float => "f32",
        ShaderProperty::UInt => "u32",
        ShaderProperty::SInt => "i32",
        ShaderProperty::Bool => "bool",
        ShaderProperty::Color => "vec4<f32>",
        ShaderProperty::Vec2 => "vec2<f32>",
        ShaderProperty::Vec3 => "vec3<f32>",
        ShaderProperty::Vec4 => "vec4<f32>",
        ShaderProperty::Mat2 => "mat2x2<f32>",
        ShaderProperty::Mat3 => "mat3x3<f32>",
        ShaderProperty::Mat4 => "mat4x4<f32>",
        _ => return None,
    };

    Some(format!("{} {}: {}", attribute, name, property))
}

pub fn define_struct<'a>(name: &str, fields: &[ShaderField]) -> String {
    let mut code = String::new();

    for field in fields.into_iter() {
        if let Some(field) =
            define_struct_field(&field.name, field.property, field.attribute.as_ref())
        {
            code += &format!("{},", field)
        }
    }

    match code.is_empty() {
        true => String::new(),
        false => format!(
            r#"struct {} {{
            {}
            }};"#,
            name, code
        ),
    }
}

pub fn define_binding(name: &str, group: u32, binding: u32, resource: ShaderResource) -> String {
    let param = match &resource {
        ShaderResource::Uniform { .. } => "<uniform>",
        ShaderResource::Storage { read_write, .. } => match read_write {
            true => "<storage, read_write>",
            false => "<storage>",
        },
        _ => "",
    };

    let ty = match &resource {
        ShaderResource::Texture2D => "texture_2d<f32>",
        ShaderResource::Texture2DArray => "texture_2d_array<f32>",
        ShaderResource::Texture3D => "texture_3d<f32>",
        ShaderResource::Texture3DArray => "texture_3d_array<f32>",
        ShaderResource::TextureCube => "texture_cube<f32>",
        ShaderResource::Sampler => "sampler",
        ShaderResource::Uniform { ty } => &ty,
        ShaderResource::Storage { ty, .. } => &ty,
    };

    format!(
        "@group({}) @binding({}) var{} {}: {};",
        group, binding, param, name, ty,
    )
}

pub fn define_material(group: u32, inputs: &[ShaderInput]) -> String {
    let mut code = String::new();
    let mut fields = vec![];
    let mut bindings = 0;
    for input in inputs {
        match input.property.is_primitive() {
            true => fields.push(ShaderField::new(&input.name, input.property)),
            false => match input.property.resource() {
                Some(resource) => {
                    code += &define_binding(&input.name, 2, bindings, resource);
                    bindings += 1;
                }
                None => continue,
            },
        }
    }

    let material = &define_struct(MATERIAL_STRUCT, &fields);
    if !material.is_empty() {
        code += &material;
        code += &define_binding(
            MATERIAL,
            group,
            bindings,
            ShaderResource::Uniform {
                ty: String::from(MATERIAL_STRUCT),
            },
        );
    }

    code
}

pub fn define_surface(model: ShaderModel) -> String {
    let fields: &[ShaderField] = match model {
        ShaderModel::Lit => &[
            ShaderField::new(SurfaceAttribute::Color.name(), ShaderProperty::Color),
            ShaderField::new(SurfaceAttribute::Normal.name(), ShaderProperty::Vec3),
            ShaderField::new(SurfaceAttribute::Specular.name(), ShaderProperty::Float),
            ShaderField::new(SurfaceAttribute::Metallic.name(), ShaderProperty::Float),
            ShaderField::new(SurfaceAttribute::Smoothness.name(), ShaderProperty::Float),
            ShaderField::new(SurfaceAttribute::Opacity.name(), ShaderProperty::Float),
            ShaderField::new(SurfaceAttribute::Emission.name(), ShaderProperty::Color),
        ],
        ShaderModel::Unlit => &[
            ShaderField::new(SurfaceAttribute::Color.name(), ShaderProperty::Color),
            ShaderField::new(SurfaceAttribute::Opacity.name(), ShaderProperty::Float),
        ],
    };

    define_struct(SURFACE_STRUCT, fields)
}

pub fn declare_surface(model: ShaderModel) -> String {
    format!(
        r#"var {surface} = {SURFACE} (
            {values}
        );
    "#,
        surface = SURFACE,
        SURFACE = SURFACE_STRUCT,
        values = match model {
            ShaderModel::Lit => {
                r#"
                vec4<f32>(1.0, 1.0, 1.0, 1.0), //color
                vec3<f32>(0.0, 0.0, 0.0), //normal
                0.0, //specular
                0.0, //metallic
                0.0, //smoothness
                1.0, //opacity
                vec4<f32>(0.0, 0.0, 0.0, 0.0), //emission
            "#
            }
            ShaderModel::Unlit => {
                r#"
                vec4<f32>(1.0, 1.0, 1.0, 1.0), //color
                1.0, //opacity
            "#
            }
        }
    )
}

pub fn define_vertex_input(inputs: &[MeshAttributeKind]) -> String {
    let fields = inputs
        .iter()
        .enumerate()
        .map(|(index, i)| match i {
            MeshAttributeKind::Position => ShaderField::new("position", ShaderProperty::Vec3)
                .with_attribute(ShaderAttribute::Location(index as u32)),
            MeshAttributeKind::Normal => ShaderField::new("normal", ShaderProperty::Vec3)
                .with_attribute(ShaderAttribute::Location(index as u32)),
            MeshAttributeKind::TexCoord0 => ShaderField::new("texCoord0", ShaderProperty::Vec2)
                .with_attribute(ShaderAttribute::Location(index as u32)),
            MeshAttributeKind::TexCoord1 => ShaderField::new("texCoord1", ShaderProperty::Vec2)
                .with_attribute(ShaderAttribute::Location(index as u32)),
            MeshAttributeKind::Tangent => ShaderField::new("tangent", ShaderProperty::Vec4)
                .with_attribute(ShaderAttribute::Location(index as u32)),
            MeshAttributeKind::Color => ShaderField::new("color", ShaderProperty::Color)
                .with_attribute(ShaderAttribute::Location(index as u32)),
        })
        .collect::<Vec<_>>();

    define_struct(VERTEX_INPUT_STRUCT, &fields)
}

pub fn define_vertex_output() -> String {
    define_fragment_input(VERTEX_OUTPUT_STRUCT)
}

pub fn declare_vertex_ouput() -> String {
    format!(
        r#"var {output} = {OUTPUT} (
            vec4<f32>(0.0, 0.0, 0.0, 1.0), //clip_position
            vec3<f32>(0.0, 0.0, 0.0), //position
            vec3<f32>(0.0, 0.0, 0.0), //normal
            vec2<f32>(0.0, 0.0), //texCoord0
        );
    "#,
        output = VERTEX_OUTPUT,
        OUTPUT = VERTEX_OUTPUT_STRUCT,
    )
}

pub fn define_camera(group: u32, binding: u32) -> String {
    let camera = define_struct(
        CAMERA_STRUCT,
        &[
            ShaderField::new("view", ShaderProperty::Mat4),
            ShaderField::new("projection", ShaderProperty::Mat4),
            ShaderField::new("position", ShaderProperty::Vec3),
        ],
    );

    let binding = define_binding(
        CAMERA,
        group,
        binding,
        ShaderResource::Uniform {
            ty: String::from(CAMERA_STRUCT),
        },
    );

    format!("{}\n{}", camera, binding)
}

pub fn define_object(group: u32, binding: u32) -> String {
    let object = define_struct(
        OBJECT_STRUCT,
        &[ShaderField::new("model", ShaderProperty::Mat4)],
    );

    let binding = define_binding(
        OBJECT,
        group,
        binding,
        ShaderResource::Uniform {
            ty: String::from(OBJECT_STRUCT),
        },
    );

    format!("{}\n{}", object, binding)
}

pub fn define_fragment_input(name: &str) -> String {
    define_struct(
        name,
        &[
            ShaderField::new("clip_position", ShaderProperty::Vec4)
                .with_attribute(ShaderAttribute::Builtin(BuiltinValue::Position)),
            ShaderField::new("position", ShaderProperty::Vec3)
                .with_attribute(ShaderAttribute::Location(0)),
            ShaderField::new("normal", ShaderProperty::Vec3)
                .with_attribute(ShaderAttribute::Location(1)),
            ShaderField::new("uv", ShaderProperty::Vec2)
                .with_attribute(ShaderAttribute::Location(2)),
        ],
    )
}

pub fn define_fragment_body(body: String, mode: BlendMode) -> String {
    let opacity = match mode {
        BlendMode::Opaque => Cow::Borrowed("1.0"),
        BlendMode::Transparent => Cow::Owned(surface_field(SurfaceAttribute::Opacity)),
    };

    format!(
        r#"
            @fragment
            fn main({FRAGMENT_INPUT}: {FRAGMENT_INPUT_STRUCT}) -> @location(0) vec4<f32> {{
                {BODY}
                let color = vec4<f32>({SURFACE}.color.rgb, {OPACITY});
                return color;
            }}
        "#,
        FRAGMENT_INPUT = FRAGMENT_INPUT,
        FRAGMENT_INPUT_STRUCT = FRAGMENT_INPUT_STRUCT,
        BODY = body,
        SURFACE = SURFACE,
        OPACITY = opacity,
    )
}

pub fn define_vertex_body(body: String) -> String {
    let output = declare_vertex_ouput();

    format!(
        r#"
            @vertex
            fn main({VERTEX_INPUT}: {VERTEX_INPUT_STRUCT}) -> {VERTEX_OUTPUT_STRUCT} {{
                {OUTPUT_DECL}
                {BODY}
                return {VERTEX_OUTPUT};
            }}
        "#,
        VERTEX_INPUT = VERTEX_INPUT,
        VERTEX_INPUT_STRUCT = VERTEX_INPUT_STRUCT,
        VERTEX_OUTPUT_STRUCT = VERTEX_OUTPUT_STRUCT,
        OUTPUT_DECL = output,
        BODY = body,
        VERTEX_OUTPUT = VERTEX_OUTPUT,
    )
}

pub fn declare_value(value: &ShaderValue) -> String {
    match value {
        ShaderValue::Float(v) => format!("{}f32", v),
        ShaderValue::UInt(v) => format!("{}u32", v),
        ShaderValue::SInt(v) => format!("{}i32", v),
        ShaderValue::Bool(v) => format!("{}", v),
        ShaderValue::Color(v) => format!("vec4<f32>({}, {}, {}, {})", v.r, v.g, v.b, v.a),
        ShaderValue::Vec2(v) => format!("vec2<f32>({}, {})", v.x, v.y),
        ShaderValue::Vec3(v) => format!("vec3<f32>({}, {}, {})", v.x, v.y, v.z),
        ShaderValue::Vec4(v) => format!("vec4<f32>({}, {}, {}, {})", v.x, v.y, v.z, v.w),
        ShaderValue::Mat2(v) => format!(
            "mat2x2<f32>(vec2<f32>({}, {}), vec2<f32>({}, {}))",
            v.x_axis.x, v.x_axis.y, v.y_axis.x, v.y_axis.y
        ),
        ShaderValue::Mat3(v) => format!(
            "mat3x3<f32>(vec3<f32>({}, {}, {}), vec3<f32>({}, {}, {}), vec3<f32>({}, {}, {}))",
            v.x_axis.x,
            v.x_axis.y,
            v.x_axis.z,
            v.y_axis.x,
            v.y_axis.y,
            v.y_axis.z,
            v.z_axis.x,
            v.z_axis.y,
            v.z_axis.z
        ),
        ShaderValue::Mat4(v) => format!(
            "mat4x4<f32>(
                vec4<f32>({}, {}, {}, {}),
                vec4<f32>({}, {}, {}, {}),
                vec4<f32>({}, {}, {}, {}),
                vec4<f32>({}, {}, {}, {})
            )",
            v.x_axis.x,
            v.x_axis.y,
            v.x_axis.z,
            v.x_axis.w,
            v.y_axis.x,
            v.y_axis.y,
            v.y_axis.z,
            v.y_axis.w,
            v.z_axis.x,
            v.z_axis.y,
            v.z_axis.z,
            v.z_axis.w,
            v.w_axis.x,
            v.w_axis.y,
            v.w_axis.z,
            v.w_axis.w
        ),
    }
}

pub fn convert_input(from: &ShaderInput, to: ShaderProperty) -> Option<String> {
    if from.property == to {
        return Some(from.name.clone());
    }

    match (from.property, to) {
        (ShaderProperty::Float, ShaderProperty::UInt) => Some(format!("u32({})", from.name)),
        (ShaderProperty::Float, ShaderProperty::SInt) => Some(format!("i32({})", from.name)),
        (ShaderProperty::Float, ShaderProperty::Vec2) => Some(format!("vec2<f32>({})", from.name)),
        (ShaderProperty::Float, ShaderProperty::Vec3) => Some(format!("vec3<f32>({})", from.name)),
        (ShaderProperty::Float, ShaderProperty::Vec4) => Some(format!("vec3<f32>({})", from.name)),
        (ShaderProperty::Float, ShaderProperty::Color) => Some(format!("vec3<f32>({})", from.name)),

        (ShaderProperty::UInt, ShaderProperty::Float) => Some(format!("f32({})", from.name)),
        (ShaderProperty::UInt, ShaderProperty::SInt) => Some(format!("i32({})", from.name)),
        (ShaderProperty::UInt, ShaderProperty::Bool) => Some(format!("bool({})", from.name)),
        (ShaderProperty::UInt, ShaderProperty::Vec2) => Some(format!("vec2<f32>({})", from.name)),
        (ShaderProperty::UInt, ShaderProperty::Vec3) => Some(format!("vec3<f32>({})", from.name)),
        (ShaderProperty::UInt, ShaderProperty::Vec4) => Some(format!("vec3<f32>({})", from.name)),
        (ShaderProperty::UInt, ShaderProperty::Color) => Some(format!("vec3<f32>({})", from.name)),

        (ShaderProperty::SInt, ShaderProperty::Float) => Some(format!("f32({})", from.name)),
        (ShaderProperty::SInt, ShaderProperty::UInt) => Some(format!("u32({})", from.name)),
        (ShaderProperty::SInt, ShaderProperty::Bool) => Some(format!("bool({})", from.name)),
        (ShaderProperty::SInt, ShaderProperty::Vec2) => Some(format!("vec2<f32>({})", from.name)),
        (ShaderProperty::SInt, ShaderProperty::Vec3) => Some(format!("vec3<f32>({})", from.name)),
        (ShaderProperty::SInt, ShaderProperty::Vec4) => Some(format!("vec3<f32>({})", from.name)),
        (ShaderProperty::SInt, ShaderProperty::Color) => Some(format!("vec3<f32>({})", from.name)),

        (ShaderProperty::Vec2, ShaderProperty::Vec3) => Some(format!("vec3<f32>({})", from.name)),
        (ShaderProperty::Vec2, ShaderProperty::Vec4) => Some(format!("vec3<f32>({})", from.name)),
        (ShaderProperty::Vec2, ShaderProperty::Color) => Some(format!("vec3<f32>({})", from.name)),

        (ShaderProperty::Vec3, ShaderProperty::Vec2) => {
            Some(format!("vec2<f32>({NAME}.x, {NAME}.y)", NAME = from.name))
        }
        (ShaderProperty::Vec3, ShaderProperty::Vec4) => Some(format!(
            "vec4<f32>({NAME}.x, {NAME}.y, {NAME}.z, 1.0)",
            NAME = from.name
        )),
        (ShaderProperty::Vec3, ShaderProperty::Color) => Some(format!(
            "vec4<f32>({NAME}.x, {NAME}.y, {NAME}.z, 1.0)",
            NAME = from.name
        )),

        (ShaderProperty::Vec4, ShaderProperty::Vec2) => {
            Some(format!("vec2<f32>({NAME}.x, {NAME}.y)", NAME = from.name))
        }
        (ShaderProperty::Vec4, ShaderProperty::Vec3) => Some(format!(
            "vec4<f32>({NAME}.x, {NAME}.y, {NAME}.z, 1.0)",
            NAME = from.name
        )),
        (ShaderProperty::Color, ShaderProperty::Vec2) => {
            Some(format!("vec2<f32>({NAME}.x, {NAME}.y)", NAME = from.name))
        }
        (ShaderProperty::Color, ShaderProperty::Vec3) => Some(format!(
            "vec4<f32>({NAME}.x, {NAME}.y, {NAME}.z, 1.0)",
            NAME = from.name
        )),
        (ShaderProperty::Color, ShaderProperty::Vec4) => Some(from.name.clone()),
        (ShaderProperty::Vec4, ShaderProperty::Color) => Some(from.name.clone()),
        _ => None,
    }
}

pub fn surface_field(attribute: SurfaceAttribute) -> String {
    format!("{}.{}", SURFACE, attribute.name())
}

pub fn material_field(name: &str) -> String {
    format!("{}.{}", MATERIAL, name)
}

pub fn material_input(input: &ShaderInput) -> ShaderInput {
    match input.property.is_primitive() {
        true => ShaderInput::new(material_field(&input.name), input.property),
        false => input.clone(),
    }
}

pub fn vertex_input(input: VertexInput) -> ShaderInput {
    match input {
        VertexInput::Position => {
            ShaderInput::new(format!("{}.position", VERTEX_INPUT), ShaderProperty::Vec3)
        }
        VertexInput::Normal => {
            ShaderInput::new(format!("{}.normal", VERTEX_INPUT), ShaderProperty::Vec3)
        }
        VertexInput::TexCoord0 => {
            ShaderInput::new(format!("{}.texCoord0", VERTEX_INPUT), ShaderProperty::Vec2)
        }
        VertexInput::TexCoord1 => {
            ShaderInput::new(format!("{}.texCoord1", VERTEX_INPUT), ShaderProperty::Vec2)
        }
        VertexInput::Tangent => {
            ShaderInput::new(format!("{}.tangent", VERTEX_INPUT), ShaderProperty::Vec4)
        }
        VertexInput::Color => {
            ShaderInput::new(format!("{}.color", VERTEX_INPUT), ShaderProperty::Color)
        }
    }
}
