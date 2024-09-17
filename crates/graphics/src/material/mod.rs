use crate::{
    core::{Color, RenderDevice},
    resources::{
        texture::{GpuTexture, TextureDimension},
        AssetUsage, ExtractedResource, RenderAsset, RenderAssetExtractor, RenderAssets, ResourceId,
    },
};
use asset::{Asset, AssetId};
use ecs::core::Resource;
use shader::fragment::MaterialShader;
use std::{
    hash::Hash,
    ops::Deref,
    sync::{Arc, RwLock},
};
use wgpu::util::DeviceExt;

pub mod pipeline;
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

#[derive(Debug, Clone, Copy)]
pub struct MaterialMeta {
    pub ty: MaterialType,
    pub model: ShaderModel,
    pub mode: BlendMode,
    pub shader: fn() -> MaterialShader,
    pub layout: fn(&RenderDevice) -> MaterialLayout,
}

impl MaterialMeta {
    pub fn new<M: Material>() -> Self {
        Self {
            ty: MaterialType::new::<M>(),
            shader: M::shader,
            model: M::model(),
            mode: M::mode(),
            layout: |device| MaterialLayout::create::<M>(device),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MaterialRegistry {
    metas: Arc<RwLock<Vec<MaterialMeta>>>,
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
        metas.push(meta);
    }

    pub fn get(&self, ty: &MaterialType) -> Option<MaterialMeta> {
        let metas = self.metas.read().unwrap();
        metas.iter().find(|meta| &meta.ty == ty).copied()
    }

    pub fn metas(&self) -> std::sync::RwLockReadGuard<'_, Vec<MaterialMeta>> {
        self.metas.read().unwrap()
    }

    pub fn create_metas(
        _: &[()],
        device: &RenderDevice,
        registry: &Self,
        layouts: &mut RenderAssets<MaterialLayout>,
    ) {
        for meta in registry.metas.read().unwrap().iter() {
            layouts.add(meta.ty, (meta.layout)(device));
        }
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

#[derive(Clone)]
pub struct MaterialInstance {
    meta: MaterialMeta,
    binding: Arc<wgpu::BindGroup>,
}

impl MaterialInstance {
    pub fn new<M: Material>(
        device: &RenderDevice,
        material: &M,
        meta: MaterialMeta,
        layout: &MaterialLayout,
        textures: &RenderAssets<GpuTexture>,
        fallback: &GpuTexture,
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

        for resource in info.resources() {
            let resource = match resource {
                MaterialResource::Texture(id) => {
                    let texture = textures.get(&id.into()).unwrap_or(fallback);
                    wgpu::BindingResource::TextureView(texture.view())
                }
                MaterialResource::Sampler(id) => {
                    let texture = textures.get(&id.into()).unwrap_or(fallback);
                    wgpu::BindingResource::Sampler(texture.sampler())
                }
            };

            entries.push(wgpu::BindGroupEntry {
                binding: entries.len() as u32,
                resource,
            });
        }

        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &entries,
        });

        Self {
            meta,
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

impl RenderAsset for MaterialInstance {
    type Id = ResourceId;
}

pub struct MaterialLayout(wgpu::BindGroupLayout);

impl MaterialLayout {
    pub fn create<M: Material>(device: &RenderDevice) -> Self {
        let mut entries = vec![];
        let offset = match M::layout()
            .iter()
            .any(|b| matches!(b, MaterialBinding::Buffer))
        {
            true => {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                });
                1
            }
            false => 0,
        };

        for binding in M::layout() {
            match binding {
                MaterialBinding::Texture(dimension) => entries.push(wgpu::BindGroupLayoutEntry {
                    binding: offset + entries.len() as u32,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: (*dimension).into(),
                        multisampled: false,
                    },
                    count: None,
                }),
                MaterialBinding::Sampler => todo!(),
                _ => {}
            }
        }

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        });

        Self(layout)
    }

    #[inline]
    pub fn inner(&self) -> &wgpu::BindGroupLayout {
        &self.0
    }
}

impl From<wgpu::BindGroupLayout> for MaterialLayout {
    fn from(layout: wgpu::BindGroupLayout) -> Self {
        Self(layout)
    }
}

impl std::ops::Deref for MaterialLayout {
    type Target = wgpu::BindGroupLayout;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RenderAsset for MaterialLayout {
    type Id = MaterialType;
}

impl<M: Material> RenderAssetExtractor for M {
    type Source = M;
    type Target = MaterialInstance;
    type Arg<'a> = (
        &'a RenderDevice,
        &'a MaterialRegistry,
        &'a RenderAssets<MaterialLayout>,
        &'a RenderAssets<GpuTexture>,
    );

    fn extract<'a>(
        source: &mut Self::Source,
        arg: &ecs::system::ArgItem<Self::Arg<'a>>,
    ) -> Option<Self::Target> {
        let (device, registry, layouts, textures) = arg;
        let ty = MaterialType::new::<M>();
        let meta = registry.get(&ty)?;
        let fallback = textures.get(&ResourceId::from("fallback"))?;
        let layout = layouts.get(&ty)?;

        Some(MaterialInstance::new(
            device, source, meta, layout, textures, fallback,
        ))
    }

    fn usage(_: &Self::Source) -> AssetUsage {
        AssetUsage::Discard
    }

    fn extracted_resource() -> Option<ExtractedResource> {
        Some(ExtractedResource::BindGroup)
    }
}
