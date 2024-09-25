use super::{
    layout::{GlobalBinding, MaterialLayout, ObjectBinding},
    shader::vertex::MeshShader,
    Material, MaterialType,
};
use ecs::core::{DenseMap, Resource};
use graphics::{
    core::RenderDevice,
    renderer::surface::RenderSurface,
    resources::{
        mesh::MeshLayout,
        pipeline::{
            FragmentState, RenderPipeline, RenderPipelineDesc, VertexBufferLayout, VertexState,
        },
        shader::Shader,
        RenderAsset, RenderAssets,
    },
};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DepthWrite {
    On,
    Off,
}

pub trait MaterialPipeline: Send + Sync + 'static {
    fn depth_write() -> DepthWrite;
    fn primitive() -> wgpu::PrimitiveState;
    fn shader() -> MeshShader;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialPipelineKey(u32);

impl MaterialPipelineKey {
    pub fn of<M: MaterialPipeline>() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        std::any::TypeId::of::<M>().hash(&mut hasher);
        Self(hasher.finalize())
    }
}

pub struct MaterialPipelineMeta {
    pub depth_write: DepthWrite,
    pub primitive: wgpu::PrimitiveState,
    shader: fn() -> MeshShader,
}

impl MaterialPipelineMeta {
    pub fn new<M: MaterialPipeline>() -> Self {
        Self {
            depth_write: M::depth_write(),
            primitive: M::primitive(),
            shader: M::shader,
        }
    }

    pub fn shader(&self) -> MeshShader {
        (self.shader)()
    }
}

#[derive(Default)]
pub struct MaterialPipelineRegistry {
    metas: DenseMap<MaterialPipelineKey, MaterialPipelineMeta>,
}

impl MaterialPipelineRegistry {
    pub fn new() -> Self {
        Self {
            metas: DenseMap::new(),
        }
    }

    pub fn register<M: MaterialPipeline>(&mut self) -> MaterialPipelineKey {
        let key = MaterialPipelineKey::of::<M>();
        self.metas.insert(key, MaterialPipelineMeta::new::<M>());
        key
    }

    pub fn get(&self, key: MaterialPipelineKey) -> Option<&MaterialPipelineMeta> {
        self.metas.get(&key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&MaterialPipelineKey, &MaterialPipelineMeta)> {
        self.metas.iter()
    }
}

impl Resource for MaterialPipelineRegistry {}

pub struct MaterialPipelines {
    shader: Shader,
    layout: MeshLayout,
    primitive: wgpu::PrimitiveState,
    depth_write: DepthWrite,
    pipelines: DenseMap<MaterialType, RenderPipeline>,
}

impl MaterialPipelines {
    pub fn create(device: &RenderDevice, meta: &MaterialPipelineMeta) -> Self {
        let shader = meta.shader();
        let layout = shader.layout();
        let primitive = meta.primitive;
        let depth_write = meta.depth_write;
        let shader = Shader::create(device, &meta.shader().generate());

        Self {
            shader,
            layout,
            primitive,
            depth_write,
            pipelines: DenseMap::new(),
        }
    }

    pub fn shader(&self) -> &Shader {
        &self.shader
    }

    pub fn layout(&self) -> &MeshLayout {
        &self.layout
    }

    pub fn primitive(&self) -> wgpu::PrimitiveState {
        self.primitive
    }

    pub fn depth_write(&self) -> DepthWrite {
        self.depth_write
    }

    pub fn pipeline(&self, material: MaterialType) -> Option<&RenderPipeline> {
        self.pipelines.get(&material)
    }

    pub fn add<M: Material>(
        &mut self,
        device: &RenderDevice,
        surface: &RenderSurface,
        layouts: &RenderAssets<MaterialLayout>,
        shaders: &RenderAssets<Shader>,
        fragment_shader: &Shader,
        global_binding: &GlobalBinding,
        object_binding: &ObjectBinding,
    ) -> Option<()> {
        let ty = MaterialType::of::<M>();
        if self.pipelines.contains(&ty) {
            return Some(());
        }

        let global = global_binding.layout();
        let model = object_binding.layout();
        let layout = layouts.get(&ty)?;

        let desc = RenderPipelineDesc {
            label: Some("Material Pipeline"),
            layout: &[global, model, layout],
            vertex: VertexState {
                shader: asset::AssetHandle::Asset(self.shader().clone()),
                entry: std::borrow::Cow::Borrowed("main"),
                buffers: VertexBufferLayout::from(wgpu::VertexStepMode::Vertex, &self.layout),
            },
            fragment: Some(FragmentState {
                shader: asset::AssetHandle::Asset(fragment_shader.clone()),
                entry: std::borrow::Cow::Borrowed("main"),
                targets: vec![Some(wgpu::ColorTargetState {
                    format: surface.format(),
                    blend: Some(M::mode().state()),
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            primitive: self.primitive,
            depth_state: Some(wgpu::DepthStencilState {
                format: surface.depth_format(),
                depth_write_enabled: self.depth_write == DepthWrite::On,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
        };

        let pipeline = RenderPipeline::create(device, desc, shaders)?;
        self.pipelines.insert(ty, pipeline);

        Some(())
    }

    pub fn remove(&mut self, material: MaterialType) {
        self.pipelines.remove(&material);
    }
}

impl RenderAsset for MaterialPipelines {
    type Id = MaterialPipelineKey;
}
