use super::{
    buffer::{BufferFlags, IndexBuffer, Indices, Vertex, VertexBuffer},
    ReadWrite, RenderAsset, RenderAssetExtractor, ResourceId,
};
use crate::core::{Color, RenderDevice, RenderQueue};
use asset::Asset;
use ecs::system::ArgItem;
use spatial::bounds::BoundingBox;
use std::{hash::Hash, ops::Range};

pub mod draw;
pub mod model;

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum MeshTopology {
    PointList = 0,
    LineList = 1,
    LineStrip = 2,
    #[default]
    TriangleList = 3,
    TriangleStrip = 4,
}

impl Into<wgpu::PrimitiveTopology> for MeshTopology {
    fn into(self) -> wgpu::PrimitiveTopology {
        match self {
            MeshTopology::PointList => wgpu::PrimitiveTopology::PointList,
            MeshTopology::LineList => wgpu::PrimitiveTopology::LineList,
            MeshTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
            MeshTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
            MeshTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
        }
    }
}

impl From<wgpu::PrimitiveTopology> for MeshTopology {
    fn from(topology: wgpu::PrimitiveTopology) -> Self {
        match topology {
            wgpu::PrimitiveTopology::PointList => MeshTopology::PointList,
            wgpu::PrimitiveTopology::LineList => MeshTopology::LineList,
            wgpu::PrimitiveTopology::LineStrip => MeshTopology::LineStrip,
            wgpu::PrimitiveTopology::TriangleList => MeshTopology::TriangleList,
            wgpu::PrimitiveTopology::TriangleStrip => MeshTopology::TriangleStrip,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MeshAttribute {
    Position(Vec<glam::Vec3>),
    Normal(Vec<glam::Vec3>),
    TexCoord0(Vec<glam::Vec2>),
    TexCoord1(Vec<glam::Vec2>),
    Tangent(Vec<glam::Vec4>),
    Color(Vec<Color>),
}

impl MeshAttribute {
    pub fn kind(&self) -> MeshAttributeKind {
        match self {
            MeshAttribute::Position(_) => MeshAttributeKind::Position,
            MeshAttribute::Normal(_) => MeshAttributeKind::Normal,
            MeshAttribute::TexCoord0(_) => MeshAttributeKind::TexCoord0,
            MeshAttribute::TexCoord1(_) => MeshAttributeKind::TexCoord1,
            MeshAttribute::Tangent(_) => MeshAttributeKind::Tangent,
            MeshAttribute::Color(_) => MeshAttributeKind::Color,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            MeshAttribute::Position(v) => v.len(),
            MeshAttribute::Normal(v) => v.len(),
            MeshAttribute::TexCoord0(v) => v.len(),
            MeshAttribute::TexCoord1(v) => v.len(),
            MeshAttribute::Tangent(v) => v.len(),
            MeshAttribute::Color(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            MeshAttribute::Position(v) => v.is_empty(),
            MeshAttribute::Normal(v) => v.is_empty(),
            MeshAttribute::TexCoord0(v) => v.is_empty(),
            MeshAttribute::TexCoord1(v) => v.is_empty(),
            MeshAttribute::Tangent(v) => v.is_empty(),
            MeshAttribute::Color(v) => v.is_empty(),
        }
    }

    pub fn data(&self, range: Range<usize>) -> &[u8] {
        match self {
            MeshAttribute::Position(v) => bytemuck::cast_slice(&v[range]),
            MeshAttribute::Normal(v) => bytemuck::cast_slice(&v[range]),
            MeshAttribute::TexCoord0(v) => bytemuck::cast_slice(&v[range]),
            MeshAttribute::TexCoord1(v) => bytemuck::cast_slice(&v[range]),
            MeshAttribute::Tangent(v) => bytemuck::cast_slice(&v[range]),
            MeshAttribute::Color(v) => bytemuck::cast_slice(&v[range]),
        }
    }

    pub fn clear(&mut self) {
        match self {
            MeshAttribute::Position(v) => v.clear(),
            MeshAttribute::Normal(v) => v.clear(),
            MeshAttribute::TexCoord0(v) => v.clear(),
            MeshAttribute::TexCoord1(v) => v.clear(),
            MeshAttribute::Tangent(v) => v.clear(),
            MeshAttribute::Color(v) => v.clear(),
        }
    }
}

impl Vertex for glam::Vec2 {}
impl Vertex for glam::Vec3 {}
impl Vertex for glam::Vec4 {}
impl Vertex for Color {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MeshAttributeKind {
    Position,
    Normal,
    TexCoord0,
    TexCoord1,
    Tangent,
    Color,
}

impl MeshAttributeKind {
    pub fn size(&self) -> usize {
        match self {
            MeshAttributeKind::Position => std::mem::size_of::<glam::Vec3>(),
            MeshAttributeKind::Normal => std::mem::size_of::<glam::Vec3>(),
            MeshAttributeKind::TexCoord0 => std::mem::size_of::<glam::Vec2>(),
            MeshAttributeKind::TexCoord1 => std::mem::size_of::<glam::Vec2>(),
            MeshAttributeKind::Tangent => std::mem::size_of::<glam::Vec4>(),
            MeshAttributeKind::Color => std::mem::size_of::<Color>(),
        }
    }

    pub fn format(&self) -> wgpu::VertexFormat {
        match self {
            MeshAttributeKind::Position => wgpu::VertexFormat::Float32x3,
            MeshAttributeKind::Normal => wgpu::VertexFormat::Float32x3,
            MeshAttributeKind::TexCoord0 => wgpu::VertexFormat::Float32x2,
            MeshAttributeKind::TexCoord1 => wgpu::VertexFormat::Float32x2,
            MeshAttributeKind::Tangent => wgpu::VertexFormat::Float32x4,
            MeshAttributeKind::Color => wgpu::VertexFormat::Float32x4,
        }
    }
}

impl Iterator for MeshAttributeKind {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MeshAttributeKind::Position => Some(MeshAttributeKind::Normal),
            MeshAttributeKind::Normal => Some(MeshAttributeKind::TexCoord0),
            MeshAttributeKind::TexCoord0 => Some(MeshAttributeKind::TexCoord1),
            MeshAttributeKind::TexCoord1 => Some(MeshAttributeKind::Tangent),
            MeshAttributeKind::Tangent => Some(MeshAttributeKind::Color),
            MeshAttributeKind::Color => None,
        }
    }
}

bitflags::bitflags! {
    #[derive(Default, Clone, Copy, PartialEq, Eq)]
    pub struct MeshDirty: u32 {
        const POSITION = 1 << 1;
        const NORMAL = 1 << 2;
        const TANGENT =  1 << 3;
        const TEXCOORD0 = 1 << 4;
        const TEXCOORD1 = 1 << 5;
        const COLOR = 1 << 6;
        const INDICES = 1 << 7;
        const BOUNDS = 1 << 8;
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mesh {
    topology: MeshTopology,
    attributes: Vec<MeshAttribute>,
    indices: Option<Indices>,
    bounds: BoundingBox,
    read_write: ReadWrite,

    #[serde(skip)]
    dirty: MeshDirty,
}

impl Mesh {
    pub fn new(topology: MeshTopology, read_write: ReadWrite) -> Self {
        Self {
            topology,
            attributes: Vec::new(),
            indices: None,
            bounds: BoundingBox::ZERO,
            read_write,
            dirty: MeshDirty::empty(),
        }
    }

    pub fn topology(&self) -> MeshTopology {
        self.topology
    }

    pub fn attributes(&self) -> &[MeshAttribute] {
        &self.attributes
    }

    pub fn attribute(&self, kind: MeshAttributeKind) -> Option<&MeshAttribute> {
        self.attribute_index(kind).map(|i| &self.attributes[i])
    }

    pub fn dirty(&self) -> MeshDirty {
        self.dirty
    }

    pub fn indices(&self) -> Option<&Indices> {
        self.indices.as_ref()
    }

    pub fn indices_mut(&mut self) -> Option<&mut Indices> {
        let indices = self.indices.as_mut();

        if indices.is_some() {
            self.dirty |= MeshDirty::INDICES;
        }

        indices
    }

    pub fn bounds(&self) -> BoundingBox {
        self.bounds
    }

    pub fn read_write(&self) -> ReadWrite {
        self.read_write
    }

    pub fn add_attribute(&mut self, attribute: MeshAttribute) {
        let kind = attribute.kind();
        match self.attribute_index(kind) {
            Some(i) => self.attributes[i] = attribute,
            None => self.attributes.push(attribute),
        }

        self.attribute_dirty(kind);
    }

    pub fn remove_attribute(&mut self, kind: MeshAttributeKind) -> Option<MeshAttribute> {
        let removed = self
            .attribute_index(kind)
            .map(|i| self.attributes.remove(i));

        self.attribute_dirty(kind);

        removed
    }

    pub fn set_indices(&mut self, indices: Indices) {
        self.indices = Some(indices);
        self.dirty |= MeshDirty::INDICES;
    }

    pub fn attribute_index(&self, kind: MeshAttributeKind) -> Option<usize> {
        self.attributes.iter().position(|a| a.kind() == kind)
    }

    pub fn clear(&mut self) {
        for attribute in &mut self.attributes {
            attribute.clear();
        }

        self.indices = None;
        self.dirty = MeshDirty::all()
    }

    pub fn vertex_count(&self) -> usize {
        self.attributes
            .iter()
            .fold(0, |len, curr| len.min(curr.len()))
    }

    pub fn calculate_bounds(&mut self) {
        let bounds_dirty = self.dirty.contains(MeshDirty::BOUNDS);

        match (bounds_dirty, self.attribute(MeshAttributeKind::Position)) {
            (true, Some(MeshAttribute::Position(positions))) => {
                self.bounds = BoundingBox::from(positions.as_slice());
                self.dirty.remove(MeshDirty::BOUNDS);
            }
            _ => (),
        }
    }

    pub fn attribute_data(&self, kind: MeshAttributeKind, range: Range<usize>) -> &[u8] {
        self.attribute(kind).map_or(&[], |a| a.data(range))
    }

    pub fn attribute_dirty(&mut self, attribute: MeshAttributeKind) {
        match attribute {
            MeshAttributeKind::Position => self.dirty |= MeshDirty::POSITION | MeshDirty::BOUNDS,
            MeshAttributeKind::Normal => self.dirty |= MeshDirty::NORMAL,
            MeshAttributeKind::Tangent => self.dirty |= MeshDirty::TANGENT,
            MeshAttributeKind::TexCoord0 => self.dirty |= MeshDirty::TEXCOORD0,
            MeshAttributeKind::TexCoord1 => self.dirty |= MeshDirty::TEXCOORD1,
            MeshAttributeKind::Color => self.dirty |= MeshDirty::COLOR,
        }
    }

    pub fn is_attribute_dirty(&self, attribute: MeshAttributeKind) -> bool {
        match attribute {
            MeshAttributeKind::Position => self.dirty.contains(MeshDirty::POSITION),
            MeshAttributeKind::Normal => self.dirty.contains(MeshDirty::NORMAL),
            MeshAttributeKind::Tangent => self.dirty.contains(MeshDirty::TANGENT),
            MeshAttributeKind::TexCoord0 => self.dirty.contains(MeshDirty::TEXCOORD0),
            MeshAttributeKind::TexCoord1 => self.dirty.contains(MeshDirty::TEXCOORD1),
            MeshAttributeKind::Color => self.dirty.contains(MeshDirty::COLOR),
        }
    }

    pub fn buffers(&mut self, device: &RenderDevice) -> MeshBuffers {
        let mut vertex_buffers = vec![];
        let mut attributes = vec![];
        let count = self.vertex_count();

        let flags = match self.read_write {
            ReadWrite::Enabled => BufferFlags::COPY_DST | BufferFlags::MAP_WRITE,
            ReadWrite::Disabled => BufferFlags::empty(),
        };

        for attribute in self.attributes() {
            let mut buffer = MeshBuffers::create_vertex_buffer(attribute, count, flags);
            buffer.create(device);
            vertex_buffers.push(buffer);
            attributes.push(attribute.kind());
        }

        let indices = match self.read_write {
            ReadWrite::Enabled => self.indices.clone(),
            ReadWrite::Disabled => {
                self.attributes.clear();
                self.indices.take()
            }
        };

        let index_buffer = match indices {
            Some(indices) => {
                let mut buffer = IndexBuffer::new(indices, flags);
                buffer.create(device);
                Some(Box::new(buffer))
            }
            None => None,
        };

        MeshBuffers {
            layout: attributes.into(),
            vertex_buffers: vertex_buffers.into_boxed_slice(),
            index_buffer,
        }
    }

    pub fn update(
        &mut self,
        buffers: &mut MeshBuffers,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) {
        let len = self.vertex_count();
        for values in self.attributes.iter() {
            if self.is_attribute_dirty(values.kind()) {
                match buffers.vertex_buffer_mut(values.kind()) {
                    Some(buffer) => {
                        match values {
                            MeshAttribute::Position(v) => buffer.set(&v[..len]),
                            MeshAttribute::Normal(v) => buffer.set(&v[..len]),
                            MeshAttribute::TexCoord0(v) => buffer.set(&v[..len]),
                            MeshAttribute::TexCoord1(v) => buffer.set(&v[..len]),
                            MeshAttribute::Tangent(v) => buffer.set(&v[..len]),
                            MeshAttribute::Color(v) => buffer.set(&v[..len]),
                        }
                        buffer.commit(device, queue);
                        self.dirty.remove(MeshDirty::POSITION);
                    }
                    _ => (),
                }
            }
        }

        if self.dirty.contains(MeshDirty::INDICES) {
            match (buffers.index_buffer_mut(), self.indices()) {
                (Some(index), Some(indices)) => {
                    index.set(indices.clone());
                    index.commit(device, queue);
                    self.dirty.remove(MeshDirty::INDICES);
                }
                _ => (),
            }
        }
    }
}

impl Asset for Mesh {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshLayout(Box<[MeshAttributeKind]>);

impl From<Vec<MeshAttributeKind>> for MeshLayout {
    fn from(attributes: Vec<MeshAttributeKind>) -> Self {
        Self(attributes.into_boxed_slice())
    }
}

impl From<&[MeshAttributeKind]> for MeshLayout {
    fn from(attributes: &[MeshAttributeKind]) -> Self {
        Self(attributes.to_vec().into_boxed_slice())
    }
}

impl std::ops::Deref for MeshLayout {
    type Target = [MeshAttributeKind];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct MeshBuffers {
    layout: MeshLayout,
    vertex_buffers: Box<[VertexBuffer]>,
    index_buffer: Option<Box<IndexBuffer>>,
}

impl MeshBuffers {
    pub fn layout(&self) -> &MeshLayout {
        &self.layout
    }

    pub fn has_attribute(&self, kind: MeshAttributeKind) -> bool {
        self.layout.contains(&kind)
    }

    pub fn attribute_index(&self, kind: MeshAttributeKind) -> Option<usize> {
        self.layout.iter().position(|k| *k == kind)
    }

    pub fn vertex_buffers(&self) -> &[VertexBuffer] {
        &self.vertex_buffers
    }

    pub fn vertex_buffer(&self, kind: MeshAttributeKind) -> Option<&VertexBuffer> {
        self.attribute_index(kind).map(|i| &self.vertex_buffers[i])
    }

    pub fn vertex_buffer_mut(&mut self, kind: MeshAttributeKind) -> Option<&mut VertexBuffer> {
        self.attribute_index(kind)
            .map(move |i| &mut self.vertex_buffers[i])
    }

    pub fn index_buffer(&self) -> Option<&IndexBuffer> {
        self.index_buffer.as_ref().map(|b| b.as_ref())
    }

    pub fn index_buffer_mut(&mut self) -> Option<&mut IndexBuffer> {
        self.index_buffer.as_mut().map(|b| b.as_mut())
    }

    fn create_vertex_buffer(
        attribute: &MeshAttribute,
        count: usize,
        flags: BufferFlags,
    ) -> VertexBuffer {
        match attribute {
            MeshAttribute::Position(v) => VertexBuffer::new(&v[..count], flags),
            MeshAttribute::Normal(v) => VertexBuffer::new(&v[..count], flags),
            MeshAttribute::TexCoord0(v) => VertexBuffer::new(&v[..count], flags),
            MeshAttribute::TexCoord1(v) => VertexBuffer::new(&v[..count], flags),
            MeshAttribute::Tangent(v) => VertexBuffer::new(&v[..count], flags),
            MeshAttribute::Color(v) => VertexBuffer::new(&v[..count], flags),
        }
    }
}

impl RenderAsset for MeshBuffers {
    type Id = ResourceId;
}

impl RenderAssetExtractor for MeshBuffers {
    type Source = Mesh;
    type Target = MeshBuffers;
    type Arg<'a> = (&'a RenderDevice, &'a RenderQueue);

    fn extract<'a>(
        source: &mut Self::Source,
        arg: &ArgItem<Self::Arg<'a>>,
    ) -> Option<Self::Target> {
        let (device, _) = arg;

        let buffers = source.buffers(device);

        Some(buffers)
    }

    fn update<'a>(
        source: &mut Self::Source,
        asset: &mut Self::Target,
        arg: &ArgItem<Self::Arg<'a>>,
    ) {
        if source.read_write() == ReadWrite::Enabled {
            let (device, queue) = arg;
            source.update(asset, device, queue);
        }
    }
}
