use crate::{
    core::{
        device::{RenderDevice, RenderQueue},
        surface::RenderSurface,
    },
    resources::ResourceId,
};
use context::{RenderContext, RenderNodeAction};
use resources::{BufferDesc, RenderGraphResources, RenderTarget, RenderTargetDesc, TextureDesc};
use shadow_ecs::{core::{DenseMap, Resource}, world::World};
use std::collections::{HashMap, HashSet};

pub mod context;
pub mod resources;

pub struct NodeEdge {
    from: ResourceId,
    to: ResourceId,
}

impl NodeEdge {
    pub fn new(from: ResourceId, to: ResourceId) -> Self {
        Self { from, to }
    }

    pub fn from(&self) -> ResourceId {
        self.from
    }

    pub fn to(&self) -> ResourceId {
        self.to
    }
}

pub struct RenderGraphBuilder {
    nodes: DenseMap<ResourceId, Box<dyn RenderGraphNode>>,
    resources: RenderGraphResources,
    dependencies: HashMap<ResourceId, HashSet<ResourceId>>,
    target: Option<ResourceId>,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Self {
            nodes: DenseMap::new(),
            resources: RenderGraphResources::new(),
            dependencies: HashMap::new(),
            target: None,
        }
    }

    pub fn node<T: RenderGraphNode>(&self, id: impl Into<ResourceId>) -> Option<&T> {
        self.nodes
            .get(&id.into())
            .map(|n| n.downcast_ref::<T>())
            .flatten()
    }

    pub fn node_mut<T: RenderGraphNode>(&mut self, id: impl Into<ResourceId>) -> Option<&mut T> {
        self.nodes
            .get_mut(&id.into())
            .map(|n| n.downcast_mut::<T>())
            .flatten()
    }

    pub fn add_node<T: RenderGraphNode>(
        &mut self,
        id: impl Into<ResourceId>,
        node: T,
    ) -> &mut Self {
        self.nodes.insert(id.into(), Box::new(node));
        self
    }

    pub fn add_edge(
        &mut self,
        from: impl Into<ResourceId>,
        to: impl Into<ResourceId>,
    ) -> &mut Self {
        let from = from.into();
        let to = to.into();

        self.dependencies
            .entry(to)
            .or_insert_with(HashSet::new)
            .insert(from);

        self
    }

    pub fn set_target(&mut self, id: impl Into<ResourceId>) -> &mut Self {
        self.target = Some(id.into());
        self
    }

    pub fn create_texture(&mut self, id: impl Into<ResourceId>, desc: TextureDesc) {
        self.resources.create_texture(id.into(), desc);
    }

    pub fn create_buffer(&mut self, id: impl Into<ResourceId>, desc: BufferDesc) {
        self.resources.create_buffer(id.into(), desc);
    }

    pub fn import_texture(&mut self, id: impl Into<ResourceId>, texture: wgpu::TextureView) {
        self.resources.import_texture(id.into(), texture);
    }

    pub fn import_buffer(&mut self, id: impl Into<ResourceId>, buffer: wgpu::Buffer) {
        self.resources.import_buffer(id.into(), buffer);
    }

    pub fn into_sub_graph(&mut self) -> ResourceId {
        let mut builder = RenderGraphBuilder::new();
        builder.nodes = std::mem::take(&mut self.nodes);
        builder.dependencies = std::mem::take(&mut self.dependencies);
        builder.target = self.target.take();
        let id = ResourceId::gen();
        self.nodes.insert(id, Box::new(builder.build()));

        id
    }

    pub fn build(mut self) -> RenderGraph {
        let mut order = vec![];
        let mut nodes = vec![];
        while !self.nodes.is_empty() {
            let mut group = vec![];
            for id in self.nodes.keys() {
                match self.dependencies.get(id) {
                    Some(deps) => {
                        if deps.iter().all(|dep| !self.nodes.contains(dep)) {
                            group.push(*id);
                        }
                    }
                    None => group.push(*id),
                }
            }

            if group.is_empty() {
                panic!("Cyclic dependency detected");
            }

            let mut indexes = vec![];
            for id in group {
                if let Some(node) = self.nodes.remove(&id) {
                    indexes.push(nodes.len());
                    nodes.push(node)
                }
            }

            if !indexes.is_empty() {
                order.push(indexes);
            }
        }

        RenderGraph::new(self.resources, nodes, order)
    }
}

impl Resource for RenderGraphBuilder {}

pub struct RenderGraph {
    resources: RenderGraphResources,
    nodes: Vec<Box<dyn RenderGraphNode>>,
    order: Vec<Vec<usize>>,
}

impl RenderGraph {
    fn new(
        resources: RenderGraphResources,
        nodes: Vec<Box<dyn RenderGraphNode>>,
        order: Vec<Vec<usize>>,
    ) -> Self {
        Self {
            resources,
            nodes,
            order,
        }
    }

    pub fn add_render_target(
        &mut self,
        device: &RenderDevice,
        id: impl Into<ResourceId>,
        desc: RenderTargetDesc,
    ) {
        self.resources.create_target(device, id.into(), desc);
    }

    pub fn remove_render_target(&mut self, id: impl Into<ResourceId>) -> Option<RenderTarget> {
        self.resources.remove_render_target(id.into())
    }

    pub fn run(&mut self, world: &World) {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();
        let surface = world.resource::<RenderSurface>();

        let texture = surface.surface_texture().unwrap();
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.resources.set_target_color(surface.id(), Some(view));

        let mut actions = vec![];
        for indexes in &self.order {
            let ctx = RenderContext::new(surface.id(), device, queue, &self.resources, world);
            for index in indexes {
                self.nodes[*index].execute(&ctx);
            }

            let mut local = ctx.finish();
            local.push(RenderNodeAction::Flush);

            actions.extend(local);
        }

        let mut buffers = vec![];
        while !actions.is_empty() {
            for i in 0..actions.len() {
                let action = actions.remove(i);
                match action {
                    RenderNodeAction::Submit(buffer) => buffers.push(buffer),
                    RenderNodeAction::Flush => {
                        queue.submit(std::mem::take(&mut buffers));
                        buffers.clear();
                    }
                }
            }
        }

        self.resources.set_target_color(surface.id(), None);

        texture.present();
    }
}

impl Resource for RenderGraph {}


pub trait RenderGraphNode: downcast_rs::Downcast + 'static {
    fn execute(&self, ctx: &RenderContext);
}

impl RenderGraphNode for RenderGraph {
    fn execute(&self, ctx: &RenderContext) {
        for indexes in &self.order {
            let local = ctx.clone();
            for index in indexes {
                self.nodes[*index].execute(&local);
            }

            let mut actions = local.finish();
            actions.push(RenderNodeAction::Flush);

            ctx.append_actions(actions);
        }
    }
}

downcast_rs::impl_downcast!(RenderGraphNode);
