use asset::AssetId;
use ecs::core::Resource;

use super::{shader::vertex::MeshShader, Material, MaterialRegistry, MaterialType};
use crate::{
    core::RenderDevice,
    resources::{
        mesh::MeshLayout,
        pipeline::{RenderPipeline, RenderPipelineDesc},
        shader::Shader,
        RenderAsset, RenderAssetExtractor, RenderAssets,
    },
};
use std::{hash::Hash, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DepthWrite {
    On,
    Off,
}

pub trait MeshPipeline: 'static {
    fn layout() -> MeshLayout;
    fn shader() -> MeshShader;
    fn depth_write() -> DepthWrite;
    fn primitive() -> wgpu::PrimitiveState;
}

pub struct MeshPipelineInfo {
    pub key: MeshPipelineKey,
    pub layout: MeshLayout,
    pub shader: Arc<MeshShader>,
    pub depth_write: DepthWrite,
    pub primitive: wgpu::PrimitiveState,
}

impl MeshPipelineInfo {
    pub fn new<P: MeshPipeline>() -> Self {
        Self {
            key: MeshPipelineKey::new::<P>(),
            layout: P::layout(),
            shader: Arc::new(P::shader()),
            depth_write: P::depth_write(),
            primitive: P::primitive(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MeshPipelineKey(u32);
impl MeshPipelineKey {
    pub fn new<M: MeshPipeline>() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        std::any::TypeId::of::<M>().hash(&mut hasher);
        Self(hasher.finalize())
    }

    pub fn raw(value: u32) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for MeshPipelineKey {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct MeshPipelines {
    pipelines: Vec<MeshPipelineInfo>,
}

impl MeshPipelines {
    pub fn new() -> Self {
        Self {
            pipelines: Vec::new(),
        }
    }

    pub fn add<P: MeshPipeline>(&mut self) {
        self.pipelines.push(MeshPipelineInfo::new::<P>());
    }

    pub fn get(&self, key: MeshPipelineKey) -> Option<&MeshPipelineInfo> {
        self.pipelines.iter().find(|p| p.key == key)
    }

    pub fn iter(&self) -> impl Iterator<Item = &MeshPipelineInfo> {
        self.pipelines.iter()
    }
}

impl Resource for MeshPipelines {}

pub struct MaterialPipeline(RenderPipeline);

impl From<RenderPipeline> for MaterialPipeline {
    fn from(pipeline: RenderPipeline) -> Self {
        Self(pipeline)
    }
}

impl std::ops::Deref for MaterialPipeline {
    type Target = RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialPipelineKey {
    pub material: MaterialType,
    pub mesh: MeshPipelineKey,
}

impl From<(MaterialType, MeshPipelineKey)> for MaterialPipelineKey {
    fn from((material, mesh): (MaterialType, MeshPipelineKey)) -> Self {
        Self { material, mesh }
    }
}

impl From<AssetId> for MaterialPipelineKey {
    fn from(_: AssetId) -> Self {
        Self {
            material: MaterialType::raw(0),
            mesh: MeshPipelineKey(0),
        }
    }
}

impl RenderAsset for MaterialPipeline {
    type Id = MaterialPipelineKey;
}

pub struct MaterialPipelineExtractor<M: Material>(std::marker::PhantomData<M>);

impl<M: Material> RenderAssetExtractor for MaterialPipelineExtractor<M> {
    type Source = M;
    type Target = MaterialPipeline;
    type Arg<'a> = (
        &'a RenderDevice,
        &'a MeshPipelines,
        &'a MaterialRegistry,
        &'a RenderAssets<Shader>,
    );

    fn extract<'a>(
        source: &mut Self::Source,
        arg: &ecs::system::ArgItem<Self::Arg<'a>>,
    ) -> Option<Self::Target> {
        let (device, mesh, registry, shaders) = arg;
        let ty = MaterialType::new::<M>();
        for pipeline in mesh.iter() {
            let key = MaterialPipelineKey::from((ty, pipeline.key));
            if let Some(material) = registry.get(&ty) {
                let desc = RenderPipelineDesc {
                    layout: todo!(),
                    vertex: todo!(),
                    fragment: todo!(),
                    primitive: todo!(),
                    depth_write: todo!(),
                    multisample: todo!(),
                };

                let pipeline = RenderPipeline::create(device, &desc, shaders);
            }
        }
        todo!()
    }
}
