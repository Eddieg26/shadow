use crate::{
    camera::RenderFrame,
    core::device::{RenderDevice, RenderQueue},
    resources::ResourceId,
};
use context::{RenderContext, RenderNodeAction};
use ecs::{core::Resource, world::World};
use node::{NodeGroup, NodeGroupInfo, RenderGraphNode};
use resources::{
    BufferDesc, RenderGraphBuffer, RenderGraphResources, RenderGraphTexture, RenderTarget,
    TextureDesc,
};

pub mod context;
pub mod node;
pub mod pass;
pub mod resources;

pub struct RenderGraphBuilder {
    nodes: Vec<Box<dyn RenderGraphNode>>,
    resources: RenderGraphResources,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            resources: RenderGraphResources::new(),
        }
    }

    pub fn node<T: RenderGraphNode>(&self, name: &str) -> Option<&T> {
        self.nodes
            .iter()
            .find_map(|node| node.downcast_ref::<T>().filter(|node| node.name() == name))
    }

    pub fn node_mut<T: RenderGraphNode>(&mut self, name: &str) -> Option<&mut T> {
        self.nodes
            .iter_mut()
            .find_map(|node| node.downcast_mut::<T>().filter(|node| node.name() == name))
    }

    pub fn add_node<T: RenderGraphNode>(&mut self, node: T) -> &mut Self {
        self.nodes.push(Box::new(node));
        self
    }

    pub fn add_texture(&mut self, id: impl Into<ResourceId>, desc: TextureDesc) {
        self.resources.add_texture(id.into(), desc);
    }

    pub fn add_buffer(&mut self, id: impl Into<ResourceId>, desc: BufferDesc) {
        self.resources.add_buffer(id.into(), desc);
    }

    pub fn import_texture(&mut self, id: impl Into<ResourceId>, texture: RenderGraphTexture) {
        self.resources.import_texture(id.into(), texture);
    }

    pub fn import_buffer(&mut self, id: impl Into<ResourceId>, buffer: RenderGraphBuffer) {
        self.resources.import_buffer(id.into(), buffer);
    }

    pub fn build(self) -> RenderGraph {
        let mut groups = vec![NodeGroupInfo::new()];
        for (index, node) in self.nodes.iter().enumerate() {
            let info = node.info();
            let mut group_index = groups.len() - 1;
            if groups[group_index].has_dependency(&info) {
                let mut group = NodeGroupInfo::new();
                group.add_node(index, info);
                groups.push(group);
            } else if group_index == 0 {
                groups[0].add_node(index, info);
            } else {
                let mut has_dependency = false;
                while group_index > 0 {
                    group_index -= 1;
                    match groups[group_index].has_dependency(&info) {
                        true => {
                            has_dependency = true;
                            break;
                        }
                        false => (),
                    }
                }

                match has_dependency {
                    true => groups[group_index + 1].add_node(index, info),
                    false => {
                        let last = groups.len() - 1;
                        groups[last].add_node(index, info);
                    }
                }
            }
        }

        let groups = groups.drain(..).map(NodeGroup::from).collect();
        RenderGraph::new(self.resources, self.nodes, groups)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

impl Resource for RenderGraphBuilder {}

#[derive(Default)]
pub struct RenderGraph {
    resources: RenderGraphResources,
    nodes: Vec<Box<dyn RenderGraphNode>>,
    groups: Option<Vec<NodeGroup>>,
}

impl RenderGraph {
    fn new(
        resources: RenderGraphResources,
        nodes: Vec<Box<dyn RenderGraphNode>>,
        groups: Vec<NodeGroup>,
    ) -> Self {
        Self {
            resources,
            nodes,
            groups: Some(groups),
        }
    }

    pub fn resources(&self) -> &RenderGraphResources {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut RenderGraphResources {
        &mut self.resources
    }

    pub fn run(
        &mut self,
        world: &World,
        device: &RenderDevice,
        queue: &RenderQueue,
        frame: &RenderFrame,
        frame_index: usize,
        frame_count: usize,
        target: &RenderTarget,
    ) {
        let groups = match self.groups.take() {
            Some(groups) => groups,
            None => return,
        };
        
        for group in &groups {
            let mut actions = vec![];

            for index in &group.nodes {
                let ctx = RenderContext::new(
                    world,
                    &frame,
                    frame_index,
                    frame_count,
                    target,
                    device,
                    queue,
                    &self.resources,
                );
                self.nodes[*index].execute(&ctx);

                actions.extend(ctx.finish());
                actions.push(RenderNodeAction::Flush);
            }

            let mut buffers = vec![];
            for action in actions.drain(..) {
                match action {
                    RenderNodeAction::Submit(buffer) => buffers.push(buffer),
                    RenderNodeAction::Flush => {
                        if !buffers.is_empty() {
                            queue.submit(buffers.drain(..));
                            queue.on_submitted_work_done(|| {});
                        }
                    }
                }
            }
        }

        self.groups = Some(groups);
    }
}

impl Resource for RenderGraph {}
