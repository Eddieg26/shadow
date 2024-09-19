use crate::{core::Color, resources::texture::TextureDimension};
use asset::{Asset, AssetId};
use shader::fragment::MaterialShader;
use std::{hash::Hash, ops::Deref};

pub mod instance;
pub mod layout;
pub mod pipeline;
pub mod registry;
pub mod shader;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderModel {
    Unlit,
    Lit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Opaque,
    Transparent,
}

impl BlendMode {
    pub fn state(&self) -> wgpu::BlendState {
        match self {
            Self::Opaque => wgpu::BlendState::REPLACE,
            Self::Transparent => wgpu::BlendState::ALPHA_BLENDING,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialBinding {
    Buffer,
    Texture(TextureDimension),
    Sampler,
}

pub trait Material: Asset + Clone + 'static {
    fn model() -> ShaderModel;
    fn mode() -> BlendMode;
    fn shader() -> MaterialShader;
    fn layout() -> &'static [MaterialBinding];
    fn info(&self) -> MaterialInfo;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialType(u32);

impl MaterialType {
    pub fn of<M: Material>() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        std::any::TypeId::of::<M>().hash(&mut hasher);
        Self(hasher.finalize())
    }

    pub fn raw(id: u32) -> Self {
        Self(id)
    }
}

impl From<AssetId> for MaterialType {
    fn from(id: AssetId) -> Self {
        Self((*(id.deref())) as u32)
    }
}

impl std::ops::Deref for MaterialType {
    type Target = u32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterialValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Color(Color),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterialResource {
    Texture(AssetId),
    Sampler(AssetId),
}

#[derive(Debug, Clone)]
pub struct MaterialInfo {
    values: Vec<MaterialValue>,
    resources: Vec<MaterialResource>,
}

impl MaterialInfo {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            resources: Vec::new(),
        }
    }

    pub fn add_value(&mut self, value: MaterialValue) {
        self.values.push(value);
    }

    pub fn add_texture(&mut self, texture: AssetId) {
        self.resources.push(MaterialResource::Texture(texture));
    }

    pub fn add_sampler(&mut self, sampler: AssetId) {
        self.resources.push(MaterialResource::Sampler(sampler));
    }

    pub fn values(&self) -> &[MaterialValue] {
        &self.values
    }

    pub fn resources(&self) -> &[MaterialResource] {
        &self.resources
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in &self.values {
            match value {
                MaterialValue::Float(value) => bytes.extend_from_slice(bytemuck::bytes_of(value)),
                MaterialValue::Vec2(value) => bytes.extend_from_slice(bytemuck::cast_slice(value)),
                MaterialValue::Vec3(value) => bytes.extend_from_slice(bytemuck::cast_slice(value)),
                MaterialValue::Vec4(value) => bytes.extend_from_slice(bytemuck::cast_slice(value)),
                MaterialValue::Color(value) => bytes.extend_from_slice(bytemuck::bytes_of(value)),
            }
        }
        bytes
    }
}
