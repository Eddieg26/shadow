use super::buffer::{BufferFlags, IndexBuffer, Indices, VertexBuffer};
use crate::core::{VertexAttributeValues, VertexAttribute, VertexAttributes, VertexLayout};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ReadWrite {
    Enabled,
    Disabled,
}

pub struct Mesh {
    topology: MeshTopology,
    attributes: Vec<VertexAttributes>,
    indices: Option<Indices>,
    read_write: ReadWrite,
}

impl Mesh {
    pub fn new(topology: MeshTopology, read_write: ReadWrite) -> Self {
        Self {
            topology,
            attributes: Vec::new(),
            indices: None,
            read_write,
        }
    }

    pub fn topology(&self) -> MeshTopology {
        self.topology
    }

    pub fn attributes(&self) -> &[VertexAttributes] {
        &self.attributes
    }

    pub fn attribute(&self, kind: VertexAttribute) -> Option<&VertexAttributeValues> {
        self.attributes
            .iter()
            .find(|a| a.attribute() == kind)
            .map(|a| a.values())
    }

    pub fn attribute_mut(&mut self, kind: VertexAttribute) -> Option<&mut VertexAttributeValues> {
        self.attributes
            .iter_mut()
            .find(|a| a.attribute() == kind)
            .map(|a| a.values_mut())
    }

    pub fn indices(&self) -> Option<&Indices> {
        self.indices.as_ref()
    }

    pub fn indices_mut(&mut self) -> Option<&mut Indices> {
        self.indices.as_mut()
    }

    pub fn read_write(&self) -> ReadWrite {
        self.read_write
    }

    pub fn layout(&self) -> VertexLayout {
        let mut layout = VertexLayout::new();
        for attribute in &self.attributes {
            layout.add(attribute.attribute());
        }
        layout
    }

    pub fn insert_attribute(&mut self, attributes: VertexAttributeValues) -> Option<VertexAttributeValues> {
        let position = self
            .attributes
            .iter()
            .position(|a| a.attribute() == attributes.kind());

        match position {
            Some(position) => {
                let attribute = self.attributes.get_mut(position)?;
                Some(std::mem::replace(attribute.values_mut(), attributes))
            }
            None => {
                self.attributes.push(VertexAttributes::new(attributes));
                self.attributes.sort_by(|a, b| a.attribute().cmp(&b.attribute()));
                None
            }
        }
    }

    pub fn add_indices(&mut self, indices: Indices) {
        self.indices = Some(indices);
    }

    pub fn remove_attribute(&mut self, kind: VertexAttribute) -> Option<VertexAttributes> {
        let position = self.attributes.iter().position(|a| a.attribute() == kind)?;
        Some(self.attributes.remove(position))
    }

    pub fn clear(&mut self) {
        for attribute in &mut self.attributes {
            attribute.clear();
        }

        self.indices = None;
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

    pub fn vertex_buffer(&mut self) -> Option<VertexBuffer<u8>> {
        let data = self.vertex_data();
        let layout = self.layout();
        let flags = match self.read_write() {
            ReadWrite::Enabled => BufferFlags::COPY_DST | BufferFlags::MAP_WRITE,
            ReadWrite::Disabled => {
                self.attributes.clear();
                BufferFlags::empty()
            }
        };

        Some(VertexBuffer::create(&data, layout, flags))
    }

    pub fn index_buffer(&mut self) -> Option<IndexBuffer> {
        let (flags, indices) = match self.read_write() {
            ReadWrite::Enabled => (
                BufferFlags::COPY_DST | BufferFlags::MAP_WRITE,
                self.indices.clone()?,
            ),
            ReadWrite::Disabled => (BufferFlags::empty(), self.indices.take()?),
        };

        Some(IndexBuffer::create(indices, flags))
    }
}
