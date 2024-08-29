use super::{
    attribute::{ShaderAttribute, ShaderValue},
    ShaderInput,
};
use either::Either;

pub struct Snippets;

impl Snippets {
    pub fn shader_value(value: &ShaderValue) -> Option<String> {
        let value = match value {
            ShaderValue::Float(value) => value.to_string(),
            ShaderValue::UInt(value) => value.to_string(),
            ShaderValue::SInt(value) => value.to_string(),
            ShaderValue::Vec2(value) => format!("vec2({}, {})", value.x, value.y),
            ShaderValue::Vec3(value) => format!("vec3({}, {}, {})", value.x, value.y, value.z),
            ShaderValue::Vec4(value) => {
                format!("vec4({}, {}, {}, {})", value.x, value.y, value.z, value.w)
            }
            ShaderValue::Color(value) => {
                format!("vec4({}, {}, {}, {})", value.r, value.g, value.b, value.a)
            }
            ShaderValue::Mat2(value) => {
                format!(
                    "mat2({}, {}, {}, {})",
                    value.x_axis.x, value.x_axis.y, value.y_axis.x, value.y_axis.y
                )
            }
            ShaderValue::Mat3(value) => format!(
                "mat3({}, {}, {}, {}, {}, {}, {}, {}, {})",
                value.x_axis.x,
                value.x_axis.y,
                value.x_axis.z,
                value.y_axis.x,
                value.y_axis.y,
                value.y_axis.z,
                value.z_axis.x,
                value.z_axis.y,
                value.z_axis.z
            ),
            ShaderValue::Mat4(value) => format!(
                "mat4({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
                value.x_axis.x,
                value.x_axis.y,
                value.x_axis.z,
                value.x_axis.w,
                value.y_axis.x,
                value.y_axis.y,
                value.y_axis.z,
                value.y_axis.w,
                value.z_axis.x,
                value.z_axis.y,
                value.z_axis.z,
                value.z_axis.w,
                value.w_axis.x,
                value.w_axis.y,
                value.w_axis.z,
                value.w_axis.w,
            ),
            ShaderValue::Bool(value) => value.to_string(),
            _ => return None,
        };

        Some(value)
    }

    pub fn assign_variable<'a>(
        attribute: ShaderAttribute,
        value: impl Into<Either<&'a ShaderInput, &'a ShaderValue>>,
    ) -> Option<String> {
        let value = value.into();
        let value = match value {
            Either::Left(input) => input.name().to_string(),
            Either::Right(value) => {
                Self::convert(&Self::shader_value(value)?, value.attribute(), attribute)?
            }
        };

        Some(format!("= {};", value))
    }

    pub fn define_variable(name: &str, attribute: ShaderAttribute) -> Option<String> {
        let define = match attribute {
            ShaderAttribute::Float => format!("let {}: f32", name),
            ShaderAttribute::UInt => format!("let {}: u32", name),
            ShaderAttribute::SInt => format!("let {}: i32", name),
            ShaderAttribute::Vec2 => format!("let {}: vec2<f32>", name),
            ShaderAttribute::Vec3 => format!("let {}: vec3<f32>", name),
            ShaderAttribute::Vec4 => format!("let {}: vec4<f32>", name),
            ShaderAttribute::Mat2 => format!("let {}: mat2<f32>", name),
            ShaderAttribute::Mat3 => format!("let {}: mat3<f32>", name),
            ShaderAttribute::Mat4 => format!("let {}: mat4<f32>", name),
            ShaderAttribute::Color => format!("let {}: vec4<f32>", name),
            ShaderAttribute::Bool => format!("let {}: bool", name),
            ShaderAttribute::Texture2D => format!("var {}: texture_2d<f32>;", name),
            ShaderAttribute::Texture2DArray => format!("var {}: texture_2d_array<f32>;", name),
            ShaderAttribute::Texture3D => format!("var {}: texture_3d<f32>;", name),
            ShaderAttribute::Texture3DArray => format!("var {}: texture_3d_array<f32>;", name),
            ShaderAttribute::Cubemap => format!("var {}: texture_cube<f32>;", name),
            ShaderAttribute::Sampler => format!("var {}: sampler;", name),
            ShaderAttribute::Dynamic => return None,
        };

        Some(format!("{0}", define))
    }

    pub fn init_variable<'a>(
        name: &str,
        attribute: ShaderAttribute,
        value: impl Into<Either<&'a ShaderInput, &'a ShaderValue>>,
    ) -> Option<String> {
        let assignment = Self::assign_variable(attribute, value)?;
        let definition = Self::define_variable(name, attribute)?;

        Some(format!("{} {}", definition, assignment))
    }

    pub fn convert(value: &str, from: ShaderAttribute, to: ShaderAttribute) -> Option<String> {
        if from == to {
            return Some(value.to_string());
        }

        let converted = match (from, to) {
            (ShaderAttribute::Float, ShaderAttribute::UInt) => format!("u32({})", value),
            (ShaderAttribute::Float, ShaderAttribute::SInt) => format!("i32({})", value),
            (ShaderAttribute::Float, ShaderAttribute::Vec2) => format!("vec2<f32>({})", value),
            (ShaderAttribute::Float, ShaderAttribute::Vec3) => format!("vec3<f32>({})", value),
            (ShaderAttribute::Float, ShaderAttribute::Vec4) => format!("vec4<f32>({})", value),
            (ShaderAttribute::Float, ShaderAttribute::Color) => format!("vec4<f32>({})", value),
            (ShaderAttribute::Float, ShaderAttribute::Bool) => format!("bool({})", value),
            (ShaderAttribute::Float, ShaderAttribute::Mat2) => format!("mat2<f32>({})", value),
            (ShaderAttribute::Float, ShaderAttribute::Mat3) => format!("mat3<f32>({})", value),
            (ShaderAttribute::Float, ShaderAttribute::Mat4) => format!("mat4<f32>({})", value),

            (ShaderAttribute::UInt, ShaderAttribute::Float) => format!("f32({})", value),
            (ShaderAttribute::UInt, ShaderAttribute::SInt) => format!("i32({})", value),
            (ShaderAttribute::UInt, ShaderAttribute::Vec2) => format!("vec2<f32>({})", value),
            (ShaderAttribute::UInt, ShaderAttribute::Vec3) => format!("vec3<f32>({})", value),
            (ShaderAttribute::UInt, ShaderAttribute::Vec4) => format!("vec4<f32>({})", value),
            (ShaderAttribute::UInt, ShaderAttribute::Color) => format!("vec4<f32>({})", value),
            (ShaderAttribute::UInt, ShaderAttribute::Bool) => format!("bool({})", value),
            (ShaderAttribute::UInt, ShaderAttribute::Mat2) => format!("mat2<f32>({})", value),
            (ShaderAttribute::UInt, ShaderAttribute::Mat3) => format!("mat3<f32>({})", value),
            (ShaderAttribute::UInt, ShaderAttribute::Mat4) => format!("mat4<f32>({})", value),

            (ShaderAttribute::SInt, ShaderAttribute::Float) => format!("f32({})", value),
            (ShaderAttribute::SInt, ShaderAttribute::UInt) => format!("u32({})", value),
            (ShaderAttribute::SInt, ShaderAttribute::Vec2) => format!("vec2<f32>({})", value),
            (ShaderAttribute::SInt, ShaderAttribute::Vec3) => format!("vec3<f32>({})", value),
            (ShaderAttribute::SInt, ShaderAttribute::Vec4) => format!("vec4<f32>({})", value),
            (ShaderAttribute::SInt, ShaderAttribute::Color) => format!("vec4<f32>({})", value),
            (ShaderAttribute::SInt, ShaderAttribute::Bool) => format!("bool({})", value),
            (ShaderAttribute::SInt, ShaderAttribute::Mat2) => format!("mat2<f32>({})", value),
            (ShaderAttribute::SInt, ShaderAttribute::Mat3) => format!("mat3<f32>({})", value),
            (ShaderAttribute::SInt, ShaderAttribute::Mat4) => format!("mat4<f32>({})", value),

            (ShaderAttribute::Vec2, ShaderAttribute::Float) => format!("{}[0]", value),
            (ShaderAttribute::Vec2, ShaderAttribute::UInt) => format!("u32({}[0])", value),
            (ShaderAttribute::Vec2, ShaderAttribute::SInt) => format!("i32({}[0])", value),
            (ShaderAttribute::Vec2, ShaderAttribute::Vec3) => format!("vec3<f32>({}, 0.0)", value),
            (ShaderAttribute::Vec2, ShaderAttribute::Vec4) => {
                format!("vec4<f32>({}, 0.0, 0.0)", value)
            }
            (ShaderAttribute::Vec2, ShaderAttribute::Color) => {
                format!("vec4<f32>({}, 0.0, 0.0)", value)
            }
            (ShaderAttribute::Vec2, ShaderAttribute::Mat2) => {
                format!("mat2<f32>({}, 0.0, 0.0, 0.0)", value)
            }
            (ShaderAttribute::Vec2, ShaderAttribute::Mat3) => {
                format!("mat3<f32>({}, 0.0, 0.0, 0.0, 0.0)", value)
            }
            (ShaderAttribute::Vec2, ShaderAttribute::Mat4) => {
                format!("mat4<f32>({}, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)", value)
            }

            (ShaderAttribute::Vec3, ShaderAttribute::Float) => format!("{}[0]", value),
            (ShaderAttribute::Vec3, ShaderAttribute::UInt) => format!("u32({}[0])", value),
            (ShaderAttribute::Vec3, ShaderAttribute::SInt) => format!("i32({}[0])", value),
            (ShaderAttribute::Vec3, ShaderAttribute::Vec2) => format!("vec2<f32>({})", value),
            (ShaderAttribute::Vec3, ShaderAttribute::Vec4) => format!("vec4<f32>({}, 0.0)", value),
            (ShaderAttribute::Vec3, ShaderAttribute::Color) => format!("vec4<f32>({}, 0.0)", value),
            (ShaderAttribute::Vec3, ShaderAttribute::Mat2) => {
                format!("mat2<f32>({}, 0.0, 0.0)", value)
            }
            (ShaderAttribute::Vec3, ShaderAttribute::Mat3) => {
                format!("mat3<f32>({}, 0.0, 0.0, 0.0, 0.0, 0.0)", value)
            }
            (ShaderAttribute::Vec3, ShaderAttribute::Mat4) => {
                format!("mat4<f32>({}, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)", value)
            }

            (ShaderAttribute::Vec4, ShaderAttribute::Float) => format!("{}[0]", value),
            (ShaderAttribute::Vec4, ShaderAttribute::UInt) => format!("u32({}[0])", value),
            (ShaderAttribute::Vec4, ShaderAttribute::SInt) => format!("i32({}[0])", value),
            (ShaderAttribute::Vec4, ShaderAttribute::Vec2) => format!("vec2<f32>({})", value),
            (ShaderAttribute::Vec4, ShaderAttribute::Vec3) => format!("vec3<f32>({})", value),
            (ShaderAttribute::Vec4, ShaderAttribute::Color) => format!("vec4<f32>({})", value),
            (ShaderAttribute::Vec4, ShaderAttribute::Mat2) => {
                format!("mat2<f32>({}, 0.0, 0.0)", value)
            }
            (ShaderAttribute::Vec4, ShaderAttribute::Mat3) => {
                format!("mat3<f32>({}, 0.0, 0.0, 0.0, 0.0, 0.0)", value)
            }
            (ShaderAttribute::Vec4, ShaderAttribute::Mat4) => {
                format!("mat4<f32>({}, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)", value)
            }

            (ShaderAttribute::Color, ShaderAttribute::Float) => format!("{}[0]", value),
            (ShaderAttribute::Color, ShaderAttribute::UInt) => format!("u32({}[0])", value),
            (ShaderAttribute::Color, ShaderAttribute::SInt) => format!("i32({}[0])", value),
            (ShaderAttribute::Color, ShaderAttribute::Vec2) => format!("vec2<f32>({})", value),
            (ShaderAttribute::Color, ShaderAttribute::Vec3) => format!("vec3<f32>({})", value),
            (ShaderAttribute::Color, ShaderAttribute::Vec4) => format!("vec4<f32>({})", value),
            (ShaderAttribute::Color, ShaderAttribute::Mat2) => {
                format!("mat2<f32>({}, 0.0, 0.0)", value)
            }
            (ShaderAttribute::Color, ShaderAttribute::Mat3) => {
                format!("mat3<f32>({}, 0.0, 0.0, 0.0, 0.0, 0.0)", value)
            }
            (ShaderAttribute::Color, ShaderAttribute::Mat4) => {
                format!("mat4<f32>({}, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)", value)
            }

            _ => return None,
        };

        Some(converted)
    }

    fn define_field(name: &str, attribute: ShaderAttribute) -> Option<String> {
        let field = match attribute {
            ShaderAttribute::Float => format!("{}: f32", name),
            ShaderAttribute::UInt => format!("{}: u32", name),
            ShaderAttribute::SInt => format!("{}: i32", name),
            ShaderAttribute::Vec2 => format!("{}: vec2<f32>", name),
            ShaderAttribute::Vec3 => format!("{}: vec3<f32>", name),
            ShaderAttribute::Vec4 => format!("{}: vec4<f32>", name),
            ShaderAttribute::Mat2 => format!("{}: mat2<f32>", name),
            ShaderAttribute::Mat3 => format!("{}: mat3<f32>", name),
            ShaderAttribute::Mat4 => format!("{}: mat4<f32>", name),
            ShaderAttribute::Color => format!("{}: vec4<f32>", name),
            ShaderAttribute::Bool => format!("{}: bool", name),
            _ => return None,
        };

        Some(field)
    }

    pub fn define_struct(name: &str, fields: &[&ShaderInput]) -> Option<String> {
        let mut code = String::new();

        for field in fields {
            let field = Self::define_field(field.name(), field.attribute())?;
            code.push_str(&field);
            code.push_str(",");
        }

        Some(format!("struct {} {{ {} }}", name, code))
    }

    pub fn define_bindings(
        group: usize,
        offset: usize,
        bindings: &[&ShaderInput],
    ) -> Option<String> {
        let mut code = String::new();

        for (binding, input) in bindings.iter().enumerate() {
            let define = match input.attribute() {
                ShaderAttribute::Texture2D => format!("texture_2d<f32>",),
                ShaderAttribute::Texture2DArray => format!("texture_2d_array<f32>",),
                ShaderAttribute::Texture3D => format!("texture_3d<f32>",),
                ShaderAttribute::Texture3DArray => format!("texture_3d_array<f32>",),
                ShaderAttribute::Cubemap => format!("texture_cube<f32>",),
                ShaderAttribute::Sampler => format!("sampler",),
                _ => return None,
            };

            code.push_str(&format!(
                "@group({}) @binding({}) var {}: {};",
                group,
                binding + offset,
                input.name(),
                define
            ));
        }

        Some(code)
    }

    pub fn define_buffer(name: &str, ty: &str, group: usize, binding: usize) -> Option<String> {
        Some(format!(
            "@group({}) @binding({}) var<uniform> {}: {};",
            group, binding, name, ty
        ))
    }
}
