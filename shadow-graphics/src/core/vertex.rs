use super::Color;
use glam::{Vec2, Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VertexAttribute {
    Position,
    Normal,
    Tangent,
    TexCoord0,
    TexCoord1,
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

    pub fn location(&self) -> u32 {
        match self {
            VertexAttribute::Position => 0,
            VertexAttribute::Normal => 1,
            VertexAttribute::TexCoord0 => 2,
            VertexAttribute::TexCoord1 => 3,
            VertexAttribute::Color => 4,
            VertexAttribute::Tangent => 5,
        }
    }
}

#[derive(Debug, Clone)]
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

    pub fn data(&self, index: usize) -> Vec<u8> {
        match self {
            VertexAttributeValues::Position(v) => bytemuck::bytes_of(&v[index]).to_vec(),
            VertexAttributeValues::Normal(v) => bytemuck::bytes_of(&v[index]).to_vec(),
            VertexAttributeValues::Tangent(v) => bytemuck::bytes_of(&v[index]).to_vec(),
            VertexAttributeValues::TexCoord0(v) => bytemuck::bytes_of(&v[index]).to_vec(),
            VertexAttributeValues::TexCoord1(v) => bytemuck::bytes_of(&v[index]).to_vec(),
            VertexAttributeValues::Color(v) => bytemuck::bytes_of(&v[index]).to_vec(),
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

    pub fn bytes(&self, index: usize) -> Vec<u8> {
        self.data.data(index)
    }
}

#[derive(Debug, Clone)]
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

    pub fn size(&self) -> usize {
        self.attributes.iter().map(|a| a.size()).sum()
    }
}

impl From<Vec<VertexAttribute>> for VertexLayout {
    fn from(attributes: Vec<VertexAttribute>) -> Self {
        Self { attributes }
    }
}
