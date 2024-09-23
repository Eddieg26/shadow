use super::{
    buffer::{BufferFlags, IndexBuffer, Indices, Vertex, VertexBuffer},
    AssetUsage, ReadWrite, RenderAsset, RenderAssetExtractor, RenderAssets,
};
use crate::core::{Color, RenderDevice, RenderQueue};
use asset::{Asset, AssetId};
use ecs::system::{unlifetime::Read, ArgItem, StaticSystemArg};
use spatial::bounds::BoundingBox;
use std::{hash::Hash, ops::Range};

pub mod loaders;

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

    pub fn extend(&mut self, other: &Self) {
        match (self, other) {
            (MeshAttribute::Position(a), MeshAttribute::Position(b)) => a.extend_from_slice(b),
            (MeshAttribute::Normal(a), MeshAttribute::Normal(b)) => a.extend_from_slice(b),
            (MeshAttribute::TexCoord0(a), MeshAttribute::TexCoord0(b)) => a.extend_from_slice(b),
            (MeshAttribute::TexCoord1(a), MeshAttribute::TexCoord1(b)) => a.extend_from_slice(b),
            (MeshAttribute::Tangent(a), MeshAttribute::Tangent(b)) => a.extend_from_slice(b),
            (MeshAttribute::Color(a), MeshAttribute::Color(b)) => a.extend_from_slice(b),
            _ => (),
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

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SubMesh {
    pub start_vertex: u32,
    pub vertex_count: u32,
    pub start_index: u32,
    pub index_count: u32,
}

impl SubMesh {
    pub fn new(start_vertex: u32, vertex_count: u32, start_index: u32, index_count: u32) -> Self {
        Self {
            start_vertex,
            vertex_count,
            start_index,
            index_count,
        }
    }
}

impl Asset for SubMesh {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mesh {
    topology: MeshTopology,
    attributes: Vec<MeshAttribute>,
    indices: Option<Indices>,
    bounds: BoundingBox,
    read_write: ReadWrite,
    sub_meshes: Vec<SubMesh>,

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
            sub_meshes: Vec::new(),
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

    pub fn attribute_mut(&mut self, kind: MeshAttributeKind) -> Option<&mut MeshAttribute> {
        match self.attribute_index(kind) {
            Some(i) => {
                self.attribute_dirty(kind);
                Some(&mut self.attributes[i])
            }
            None => None,
        }
    }

    pub fn sub_meshes(&self) -> &[SubMesh] {
        &self.sub_meshes
    }

    pub fn sub_mesh(&self, index: usize) -> Option<&SubMesh> {
        self.sub_meshes.get(index)
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

    pub fn add_indices(&mut self, indices: Indices) {
        match self.indices {
            Some(ref mut i) => i.extend(indices),
            None => self.indices = Some(indices),
        }
    }

    pub fn attribute_index(&self, kind: MeshAttributeKind) -> Option<usize> {
        self.attributes.iter().position(|a| a.kind() == kind)
    }

    pub fn add_sub_mesh(&mut self, sub_mesh: SubMesh) {
        self.sub_meshes.push(sub_mesh);
    }

    pub fn remove_sub_mesh(&mut self, index: usize) -> SubMesh {
        self.sub_meshes.remove(index)
    }

    pub fn clear(&mut self) {
        for attribute in &mut self.attributes {
            attribute.clear();
        }

        self.indices = None;
        self.dirty = MeshDirty::all()
    }

    pub fn vertex_count(&self) -> usize {
        if self.attributes.is_empty() {
            return 0;
        }

        self.attributes
            .iter()
            .fold(usize::MAX, |len, curr| len.min(curr.len()))
    }

    pub fn index_count(&self) -> usize {
        self.indices.as_ref().map_or(0, |i| i.len())
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
            vertex_count: count,
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

impl<A: AsRef<[MeshAttributeKind]>> From<&A> for MeshLayout {
    fn from(attributes: &A) -> Self {
        Self(attributes.as_ref().to_vec().into_boxed_slice())
    }
}

impl std::ops::Deref for MeshLayout {
    type Target = [MeshAttributeKind];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for MeshLayout {
    type Item = MeshAttributeKind;
    type IntoIter = std::vec::IntoIter<MeshAttributeKind>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a MeshLayout {
    type Item = &'a MeshAttributeKind;
    type IntoIter = std::slice::Iter<'a, MeshAttributeKind>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

pub struct MeshBuffers {
    layout: MeshLayout,
    vertex_buffers: Box<[VertexBuffer]>,
    index_buffer: Option<Box<IndexBuffer>>,
    vertex_count: usize,
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

    pub fn get_vertex_buffer(&self, index: usize) -> Option<&VertexBuffer> {
        self.vertex_buffers.get(index)
    }

    pub fn vertex_buffer(&self, kind: MeshAttributeKind) -> Option<&VertexBuffer> {
        self.attribute_index(kind).map(|i| &self.vertex_buffers[i])
    }

    pub fn vertex_buffer_mut(&mut self, kind: MeshAttributeKind) -> Option<&mut VertexBuffer> {
        self.attribute_index(kind)
            .map(move |i| &mut self.vertex_buffers[i])
    }

    pub fn vertex_count(&self) -> usize {
        self.vertex_count
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
        let buffer = match attribute {
            MeshAttribute::Position(v) => VertexBuffer::new(&v[..count], flags),
            MeshAttribute::Normal(v) => VertexBuffer::new(&v[..count], flags),
            MeshAttribute::TexCoord0(v) => VertexBuffer::new(&v[..count], flags),
            MeshAttribute::TexCoord1(v) => VertexBuffer::new(&v[..count], flags),
            MeshAttribute::Tangent(v) => VertexBuffer::new(&v[..count], flags),
            MeshAttribute::Color(v) => VertexBuffer::new(&v[..count], flags),
        };

        buffer.with_label("Mesh Vertex Buffer")
    }
}

impl RenderAsset for MeshBuffers {
    type Id = AssetId;
}

impl RenderAssetExtractor for Mesh {
    type Source = Mesh;
    type Target = MeshBuffers;
    type Arg = StaticSystemArg<'static, (Read<RenderDevice>, Read<RenderQueue>)>;

    fn extract(
        id: &AssetId,
        source: &mut Self::Source,
        arg: &mut ArgItem<Self::Arg>,
        assets: &mut RenderAssets<Self::Target>,
    ) -> Option<AssetUsage> {
        let (device, queue) = **arg;

        match assets.get_mut(id) {
            Some(buffers) => match source.read_write {
                ReadWrite::Enabled => source.update(buffers, device, queue),
                ReadWrite::Disabled => (),
            },
            None => {
                assets.add(*id, source.buffers(device));
            }
        }

        match source.read_write {
            ReadWrite::Enabled => Some(AssetUsage::Keep),
            ReadWrite::Disabled => Some(AssetUsage::Discard),
        }
    }

    fn remove(id: &AssetId, assets: &mut RenderAssets<Self::Target>, _: &mut ArgItem<Self::Arg>) {
        assets.remove(id);
    }
}
