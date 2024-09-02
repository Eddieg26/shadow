use super::{
    buffer::{BufferFlags, IndexBuffer, Indices, VertexBuffer},
    ReadWrite, RenderAsset, RenderAssetExtractor,
};
use crate::core::{
    RenderDevice, RenderQueue, VertexAttribute, VertexAttributeValues, VertexAttributes,
    VertexLayout,
};
use asset::Asset;
use ecs::system::ArgItem;
use spatial::bounds::BoundingBox;
use std::hash::Hash;

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
    pub struct MeshDirty: u8 {
        const NONE = 0;
        const ATTRIBUTES = 1;
        const INDICES = 2;
        const BOUNDS = 4;
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
            dirty: MeshDirty::NONE,
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
        let attributes = self
            .attributes
            .iter_mut()
            .find(|a| a.attribute() == kind)
            .map(|a| a.values_mut());

        if attributes.is_some() {
            self.dirty |= MeshDirty::ATTRIBUTES;
        }

        if kind == VertexAttribute::Position {
            self.dirty |= MeshDirty::BOUNDS;
        }

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

        self.dirty |= MeshDirty::ATTRIBUTES;

        if attributes.kind() == VertexAttribute::Position {
            self.dirty |= MeshDirty::BOUNDS;
        }

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

        if kind == VertexAttribute::Position {
            self.dirty |= MeshDirty::BOUNDS;
        }

        Some(self.attributes.remove(position))
    }

    pub fn clear(&mut self) {
        for attribute in &mut self.attributes {
            attribute.clear();
        }

        self.indices = None;
        self.dirty = MeshDirty::ATTRIBUTES | MeshDirty::INDICES | MeshDirty::BOUNDS;
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

    pub fn vertex_data(&self) -> Vec<u8> {
        let vertex_size = self.vertex_size();
        let vertex_count = self.vertex_count();

        let mut data = vec![0; vertex_size * vertex_count];

        for index in 0..vertex_count {
            for attribute in &self.attributes {
                data.extend(attribute.bytes(index));
            }
        }

        data
    }

    pub fn vertex_buffer(&self) -> Option<VertexBuffer<u8>> {
        let data = self.vertex_data();
        let layout = self.layout();
        let flags = match self.read_write() {
            ReadWrite::Enabled => BufferFlags::COPY_DST | BufferFlags::MAP_WRITE,
            ReadWrite::Disabled => BufferFlags::empty(),
        };

        match data.is_empty() {
            true => None,
            false => Some(VertexBuffer::create(data, layout, flags)),
        }
    }

    pub fn index_buffer(&self) -> Option<IndexBuffer> {
        let flags = match self.read_write() {
            ReadWrite::Enabled => BufferFlags::COPY_DST | BufferFlags::MAP_WRITE,
            ReadWrite::Disabled => BufferFlags::empty(),
        };

        let indices = self.indices.as_ref()?;
        match indices.is_empty() {
            true => None,
            false => Some(IndexBuffer::create(indices.clone(), flags)),
        }
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

    pub fn upload(
        &mut self,
        buffers: &mut MeshBuffers,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) {
        if self.dirty.contains(MeshDirty::ATTRIBUTES) {
            buffers.vertex_mut().set_vertices(self.vertex_data());
            buffers.vertex_mut().commit(device, queue);
            self.dirty.remove(MeshDirty::ATTRIBUTES);
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
    vertex: VertexBuffer<u8>,
    index: Option<IndexBuffer>,
}

impl MeshBuffers {
    pub fn create(mesh: &Mesh) -> Option<MeshBuffers> {
        let vertex = mesh.vertex_buffer()?;
        let index = mesh.index_buffer();

        Some(Self { vertex, index })
    }

    pub fn vertex(&self) -> &VertexBuffer<u8> {
        &self.vertex
    }

    pub fn index(&self) -> Option<&IndexBuffer> {
        self.index.as_ref()
    }

    pub fn vertex_mut(&mut self) -> &mut VertexBuffer<u8> {
        &mut self.vertex
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

        let mut buffers = Self::create(source)?;
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
