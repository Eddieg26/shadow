use super::{
    context::RenderContext,
    resources::{BufferDesc, TextureDesc},
};
use crate::resources::ResourceId;
use std::collections::{HashMap, HashSet};

pub trait RenderGraphNode: downcast_rs::Downcast + Send + Sync + 'static {
    fn name(&self) -> &str;
    fn info(&self) -> NodeInfo;
    fn execute(&mut self, ctx: &RenderContext);
}
downcast_rs::impl_downcast!(RenderGraphNode);

pub struct NodeInfo {
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
    create_textures: HashMap<ResourceId, TextureDesc>,
    create_buffers: HashMap<ResourceId, BufferDesc>,
}

impl NodeInfo {
    pub fn new() -> Self {
        Self {
            reads: Vec::new(),
            writes: Vec::new(),
            create_textures: HashMap::new(),
            create_buffers: HashMap::new(),
        }
    }

    pub fn reads(&self) -> &[ResourceId] {
        &self.reads
    }

    pub fn writes(&self) -> &[ResourceId] {
        &self.writes
    }

    pub fn read(&mut self, resource: impl Into<ResourceId>) {
        self.reads.push(resource.into());
    }

    pub fn write(&mut self, resource: impl Into<ResourceId>) {
        self.writes.push(resource.into());
    }

    pub fn create_texture(&mut self, id: impl Into<ResourceId>, desc: TextureDesc) {
        self.create_textures.insert(id.into(), desc);
    }

    pub fn create_buffer(&mut self, id: impl Into<ResourceId>, desc: BufferDesc) {
        self.create_buffers.insert(id.into(), desc);
    }
}

pub struct NodeGroupInfo {
    pub reads: HashSet<ResourceId>,
    pub writes: HashSet<ResourceId>,
    pub create_textures: HashMap<ResourceId, TextureDesc>,
    pub create_buffers: HashMap<ResourceId, BufferDesc>,
    pub nodes: Vec<usize>,
}

impl NodeGroupInfo {
    pub fn new() -> Self {
        Self {
            reads: HashSet::new(),
            writes: HashSet::new(),
            create_textures: HashMap::new(),
            create_buffers: HashMap::new(),
            nodes: Vec::new(),
        }
    }

    pub fn has_dependency(&self, info: &NodeInfo) -> bool {
        info.reads()
            .iter()
            .any(|handle| self.writes.contains(handle))
            || info
                .writes()
                .iter()
                .all(|handle| self.writes.contains(handle) || self.reads.contains(handle))
    }

    pub fn add_node(&mut self, index: usize, mut info: NodeInfo) {
        self.nodes.push(index);
        self.reads.extend(info.reads.drain(..));
        self.writes.extend(info.writes.drain(..));
        self.create_textures.extend(info.create_textures.drain());
        self.create_buffers.extend(info.create_buffers.drain());
    }
}

#[derive(Default)]
pub struct NodeGroup {
    pub create_textures: HashMap<ResourceId, TextureDesc>,
    pub create_buffers: HashMap<ResourceId, BufferDesc>,
    pub nodes: Vec<usize>,
}

impl From<NodeGroupInfo> for NodeGroup {
    fn from(info: NodeGroupInfo) -> Self {
        Self {
            create_textures: info.create_textures,
            create_buffers: info.create_buffers,
            nodes: info.nodes,
        }
    }
}
