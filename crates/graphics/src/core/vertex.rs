use super::Color;
use glam::{Vec2, Vec3, Vec4};
use std::{hash::Hash, ops::Range};
use wgpu::{BufferAddress, VertexStepMode};

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum VertexAttribute {
    Position,
    TexCoord0,
    TexCoord1,
    Normal,
    Tangent,
    Color,
}

impl VertexAttribute {
    pub fn size(&self) -> usize {
        match self {
            VertexAttribute::Position => std::mem::size_of::<Vec3>(),
            VertexAttribute::Normal => std::mem::size_of::<Vec3>(),
            VertexAttribute::Tangent => std::mem::size_of::<Vec4>(),
            VertexAttribute::TexCoord0 => std::mem::size_of::<Vec2>(),
            VertexAttribute::TexCoord1 => std::mem::size_of::<Vec2>(),
            VertexAttribute::Color => std::mem::size_of::<Color>(),
        }
    }

    pub fn format(&self) -> wgpu::VertexFormat {
        match self {
            VertexAttribute::Position => wgpu::VertexFormat::Float32x3,
            VertexAttribute::Normal => wgpu::VertexFormat::Float32x3,
            VertexAttribute::Tangent => wgpu::VertexFormat::Float32x4,
            VertexAttribute::TexCoord0 => wgpu::VertexFormat::Float32x2,
            VertexAttribute::TexCoord1 => wgpu::VertexFormat::Float32x2,
            VertexAttribute::Color => wgpu::VertexFormat::Float64x4,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum VertexAttributeValues {
    Position(Vec<Vec3>),
    Normal(Vec<Vec3>),
    Tangent(Vec<Vec4>),
    TexCoord0(Vec<Vec2>),
    TexCoord1(Vec<Vec2>),
    Color(Vec<Color>),
}

impl VertexAttributeValues {
    pub fn kind(&self) -> VertexAttribute {
        match self {
            VertexAttributeValues::Position(_) => VertexAttribute::Position,
            VertexAttributeValues::Normal(_) => VertexAttribute::Normal,
            VertexAttributeValues::Tangent(_) => VertexAttribute::Tangent,
            VertexAttributeValues::TexCoord0(_) => VertexAttribute::TexCoord0,
            VertexAttributeValues::TexCoord1(_) => VertexAttribute::TexCoord1,
            VertexAttributeValues::Color(_) => VertexAttribute::Color,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            VertexAttributeValues::Position(pos) => pos.len(),
            VertexAttributeValues::Normal(v) => v.len(),
            VertexAttributeValues::Tangent(v) => v.len(),
            VertexAttributeValues::TexCoord0(v) => v.len(),
            VertexAttributeValues::TexCoord1(v) => v.len(),
            VertexAttributeValues::Color(v) => v.len(),
        }
    }

    pub fn data(&self, range: Range<usize>) -> Vec<u8> {
        match self {
            VertexAttributeValues::Position(v) => bytemuck::cast_slice(&v[range]).to_vec(),
            VertexAttributeValues::Normal(v) => bytemuck::cast_slice(&v[range]).to_vec(),
            VertexAttributeValues::Tangent(v) => bytemuck::cast_slice(&v[range]).to_vec(),
            VertexAttributeValues::TexCoord0(v) => bytemuck::cast_slice(&v[range]).to_vec(),
            VertexAttributeValues::TexCoord1(v) => bytemuck::cast_slice(&v[range]).to_vec(),
            VertexAttributeValues::Color(v) => bytemuck::cast_slice(&v[range]).to_vec(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            VertexAttributeValues::Position(v) => v.clear(),
            VertexAttributeValues::Normal(v) => v.clear(),
            VertexAttributeValues::Tangent(v) => v.clear(),
            VertexAttributeValues::TexCoord0(v) => v.clear(),
            VertexAttributeValues::TexCoord1(v) => v.clear(),
            VertexAttributeValues::Color(v) => v.clear(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VertexAttributes {
    attribute: VertexAttribute,
    data: VertexAttributeValues,
}

impl VertexAttributes {
    pub fn new(data: VertexAttributeValues) -> Self {
        Self {
            attribute: data.kind(),
            data,
        }
    }

    pub fn attribute(&self) -> VertexAttribute {
        self.attribute
    }

    pub fn values(&self) -> &VertexAttributeValues {
        &self.data
    }

    pub fn values_mut(&mut self) -> &mut VertexAttributeValues {
        &mut self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn bytes(&self, range: Range<usize>) -> Vec<u8> {
        self.data.data(range)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexLayout {
    attributes: Vec<VertexAttribute>,
}

impl VertexLayout {
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
        }
    }

    pub fn add(&mut self, kind: VertexAttribute) {
        self.attributes.push(kind);
    }

    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    pub fn attributes(&self) -> &[VertexAttribute] {
        &self.attributes
    }

    pub fn contains(&self, attribute: VertexAttribute) -> bool {
        self.attributes.contains(&attribute)
    }

    pub fn location(&self, attribute: VertexAttribute) -> Option<usize> {
        self.attributes.iter().position(|a| *a == attribute)
    }

    pub fn size(&self) -> usize {
        self.attributes.iter().map(|a| a.size()).sum()
    }

    pub fn buffer_layout(&self) -> VertexBufferLayout {
        VertexBufferLayout::new(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VertexLayoutKey(u32);

impl VertexLayoutKey {
    pub fn new(attributes: &[VertexAttribute]) -> Self {
        let mut attributes = attributes.to_vec();
        attributes.sort();
        let mut hasher = crc32fast::Hasher::new();
        attributes.hash(&mut hasher);
        Self(hasher.finalize())
    }
}

pub struct VertexBufferLayout {
    /// The stride, in bytes, between elements of this buffer.
    pub array_stride: BufferAddress,
    /// How often this vertex buffer is "stepped" forward.
    pub step_mode: VertexStepMode,
    /// The list of attributes which comprise a single vertex.
    pub attributes: Vec<wgpu::VertexAttribute>,
}

impl VertexBufferLayout {
    pub fn new(layout: &VertexLayout) -> Self {
        let mut attributes = Vec::new();
        let mut offset = 0;
        for (location, kind) in layout.attributes().iter().enumerate() {
            let size = kind.size() as BufferAddress;
            attributes.push(wgpu::VertexAttribute {
                format: kind.format(),
                offset,
                shader_location: location as u32,
            });
            offset += size;
        }

        Self {
            array_stride: offset,
            step_mode: VertexStepMode::Vertex,
            attributes,
        }
    }

    pub fn wgpu(&self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: self.array_stride,
            step_mode: self.step_mode,
            attributes: &self.attributes,
        }
    }
}

impl From<Vec<VertexAttribute>> for VertexLayout {
    fn from(attributes: Vec<VertexAttribute>) -> Self {
        Self { attributes }
    }
}
