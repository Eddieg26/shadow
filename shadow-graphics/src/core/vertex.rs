use super::Color;
use glam::{Vec2, Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VertexPositionKind {
    Vec2,
    Vec3,
}

impl VertexPositionKind {
    pub fn size(&self) -> usize {
        match self {
            VertexPositionKind::Vec2 => std::mem::size_of::<Vec2>(),
            VertexPositionKind::Vec3 => std::mem::size_of::<Vec3>(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VertexAttributeKind {
    Position(VertexPositionKind),
    Normal,
    Tangent,
    TexCoord0,
    TexCoord1,
    Color,
}

impl VertexAttributeKind {
    pub fn size(&self) -> usize {
        match self {
            VertexAttributeKind::Position(kind) => kind.size(),
            VertexAttributeKind::Normal => std::mem::size_of::<Vec3>(),
            VertexAttributeKind::Tangent => std::mem::size_of::<Vec4>(),
            VertexAttributeKind::TexCoord0 => std::mem::size_of::<Vec2>(),
            VertexAttributeKind::TexCoord1 => std::mem::size_of::<Vec2>(),
            VertexAttributeKind::Color => std::mem::size_of::<Color>(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VertexPositions {
    Vec2(Vec<Vec2>),
    Vec3(Vec<Vec3>),
}

impl VertexPositions {
    pub fn kind(&self) -> VertexAttributeKind {
        match self {
            VertexPositions::Vec2(_) => VertexAttributeKind::Position(VertexPositionKind::Vec2),
            VertexPositions::Vec3(_) => VertexAttributeKind::Position(VertexPositionKind::Vec3),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            VertexPositions::Vec2(v) => v.len(),
            VertexPositions::Vec3(v) => v.len(),
        }
    }

    pub fn data(&self, index: usize) -> Vec<u8> {
        match self {
            VertexPositions::Vec2(v) => bytemuck::bytes_of(&v[index]).to_vec(),
            VertexPositions::Vec3(v) => bytemuck::bytes_of(&v[index]).to_vec(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VertexAttribute {
    Position(VertexPositions),
    Normal(Vec<Vec3>),
    Tangent(Vec<Vec4>),
    TexCoord0(Vec<Vec2>),
    TexCoord1(Vec<Vec2>),
    Color(Vec<Color>),
}

impl VertexAttribute {
    pub fn kind(&self) -> VertexAttributeKind {
        match self {
            VertexAttribute::Position(pos) => match pos {
                VertexPositions::Vec2(_) => VertexAttributeKind::Position(VertexPositionKind::Vec2),
                VertexPositions::Vec3(_) => VertexAttributeKind::Position(VertexPositionKind::Vec3),
            },
            VertexAttribute::Normal(_) => VertexAttributeKind::Normal,
            VertexAttribute::Tangent(_) => VertexAttributeKind::Tangent,
            VertexAttribute::TexCoord0(_) => VertexAttributeKind::TexCoord0,
            VertexAttribute::TexCoord1(_) => VertexAttributeKind::TexCoord1,
            VertexAttribute::Color(_) => VertexAttributeKind::Color,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            VertexAttribute::Position(pos) => pos.len(),
            VertexAttribute::Normal(v) => v.len(),
            VertexAttribute::Tangent(v) => v.len(),
            VertexAttribute::TexCoord0(v) => v.len(),
            VertexAttribute::TexCoord1(v) => v.len(),
            VertexAttribute::Color(v) => v.len(),
        }
    }

    pub fn data(&self, index: usize) -> Vec<u8> {
        match self {
            VertexAttribute::Position(pos) => pos.data(index),
            VertexAttribute::Normal(v) => bytemuck::bytes_of(&v[index]).to_vec(),
            VertexAttribute::Tangent(v) => bytemuck::bytes_of(&v[index]).to_vec(),
            VertexAttribute::TexCoord0(v) => bytemuck::bytes_of(&v[index]).to_vec(),
            VertexAttribute::TexCoord1(v) => bytemuck::bytes_of(&v[index]).to_vec(),
            VertexAttribute::Color(v) => bytemuck::bytes_of(&v[index]).to_vec(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            VertexAttribute::Position(pos) => match pos {
                VertexPositions::Vec2(v) => v.clear(),
                VertexPositions::Vec3(v) => v.clear(),
            },
            VertexAttribute::Normal(v) => v.clear(),
            VertexAttribute::Tangent(v) => v.clear(),
            VertexAttribute::TexCoord0(v) => v.clear(),
            VertexAttribute::TexCoord1(v) => v.clear(),
            VertexAttribute::Color(v) => v.clear(),
        }
    }
}

pub struct VertexAttributes {
    kind: VertexAttributeKind,
    data: VertexAttribute,
}

impl VertexAttributes {
    pub fn new(data: VertexAttribute) -> Self {
        Self {
            kind: data.kind(),
            data,
        }
    }

    pub fn kind(&self) -> VertexAttributeKind {
        self.kind
    }

    pub fn data(&self) -> &VertexAttribute {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut VertexAttribute {
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
    attributes: Vec<VertexAttributeKind>,
}

impl VertexLayout {
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
        }
    }

    pub fn add(&mut self, kind: VertexAttributeKind) {
        self.attributes.push(kind);
    }

    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    pub fn attributes(&self) -> &[VertexAttributeKind] {
        &self.attributes
    }

    pub fn size(&self) -> usize {
        self.attributes.iter().map(|a| a.size()).sum()
    }
}

impl From<Vec<VertexAttributeKind>> for VertexLayout {
    fn from(attributes: Vec<VertexAttributeKind>) -> Self {
        Self { attributes }
    }
}
