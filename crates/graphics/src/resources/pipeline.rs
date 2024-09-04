use super::{
    binding::ShaderBindGroup,
    material::{Material, MaterialType},
    shader::Shader,
    RenderAssets, ResourceId,
};
use crate::core::{RenderDevice, VertexLayout, VertexLayoutKey};
use ecs::core::{DenseMap, Resource};
use std::{collections::HashMap, hash::Hash};
use wgpu::{
    ColorTargetState, DepthStencilState, MultisampleState, PrimitiveState, PushConstantRange,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexState {
    pub shader: ResourceId,
    pub entry: &'static str,
    pub layout: VertexLayout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentState {
    pub shader: ResourceId,
    pub entry: &'static str,
    pub targets: Vec<Option<ColorTargetState>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPipelineDesc {
    pub bindings: Vec<ShaderBindGroup>,
    pub push_constant_ranges: Vec<PushConstantRange>,
    pub vertex: VertexState,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub multisample: MultisampleState,
    pub fragment: Option<FragmentState>,
}

pub struct RenderPipeline(wgpu::RenderPipeline);

impl RenderPipeline {
    pub fn new(wgpu: wgpu::RenderPipeline) -> Self {
        Self(wgpu)
    }

    pub fn create(
        device: &RenderDevice,
        shaders: &RenderAssets<Shader>,
        desc: &RenderPipelineDesc,
    ) -> Option<Self> {
        let vertex_shader = shaders.get(&desc.vertex.shader)?;
        let layout = desc.vertex.layout.buffer_layout();

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: desc.vertex.entry,
                buffers: &[layout.wgpu()],
                compilation_options: Default::default(),
            },
            fragment: match &desc.fragment {
                Some(fragment) => {
                    let shader = shaders.get(&fragment.shader)?;
                    Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: fragment.entry,
                        targets: &fragment.targets,
                        compilation_options: Default::default(),
                    })
                }
                None => None,
            },
            primitive: desc.primitive,
            depth_stencil: desc.depth_stencil.clone(),
            multisample: desc.multisample,
            multiview: None,
            cache: None,
        });

        Some(Self::new(pipeline))
    }
}

impl std::ops::Deref for RenderPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait MeshPipeline: 'static {
    fn primitive() -> PrimitiveState;
    fn depth_write() -> bool;
}

pub struct Opaque3D;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshPipelineKey(u32);
impl MeshPipelineKey {
    pub fn new<P: MeshPipeline>() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        std::any::TypeId::of::<P>().hash(&mut hasher);
        Self(hasher.finalize())
    }
}

pub struct MeshPipelineInfo {
    states: HashMap<VertexLayoutKey, VertexState>,
    primitive: PrimitiveState,
    depth_write: bool,
}

impl MeshPipelineInfo {
    pub fn new<P: MeshPipeline>() -> Self {
        Self {
            states: HashMap::new(),
            primitive: P::primitive(),
            depth_write: P::depth_write(),
        }
    }

    pub fn add(&mut self, layout: VertexLayout, state: VertexState) {
        let layout_key = VertexLayoutKey::new(layout.attributes());
        self.states.insert(layout_key, state);
    }

    pub fn get(&self, layout: &VertexLayout) -> Option<&VertexState> {
        let layout_key = VertexLayoutKey::new(layout.attributes());
        self.states.get(&layout_key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&VertexLayoutKey, &VertexState)> {
        self.states.iter()
    }

    pub fn primitive(&self) -> &PrimitiveState {
        &self.primitive
    }

    pub fn depth_write(&self) -> bool {
        self.depth_write
    }
}

pub struct MeshPipelineRegistry {
    states: DenseMap<MeshPipelineKey, MeshPipelineInfo>,
}

impl MeshPipelineRegistry {
    pub fn new() -> Self {
        Self {
            states: DenseMap::new(),
        }
    }

    pub fn add<P: MeshPipeline>(&mut self, layout: VertexLayout, state: VertexState) {
        let key = MeshPipelineKey::new::<P>();
        match self.states.get_mut(&key) {
            Some(info) => info.add(layout, state),
            None => {
                let mut info = MeshPipelineInfo::new::<P>();
                info.add(layout, state);
                self.states.insert(key, info);
            }
        }
    }

    pub fn get<P: MeshPipeline>(&self, layout: &VertexLayout) -> Option<&VertexState> {
        let key = MeshPipelineKey::new::<P>();
        self.states.get(&key).and_then(|info| info.get(layout))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&MeshPipelineKey, &MeshPipelineInfo)> {
        self.states.iter()
    }

    pub fn clear(&mut self) {
        self.states.clear();
    }
}

impl Resource for MeshPipelineRegistry {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialPipelineKey(u32);

impl MaterialPipelineKey {
    pub fn new(material_type: MaterialType, mesh_key: MeshPipelineKey) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        material_type.hash(&mut hasher);
        mesh_key.hash(&mut hasher);
        Self(hasher.finalize())
    }

    pub fn from<M: Material>(mesh_key: MeshPipelineKey) -> Self {
        Self::new(MaterialType::new::<M>(), mesh_key)
    }
}

pub struct MaterialPipelines {
    pipelines: DenseMap<MaterialPipelineKey, wgpu::RenderPipeline>,
}

impl MaterialPipelines {
    pub fn new() -> Self {
        Self {
            pipelines: DenseMap::new(),
        }
    }
}
