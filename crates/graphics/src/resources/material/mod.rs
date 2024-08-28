use crate::core::Color;
use asset::{Asset, AssetId};

pub mod shader;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderModel {
    Unlit = 0,
    Lit = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Opaque = 0,
    Translucent = 1,
}

pub trait Material: Asset + Clone + Sized {
    fn info(&self) -> MaterialInfo;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShaderInput {
    Color(Color),
    Texture(AssetId),
    Float(f32),
    UInt(u32),
    SInt(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialPropertyKind {
    Color,
    Normal,
    Specular,
    Metallic,
    Roughness,
    Emissive,
    Opacity,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialProperty {
    Color(ShaderInput),
    Normal(ShaderInput),
    Specular(ShaderInput),
    Metallic(ShaderInput),
    Roughness(ShaderInput),
    Emissive(ShaderInput),
    Opacity(ShaderInput),
}

impl MaterialProperty {
    pub fn kind(&self) -> MaterialPropertyKind {
        match self {
            Self::Color(_) => MaterialPropertyKind::Color,
            Self::Normal(_) => MaterialPropertyKind::Normal,
            Self::Specular(_) => MaterialPropertyKind::Specular,
            Self::Metallic(_) => MaterialPropertyKind::Metallic,
            Self::Roughness(_) => MaterialPropertyKind::Roughness,
            Self::Emissive(_) => MaterialPropertyKind::Emissive,
            Self::Opacity(_) => MaterialPropertyKind::Opacity,
        }
    }
}

pub struct MaterialInfo {
    shader_model: ShaderModel,
    blend_mode: BlendMode,
    properties: Vec<MaterialProperty>,
}

impl MaterialInfo {
    pub fn new(model: ShaderModel, mode: BlendMode) -> Self {
        Self {
            shader_model: model,
            blend_mode: mode,
            properties: vec![],
        }
    }

    pub fn shader_model(&self) -> ShaderModel {
        self.shader_model
    }

    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    pub fn properties(&self) -> &[MaterialProperty] {
        &self.properties
    }

    pub fn with_normal(mut self, normal: ShaderInput) -> Self {
        self.properties.push(MaterialProperty::Normal(normal));
        self
    }

    pub fn with_specular(mut self, specular: ShaderInput) -> Self {
        self.properties.push(MaterialProperty::Specular(specular));
        self
    }

    pub fn with_metallic(mut self, metallic: ShaderInput) -> Self {
        self.properties.push(MaterialProperty::Metallic(metallic));
        self
    }

    pub fn with_roughness(mut self, roughness: ShaderInput) -> Self {
        self.properties.push(MaterialProperty::Roughness(roughness));
        self
    }

    pub fn with_emissive(mut self, emissive: ShaderInput) -> Self {
        self.properties.push(MaterialProperty::Emissive(emissive));
        self
    }

    pub fn with_opacity(mut self, opacity: ShaderInput) -> Self {
        self.properties.push(MaterialProperty::Opacity(opacity));
        self
    }
}
