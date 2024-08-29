use crate::core::Color;
use asset::AssetId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShaderAttribute {
    Float,
    UInt,
    SInt,
    Vec2,
    Vec3,
    Vec4,
    Mat2,
    Mat3,
    Mat4,
    Color,
    Bool,
    Texture2D,
    Texture2DArray,
    Texture3D,
    Texture3DArray,
    Cubemap,
    Sampler,
    Dynamic,
}

impl ShaderAttribute {
    pub fn is_buffer_value(&self) -> bool {
        matches!(
            self,
            ShaderAttribute::Vec2
                | ShaderAttribute::Vec3
                | ShaderAttribute::Vec4
                | ShaderAttribute::Color
                | ShaderAttribute::Mat2
                | ShaderAttribute::Mat3
                | ShaderAttribute::Mat4
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ShaderValue {
    Float(f32),
    UInt(u32),
    SInt(i32),
    Vec2(glam::Vec2),
    Vec3(glam::Vec3),
    Vec4(glam::Vec4),
    Mat2(glam::Mat2),
    Mat3(glam::Mat3),
    Mat4(glam::Mat4),
    Color(Color),
    Bool(bool),
    Texture2D(AssetId),
    Texture2DArray(AssetId),
    Texture3D(AssetId),
    Texture3DArray(AssetId),
    Cubemap(AssetId),
    Sampler(AssetId),
    Dynamic,
}

impl ShaderValue {
    pub fn attribute(&self) -> ShaderAttribute {
        match self {
            ShaderValue::Float(_) => ShaderAttribute::Float,
            ShaderValue::UInt(_) => ShaderAttribute::UInt,
            ShaderValue::SInt(_) => ShaderAttribute::SInt,
            ShaderValue::Vec2(_) => ShaderAttribute::Vec2,
            ShaderValue::Vec3(_) => ShaderAttribute::Vec3,
            ShaderValue::Vec4(_) => ShaderAttribute::Vec4,
            ShaderValue::Mat2(_) => ShaderAttribute::Mat2,
            ShaderValue::Mat3(_) => ShaderAttribute::Mat3,
            ShaderValue::Mat4(_) => ShaderAttribute::Mat4,
            ShaderValue::Color(_) => ShaderAttribute::Color,
            ShaderValue::Bool(_) => ShaderAttribute::Bool,
            ShaderValue::Texture2D(_) => ShaderAttribute::Texture2D,
            ShaderValue::Texture2DArray(_) => ShaderAttribute::Texture2DArray,
            ShaderValue::Texture3D(_) => ShaderAttribute::Texture3D,
            ShaderValue::Texture3DArray(_) => ShaderAttribute::Texture3DArray,
            ShaderValue::Cubemap(_) => ShaderAttribute::Cubemap,
            ShaderValue::Sampler(_) => ShaderAttribute::Sampler,
            ShaderValue::Dynamic => ShaderAttribute::Dynamic,
        }
    }
}
