use super::{texture::GPUTexture, RenderAsset, RenderAssets};
use crate::core::{Color, RenderDevice};
use asset::{Asset, AssetId};
use ecs::core::{DenseMap, Resource};
use std::{
    hash::Hash,
    sync::{Arc, RwLock},
};
use wgpu::util::DeviceExt;

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

pub trait Material: Asset + Clone + 'static {
    fn fragment_shader() -> AssetId;
    fn model() -> ShaderModel;
    fn mode() -> BlendMode;
    fn info(&self) -> MaterialInfo;
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialMeta {
    pub ty: MaterialType,
    pub fragment: AssetId,
    pub model: ShaderModel,
    pub mode: BlendMode,
}

impl MaterialMeta {
    pub fn new<M: Material>() -> Self {
        Self {
            ty: MaterialType::new::<M>(),
            fragment: M::fragment_shader(),
            model: M::model(),
            mode: M::mode(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MaterialRegistry {
    metas: Arc<RwLock<DenseMap<MaterialType, MaterialMeta>>>,
}

impl MaterialRegistry {
    pub fn new() -> Self {
        Self {
            metas: Arc::default(),
        }
    }

    pub fn register<M: Material>(&mut self) {
        let meta = MaterialMeta::new::<M>();
        let mut metas = self.metas.write().unwrap();
        metas.insert(meta.ty, meta);
    }

    pub fn get(&self, ty: &MaterialType) -> Option<MaterialMeta> {
        let metas = self.metas.read().unwrap();
        metas.get(ty).copied()
    }
}

impl Resource for MaterialRegistry {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialType(u32);

impl MaterialType {
    pub fn new<M: Material>() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        std::any::TypeId::of::<M>().hash(&mut hasher);
        Self(hasher.finalize())
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

#[derive(Debug, Clone)]
pub struct MaterialInfo {
    buffer: Vec<MaterialValue>,
    textures: Vec<AssetId>,
}

impl MaterialInfo {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            textures: Vec::new(),
        }
    }

    pub fn add_value(&mut self, value: MaterialValue) {
        self.buffer.push(value);
    }

    pub fn add_texture(&mut self, texture: AssetId) {
        self.textures.push(texture);
    }

    pub fn textures(&self) -> &[AssetId] {
        &self.textures
    }

    pub fn buffer(&self) -> &[MaterialValue] {
        &self.buffer
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in &self.buffer {
            match value {
                MaterialValue::Float(value) => bytes.extend_from_slice(&value.to_ne_bytes()),
                MaterialValue::Vec2(value) => bytes.extend_from_slice(bytemuck::cast_slice(value)),
                MaterialValue::Vec3(value) => bytes.extend_from_slice(bytemuck::cast_slice(value)),
                MaterialValue::Vec4(value) => bytes.extend_from_slice(bytemuck::cast_slice(value)),
                MaterialValue::Color(value) => bytes.extend_from_slice(bytemuck::bytes_of(value)),
            }
        }
        bytes
    }
}

#[derive(Clone)]
pub struct MaterialInstance {
    meta: MaterialMeta,
    binding: Arc<wgpu::BindGroup>,
}

impl MaterialInstance {
    pub fn new<M: Material>(
        device: &RenderDevice,
        layout: &wgpu::BindGroupLayout,
        textures: &RenderAssets<GPUTexture>,
        fallback: &GPUTexture,
        material: M,
    ) -> Self {
        let info = material.info();
        let bytes = info.bytes();
        let buffer = match bytes.is_empty() {
            true => None,
            false => Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: &bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                }),
            ),
        };

        let mut entries = Vec::new();
        buffer.as_ref().and_then(|buffer| {
            entries.push(wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            });
            Some(buffer)
        });

        for texture in info.textures() {
            let texture = match textures.get(&texture.into()) {
                Some(texture) => texture,
                None => fallback,
            };

            entries.push(wgpu::BindGroupEntry {
                binding: entries.len() as u32,
                resource: wgpu::BindingResource::TextureView(texture.view()),
            });

            entries.push(wgpu::BindGroupEntry {
                binding: entries.len() as u32,
                resource: wgpu::BindingResource::Sampler(texture.sampler()),
            });
        }

        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &entries,
        });

        Self {
            meta: MaterialMeta::new::<M>(),
            binding: Arc::new(binding),
        }
    }

    pub fn ty(&self) -> &MaterialType {
        &self.meta.ty
    }

    pub fn mode(&self) -> BlendMode {
        self.meta.mode
    }

    pub fn model(&self) -> ShaderModel {
        self.meta.model
    }

    pub fn binding(&self) -> &wgpu::BindGroup {
        &self.binding
    }
}

impl RenderAsset for MaterialInstance {}
