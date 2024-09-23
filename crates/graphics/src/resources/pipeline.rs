use super::{mesh::MeshAttributeKind, shader::Shader, RenderAssets};
use crate::core::RenderDevice;
use asset::AssetHandle;
use std::borrow::Cow;

pub struct RenderPipeline(wgpu::RenderPipeline);
impl std::ops::Deref for RenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RenderPipeline {
    pub fn create(
        device: &RenderDevice,
        desc: RenderPipelineDesc,
        shaders: &RenderAssets<Shader>,
    ) -> Option<Self> {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: desc.layout,
            push_constant_ranges: &[],
        });

        let vertex_shader = match &desc.vertex.shader {
            AssetHandle::Id(id) => shaders.get(id)?,
            AssetHandle::Asset(shader) => shader,
        };

        let vertex_buffer_layouts = desc
            .vertex
            .buffers
            .iter()
            .map(|layout| wgpu::VertexBufferLayout {
                array_stride: layout.array_stride,
                step_mode: layout.step_mode,
                attributes: &layout.attributes,
            })
            .collect::<Vec<_>>();

        let vertex = wgpu::VertexState {
            module: vertex_shader.module(),
            entry_point: &desc.vertex.entry,
            compilation_options: Default::default(),
            buffers: &vertex_buffer_layouts,
        };

        let fragment = match &desc.fragment {
            Some(state) => Some(wgpu::FragmentState {
                module: match &state.shader {
                    AssetHandle::Id(id) => shaders.get(id)?.module(),
                    AssetHandle::Asset(shader) => shader.module(),
                },
                entry_point: &state.entry,
                compilation_options: Default::default(),
                targets: &state.targets,
            }),
            None => None,
        };

        let desc = wgpu::RenderPipelineDescriptor {
            label: desc.label,
            layout: Some(&layout),
            vertex,
            primitive: desc.primitive,
            depth_stencil: desc.depth_state,
            fragment,
            multisample: desc.multisample,
            multiview: None,
            cache: None,
        };

        Some(RenderPipeline(device.create_render_pipeline(&desc)))
    }

    pub fn inner(&self) -> &wgpu::RenderPipeline {
        &self.0
    }
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct VertexBufferLayout {
    pub array_stride: wgpu::BufferAddress,
    pub step_mode: wgpu::VertexStepMode,
    pub attributes: Vec<wgpu::VertexAttribute>,
}

impl VertexBufferLayout {
    // pub fn from(step_mode: wgpu::VertexStepMode, layout: &[MeshAttributeKind]) -> Self {
    //     let mut offset = 0;
    //     let mut attributes = vec![];
    //     for (location, attribute) in layout.iter().enumerate() {
    //         let attribute = wgpu::VertexAttribute {
    //             format: attribute.format(),
    //             offset,
    //             shader_location: location as u32,
    //         };
    //         offset += attribute.format.size();
    //         attributes.push(attribute);
    //     }

    //     Self {
    //         array_stride: offset as wgpu::BufferAddress,
    //         step_mode,
    //         attributes,
    //     }
    // }

    pub fn from(step_mode: wgpu::VertexStepMode, layout: &[MeshAttributeKind]) -> Vec<Self> {
        layout
            .iter()
            .enumerate()
            .map(|(location, attribute)| Self {
                array_stride: attribute.size() as wgpu::BufferAddress,
                step_mode,
                attributes: vec![wgpu::VertexAttribute {
                    format: attribute.format(),
                    offset: 0,
                    shader_location: location as u32,
                }],
            })
            .collect()
    }
}

pub struct VertexState {
    pub shader: AssetHandle<Shader>,
    pub entry: Cow<'static, str>,
    pub buffers: Vec<VertexBufferLayout>,
}

pub struct FragmentState {
    pub shader: AssetHandle<Shader>,
    pub entry: Cow<'static, str>,
    pub targets: Vec<Option<wgpu::ColorTargetState>>,
}

pub struct RenderPipelineDesc<'a> {
    pub label: Option<&'a str>,
    pub layout: &'a [&'a wgpu::BindGroupLayout],
    pub vertex: VertexState,
    pub fragment: Option<FragmentState>,
    pub primitive: wgpu::PrimitiveState,
    pub depth_state: Option<wgpu::DepthStencilState>,
    pub multisample: wgpu::MultisampleState,
}
