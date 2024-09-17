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
        desc: &RenderPipelineDesc,
        shaders: &RenderAssets<Shader>,
    ) -> Option<Self> {
        None
    }
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct VertexBufferLayout {
    pub array_stride: wgpu::BufferAddress,
    pub step_mode: wgpu::VertexStepMode,
    pub attributes: Vec<wgpu::VertexAttribute>,
}

impl VertexBufferLayout {
    pub fn from(step_mode: wgpu::VertexStepMode, layout: &[MeshAttributeKind]) -> Self {
        let mut offset = 0;
        let mut attributes = vec![];
        for (location, attribute) in layout.iter().enumerate() {
            let attribute = wgpu::VertexAttribute {
                format: attribute.format(),
                offset,
                shader_location: location as u32,
            };
            offset += attribute.format.size();
            attributes.push(attribute);
        }

        Self {
            array_stride: offset as wgpu::BufferAddress,
            step_mode,
            attributes,
        }
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

pub struct RenderPipelineDesc {
    pub layout: Vec<wgpu::BindGroupLayout>,
    pub vertex: VertexState,
    pub fragment: Option<FragmentState>,
    pub primitive: wgpu::PrimitiveState,
    pub depth_write: Option<wgpu::DepthStencilState>,
    pub multisample: wgpu::MultisampleState,
}
