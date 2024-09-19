use super::{layout::MaterialLayouts, shader::vertex::MeshShader, Material, MaterialType};
use crate::{
    core::RenderDevice,
    renderer::surface::RenderSurface,
    resources::{
        mesh::MeshLayout,
        pipeline::{
            FragmentState, RenderPipeline, RenderPipelineDesc, VertexBufferLayout, VertexState,
        },
        shader::Shader,
        AssetUsage, RenderAsset, RenderAssetExtractor, RenderAssets,
    },
};
use ecs::{
    core::DenseMap,
    system::{unlifetime::Read, StaticSystemArg},
};
use std::{hash::Hash, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DepthWrite {
    On,
    Off,
}

pub trait MaterialPipeline: 'static {
    fn layout() -> MeshLayout;
    fn shader() -> MeshShader;
    fn depth_write() -> DepthWrite;
    fn primitive() -> wgpu::PrimitiveState;
}

pub struct MaterialPipelines {
    pipelines: DenseMap<MaterialType, RenderPipeline>,
    layout: MeshLayout,
    shader: Arc<Option<Shader>>,
    depth_write: DepthWrite,
    primitive: wgpu::PrimitiveState,
    create_shader: fn() -> MeshShader,
}

impl MaterialPipelines {
    pub fn new<M: MaterialPipeline>() -> Self {
        Self {
            pipelines: DenseMap::new(),
            layout: M::layout(),
            shader: Arc::new(None),
            depth_write: M::depth_write(),
            primitive: M::primitive(),
            create_shader: M::shader,
        }
    }

    pub fn pipelines(&self) -> impl Iterator<Item = (&MaterialType, &RenderPipeline)> + '_ {
        self.pipelines.iter()
    }

    pub fn pipeline(&self, ty: MaterialType) -> Option<&RenderPipeline> {
        self.pipelines.get(&ty)
    }

    pub fn layout(&self) -> &MeshLayout {
        &self.layout
    }

    pub fn shader(&self) -> Option<&Shader> {
        match self.shader.as_ref() {
            Some(shader) => Some(shader),
            None => None,
        }
    }

    pub fn depth_write(&self) -> DepthWrite {
        self.depth_write
    }

    pub fn primitive(&self) -> &wgpu::PrimitiveState {
        &self.primitive
    }

    pub fn create_shader(&mut self, device: &RenderDevice) {
        if self.shader.is_none() {
            let source = (self.create_shader)().generate();
            let shader = Shader::create(device, &source);
            self.shader = Arc::new(Some(shader));
        }
    }

    pub fn add_pipeline<M: Material>(
        &mut self,
        device: &RenderDevice,
        surface: &RenderSurface,
        layouts: &MaterialLayouts,
        shaders: &RenderAssets<Shader>,
    ) -> Option<()> {
        let ty = MaterialType::of::<M>();
        if self.pipelines.contains(&ty) {
            return None;
        }

        self.create_shader(device);

        let global = layouts.global()?;
        let model = layouts.model()?;
        let layout = layouts.layout(ty)?;

        let fragment_shader = Shader::create(device, &M::shader().generate());

        let desc = RenderPipelineDesc {
            layout: &[global, model, layout],
            vertex: VertexState {
                shader: asset::AssetHandle::Asset(self.shader()?.clone()),
                entry: std::borrow::Cow::Borrowed("main"),
                buffers: vec![VertexBufferLayout::from(
                    wgpu::VertexStepMode::Vertex,
                    &self.layout,
                )],
            },
            fragment: Some(FragmentState {
                shader: asset::AssetHandle::Asset(fragment_shader),
                entry: std::borrow::Cow::Borrowed("main"),
                targets: vec![Some(wgpu::ColorTargetState {
                    format: surface.format(),
                    blend: Some(M::mode().state()),
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            primitive: self.primitive,
            depth_state: match self.depth_write {
                DepthWrite::On => Some(wgpu::DepthStencilState {
                    format: surface.depth_format(),
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                DepthWrite::Off => None,
            },
            multisample: Default::default(),
        };

        self.pipelines
            .insert(ty, RenderPipeline::create(device, desc, shaders)?);

        Some(())
    }
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

impl RenderAsset for MaterialPipelines {
    type Id = MaterialPipelineKey;
}

pub struct MaterialPipelineExtractor<M: Material>(std::marker::PhantomData<M>);

impl<M: Material> RenderAssetExtractor for MaterialPipelineExtractor<M> {
    type Source = M;
    type Target = MaterialPipelines;
    type Arg = StaticSystemArg<
        'static,
        (
            Read<RenderDevice>,
            Read<RenderSurface>,
            Read<MaterialLayouts>,
            Read<RenderAssets<Shader>>,
        ),
    >;

    fn extract(
        _: &asset::AssetId,
        _: &mut Self::Source,
        arg: &ecs::system::ArgItem<Self::Arg>,
        pipelines: &mut RenderAssets<Self::Target>,
    ) -> Option<AssetUsage> {
        let (device, surface, layouts, shaders) = **arg;
        for (_, pipelines) in pipelines.iter_mut() {
            pipelines.add_pipeline::<M>(device, surface, layouts, shaders);
        }

        Some(AssetUsage::Discard)
    }

    fn remove(_: &asset::AssetId, _: &mut RenderAssets<Self::Target>) {}
}
