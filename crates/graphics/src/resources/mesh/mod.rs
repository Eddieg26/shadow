use super::{
    buffer::{BufferFlags, IndexBuffer, Indices, VertexBuffer},
    ReadWrite, RenderAsset, RenderAssetExtractor,
};
use crate::core::{
    RenderDevice, RenderQueue, VertexAttribute, VertexAttributeValues, VertexAttributes,
    VertexLayout,
};
use asset::Asset;
use ecs::{core::DenseMap, system::ArgItem};
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
    attributes: Vec<VertexAttributes>,
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

    pub fn attributes(&self) -> &[VertexAttributes] {
        &self.attributes
    }

    pub fn dirty(&self) -> MeshDirty {
        self.dirty
    }

    pub fn attribute(&self, kind: VertexAttribute) -> Option<&VertexAttributeValues> {
        self.attributes
            .iter()
            .find(|a| a.attribute() == kind)
            .map(|a| a.values())
    }

    pub fn attribute_mut(&mut self, kind: VertexAttribute) -> Option<&mut VertexAttributeValues> {
        self.attribute_dirty(kind);

        let attributes = self
            .attributes
            .iter_mut()
            .find(|a| a.attribute() == kind)
            .map(|a| a.values_mut());

        attributes
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

    pub fn layout(&self) -> VertexLayout {
        let attributes = self
            .attributes
            .iter()
            .map(|a| a.attribute())
            .collect::<Vec<VertexAttribute>>();

        VertexLayout::from(attributes)
    }

    pub fn add_attribute(
        &mut self,
        attributes: VertexAttributeValues,
    ) -> Option<VertexAttributeValues> {
        let position = self
            .attributes
            .iter()
            .position(|a| a.attribute() == attributes.kind());

        self.attribute_dirty(attributes.kind());

        match position {
            Some(position) => {
                let attribute = self.attributes.get_mut(position)?;
                Some(std::mem::replace(attribute.values_mut(), attributes))
            }
            None => {
                self.attributes.push(VertexAttributes::new(attributes));
                self.attributes
                    .sort_by(|a, b| a.attribute().cmp(&b.attribute()));
                None
            }
        }
    }

    pub fn add_indices(&mut self, indices: Indices) {
        self.indices = Some(indices);
        self.dirty |= MeshDirty::INDICES;
    }

    pub fn remove_attribute(&mut self, kind: VertexAttribute) -> Option<VertexAttributes> {
        let position = self.attributes.iter().position(|a| a.attribute() == kind)?;

        self.attribute_dirty(kind);

        Some(self.attributes.remove(position))
    }

    pub fn clear(&mut self) {
        for attribute in &mut self.attributes {
            attribute.clear();
        }

        self.indices = None;
        self.dirty = MeshDirty::all()
    }

    pub fn vertex_count(&self) -> usize {
        let mut count = 0usize;
        for attribute in &self.attributes {
            count = count.min(attribute.len());
        }
        count
    }

    pub fn vertex_size(&self) -> usize {
        self.attributes.iter().map(|a| a.attribute().size()).sum()
    }

    pub fn attribute_data(&self, attribute: VertexAttribute, range: Range<usize>) -> Vec<u8> {
        let attribute = match self.attribute(attribute) {
            Some(attribute) => attribute,
            None => return Vec::new(),
        };

        attribute.data(range)
    }

    pub fn buffers(&self) -> Option<MeshBuffers> {
        if self.attributes.is_empty() {
            return None;
        }

        let mut vertex_buffers = DenseMap::new();
        let flags = match self.read_write() {
            ReadWrite::Enabled => BufferFlags::COPY_DST | BufferFlags::MAP_WRITE,
            ReadWrite::Disabled => BufferFlags::empty(),
        };

        let len = self.len();
        for attribute in &self.attributes {
            let data = self.attribute_data(attribute.attribute(), 0..len);
            if data.is_empty() {
                continue;
            }

            let layout = VertexLayout::from(vec![attribute.attribute()]);
            let buffer = VertexBuffer::create(data, layout, flags);
            vertex_buffers.insert(attribute.attribute(), buffer);
        }

        let index = self.indices.as_ref().map(|indices| {
            let buffer = IndexBuffer::create(indices.clone(), flags);
            buffer
        });

        match vertex_buffers.is_empty() {
            true => return None,
            false => Some(MeshBuffers {
                layout: self.layout().clone(),
                vertex_buffers,
                index,
            }),
        }
    }

    pub fn len(&self) -> usize {
        let mut size = usize::MAX;
        for attribute in &self.attributes {
            size = size.min(attribute.len());
        }

        size
    }

    pub fn calculate_bounds(&mut self) {
        let bounds_dirty = self.dirty.contains(MeshDirty::BOUNDS);

        match (bounds_dirty, self.attribute(VertexAttribute::Position)) {
            (true, Some(VertexAttributeValues::Position(positions))) => {
                self.bounds = BoundingBox::from(positions.as_slice());
                self.dirty.remove(MeshDirty::BOUNDS);
            }
            _ => (),
        }
    }

    pub fn attribute_dirty(&mut self, attribute: VertexAttribute) {
        match attribute {
            VertexAttribute::Position => self.dirty |= MeshDirty::POSITION | MeshDirty::BOUNDS,
            VertexAttribute::Normal => self.dirty |= MeshDirty::NORMAL,
            VertexAttribute::Tangent => self.dirty |= MeshDirty::TANGENT,
            VertexAttribute::TexCoord0 => self.dirty |= MeshDirty::TEXCOORD0,
            VertexAttribute::TexCoord1 => self.dirty |= MeshDirty::TEXCOORD1,
            VertexAttribute::Color => self.dirty |= MeshDirty::COLOR,
        }
    }

    pub fn is_attribute_dirty(&self, attribute: VertexAttribute) -> bool {
        match attribute {
            VertexAttribute::Position => self.dirty.contains(MeshDirty::POSITION),
            VertexAttribute::Normal => self.dirty.contains(MeshDirty::NORMAL),
            VertexAttribute::Tangent => self.dirty.contains(MeshDirty::TANGENT),
            VertexAttribute::TexCoord0 => self.dirty.contains(MeshDirty::TEXCOORD0),
            VertexAttribute::TexCoord1 => self.dirty.contains(MeshDirty::TEXCOORD1),
            VertexAttribute::Color => self.dirty.contains(MeshDirty::COLOR),
        }
    }

    pub fn upload(
        &mut self,
        buffers: &mut MeshBuffers,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) {
        let len = self.len();
        for values in self.attributes.iter() {
            if self.is_attribute_dirty(values.attribute()) {
                match buffers.vertex_buffer_mut(values.attribute()) {
                    Some(buffer) => {
                        let data = self.attribute_data(values.attribute(), 0..len);
                        buffer.set_vertices(data);
                        buffer.commit(device, queue);
                        self.dirty.remove(MeshDirty::POSITION);
                    }
                    _ => (),
                }
            }
        }

        if self.dirty.contains(MeshDirty::INDICES) {
            match (buffers.index_mut(), self.indices()) {
                (Some(index), Some(indices)) => {
                    index.set_indices(indices.clone());
                    index.commit(device, queue);
                    self.dirty.remove(MeshDirty::INDICES);
                }
                _ => (),
            }
        }
    }
}

impl Asset for Mesh {}

pub struct MeshBuffers {
    layout: VertexLayout,
    vertex_buffers: DenseMap<VertexAttribute, VertexBuffer<u8>>,
    index: Option<IndexBuffer>,
}

impl MeshBuffers {
    pub fn layout(&self) -> &VertexLayout {
        &self.layout
    }

    pub fn vertex_buffer(&self, attribute: VertexAttribute) -> Option<&VertexBuffer<u8>> {
        self.vertex_buffers.get(&attribute)
    }

    pub fn index(&self) -> Option<&IndexBuffer> {
        self.index.as_ref()
    }

    pub fn vertex_buffer_mut(
        &mut self,
        attribute: VertexAttribute,
    ) -> Option<&mut VertexBuffer<u8>> {
        self.vertex_buffers.get_mut(&attribute)
    }

    pub fn attributes(&self) -> &[VertexAttribute] {
        self.vertex_buffers.keys()
    }

    pub fn index_mut(&mut self) -> Option<&mut IndexBuffer> {
        self.index.as_mut()
    }
}

impl RenderAsset for MeshBuffers {}

impl RenderAssetExtractor for MeshBuffers {
    type Source = Mesh;
    type Target = MeshBuffers;
    type Arg<'a> = (&'a RenderDevice, &'a RenderQueue);

    fn extract<'a>(
        source: &mut Self::Source,
        arg: &ArgItem<Self::Arg<'a>>,
    ) -> Option<Self::Target> {
        let (device, queue) = arg;

        let mut buffers = source.buffers()?;
        source.upload(&mut buffers, device, queue);

        Some(buffers)
    }

    fn update<'a>(
        source: &mut Self::Source,
        asset: &mut Self::Target,
        arg: &ArgItem<Self::Arg<'a>>,
    ) {
        if source.read_write() == ReadWrite::Enabled {
            let (device, queue) = arg;
            source.upload(asset, device, queue);
        }
    }
}
