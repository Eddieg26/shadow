use super::{
    layout::MaterialLayout,
    registry::{MaterialMeta, MaterialRegistry},
    BlendMode, Material, MaterialResource, MaterialType, ShaderModel,
};
use crate::{
    core::RenderDevice,
    resources::{
        binding::BindGroup, texture::GpuTexture, AssetUsage, ExtractedResource, RenderAsset,
        RenderAssetExtractor, RenderAssets,
    },
};
use asset::AssetId;
use ecs::system::{unlifetime::Read, StaticSystemArg};
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[derive(Clone)]
pub struct MaterialInstance {
    pub ty: MaterialType,
    pub model: ShaderModel,
    pub mode: BlendMode,
    binding: Arc<BindGroup>,
}

impl MaterialInstance {
    pub fn new<M: Material>(
        device: &RenderDevice,
        material: &M,
        meta: &MaterialMeta,
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
                    let texture = textures.get(id).unwrap_or(fallback);
                    wgpu::BindingResource::TextureView(texture.view())
                }
                MaterialResource::Sampler(id) => {
                    let texture = textures.get(id).unwrap_or(fallback);
                    wgpu::BindingResource::Sampler(texture.sampler())
                }
            };

            entries.push(wgpu::BindGroupEntry {
                binding: entries.len() as u32,
                resource,
            });
        }

        let binding = BindGroup::create(device, layout, &entries);

        Self {
            ty: meta.ty,
            model: meta.model,
            mode: meta.mode,
            binding: Arc::new(binding),
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }
}

impl RenderAsset for MaterialInstance {
    type Id = AssetId;
}

impl<M: Material> RenderAssetExtractor for M {
    type Source = M;
    type Target = MaterialInstance;
    type Arg = StaticSystemArg<
        'static,
        (
            Read<RenderDevice>,
            Read<MaterialRegistry>,
            Read<RenderAssets<MaterialLayout>>,
            Read<RenderAssets<GpuTexture>>,
        ),
    >;

    fn extract(
        id: &AssetId,
        source: &mut Self::Source,
        arg: &ecs::system::ArgItem<Self::Arg>,
        assets: &mut RenderAssets<Self::Target>,
    ) -> Option<AssetUsage> {
        let (device, registry, layouts, textures) = **arg;
        let ty = MaterialType::of::<M>();
        let meta = registry.get(&ty)?;
        let fallback = textures.get(&AssetId::from("fallback"))?;
        let layout = layouts.get(&ty)?;

        let instance = MaterialInstance::new(device, source, &meta, layout, textures, fallback);
        assets.add(*id, instance);

        Some(AssetUsage::Discard)
    }

    fn remove(id: &AssetId, assets: &mut RenderAssets<Self::Target>) {
        assets.remove(id);
    }

    fn extracted_resource() -> Option<ExtractedResource> {
        Some(ExtractedResource::BindGroup)
    }
}
