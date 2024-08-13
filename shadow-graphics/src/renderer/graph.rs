use crate::{
    core::{
        device::{RenderDevice, RenderQueue},
        surface::RenderSurface,
    },
    resources::{buffer::BufferInfo, GpuResourceId},
};
use downcast_rs::{impl_downcast, Downcast};
use shadow_ecs::{
    core::{DenseMap, DenseSet, Resource},
    world::World,
};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct RenderGraphContext<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    resources: &'a RenderGraphResources,
    world: &'a World,
    buffers: Arc<Mutex<Vec<wgpu::CommandBuffer>>>,
    target: &'a wgpu::TextureView,
    surface_override: Option<GpuResourceId>,
}

impl<'a> RenderGraphContext<'a> {
    pub fn new(
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        world: &'a World,
        resources: &'a RenderGraphResources,
        buffers: Arc<Mutex<Vec<wgpu::CommandBuffer>>>,
        target: &'a wgpu::TextureView,
    ) -> Self {
        Self {
            device,
            queue,
            world,
            resources,
            buffers,
            target,
            surface_override: None,
        }
    }

    pub fn with_surface_override(&self, id: GpuResourceId) -> Self {
        let mut ctx = self.clone();
        ctx.surface_override = Some(id);
        ctx
    }

    pub fn device(&self) -> &'a wgpu::Device {
        self.device
    }

    pub fn queue(&self) -> &'a wgpu::Queue {
        self.queue
    }

    pub fn resources(&self) -> &'a RenderGraphResources {
        self.resources
    }

    pub fn target(&self) -> &'a wgpu::TextureView {
        match self.surface_override {
            Some(id) => self.resources.texture(id).unwrap_or(&self.target),
            None => self.target,
        }
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.world.resource::<R>()
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
    }

    pub fn add_buffer(&self, buffer: wgpu::CommandBuffer) {
        self.buffers.lock().unwrap().push(buffer);
    }

    pub fn finish(self) -> Vec<wgpu::CommandBuffer> {
        self.buffers.lock().unwrap().drain(..).collect()
    }
}

pub trait RenderGraphNode: Downcast + Send + Sync + 'static {
    fn execute(&self, ctx: &RenderGraphContext);
}

impl_downcast!(RenderGraphNode);

pub trait RenderGraphNodeBuilder: Downcast + Send + Sync + 'static {
    fn build(&self, world: &World) -> Box<dyn RenderGraphNode>;
}

impl_downcast!(RenderGraphNodeBuilder);

pub struct SubGraph {
    nodes: Vec<Box<dyn RenderGraphNode>>,
    hierarchy: Vec<Vec<usize>>,
    surface_override: Option<GpuResourceId>,
}

impl RenderGraphNode for SubGraph {
    fn execute(&self, ctx: &RenderGraphContext) {
        let ctx = match self.surface_override {
            Some(id) => Cow::Owned(ctx.with_surface_override(id)),
            None => Cow::Borrowed(ctx),
        };

        for level in &self.hierarchy {
            for index in level {
                let node = &self.nodes[*index];
                node.execute(&ctx);
            }
        }
    }
}

pub struct SubGraphBuilder {
    nodes: DenseMap<GpuResourceId, Box<dyn RenderGraphNodeBuilder>>,
    dependencies: DenseMap<GpuResourceId, Vec<GpuResourceId>>,
    surface_override: Option<GpuResourceId>,
}

impl SubGraphBuilder {
    pub fn new() -> Self {
        Self {
            nodes: DenseMap::new(),
            dependencies: DenseMap::new(),
            surface_override: None,
        }
    }

    pub fn add_node<N: RenderGraphNodeBuilder>(&mut self, id: impl Into<GpuResourceId>, node: N) {
        self.nodes.insert(id.into(), Box::new(node));
    }

    pub fn add_dependency(
        &mut self,
        id: impl Into<GpuResourceId>,
        dependency: impl Into<GpuResourceId>,
    ) {
        let id = id.into();
        let dependency = dependency.into();

        if let Some(dependencies) = self.dependencies.get_mut(&id) {
            dependencies.push(dependency);
        } else {
            self.dependencies.insert(id, vec![dependency]);
        }
    }

    pub fn surface_override(&mut self, id: GpuResourceId) {
        self.surface_override = Some(id);
    }
}

impl RenderGraphNodeBuilder for SubGraphBuilder {
    fn build(&self, world: &World) -> Box<dyn RenderGraphNode> {
        let mut nodes = Vec::new();
        let mut hierarchy = Vec::new();
        let mut ids = self.nodes.keys().iter().collect::<DenseSet<_>>();

        while !ids.is_empty() {
            let mut level = Vec::new();
            for id in ids.iter() {
                let remove = match self.dependencies.get(id) {
                    Some(dependencies) => dependencies.iter().all(|id| !ids.contains(&id)),
                    None => true,
                };

                if remove {
                    level.push(self.nodes.index_of(id).unwrap());
                }
            }

            if !level.is_empty() {
                panic!("Circular dependency detected");
            }

            ids.retain(|id| {
                self.nodes
                    .index_of(id)
                    .map(|i| !level.contains(&i))
                    .unwrap_or(true)
            });

            hierarchy.push(level);
        }

        for (_, node) in self.nodes.iter() {
            nodes.push(node.build(&world));
        }

        Box::new(SubGraph {
            nodes,
            hierarchy,
            surface_override: self.surface_override,
        })
    }
}

pub struct RenderGraphBuilder {
    nodes: DenseMap<GpuResourceId, Box<dyn RenderGraphNodeBuilder>>,
    dependencies: DenseMap<GpuResourceId, Vec<GpuResourceId>>,
    resources: RenderGraphResources,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Self {
            nodes: DenseMap::new(),
            dependencies: DenseMap::new(),
            resources: RenderGraphResources::new(),
        }
    }

    pub fn add_node<N: RenderGraphNodeBuilder>(&mut self, id: impl Into<GpuResourceId>, node: N) {
        self.nodes.insert(id.into(), Box::new(node));
    }

    pub fn add_dependency(
        &mut self,
        id: impl Into<GpuResourceId>,
        dependency: impl Into<GpuResourceId>,
    ) {
        let id = id.into();
        let dependency = dependency.into();

        if let Some(dependencies) = self.dependencies.get_mut(&id) {
            dependencies.push(dependency);
        } else {
            self.dependencies.insert(id, vec![dependency]);
        }
    }

    pub fn add_texture(&mut self, id: impl Into<GpuResourceId>, info: TextureInfo) {
        self.resources.add_texture(id, info);
    }

    pub fn add_buffer(&mut self, id: impl Into<GpuResourceId>, info: BufferInfo) {
        self.resources.add_buffer(id, info);
    }

    pub fn import_texture(&mut self, id: impl Into<GpuResourceId>, texture: wgpu::TextureView) {
        self.resources.import_texture(id, texture);
    }

    pub fn import_buffer(&mut self, id: impl Into<GpuResourceId>, buffer: wgpu::Buffer) {
        self.resources.import_buffer(id, buffer);
    }

    pub fn remove_texture(&mut self, id: impl Into<GpuResourceId>) -> Option<wgpu::TextureView> {
        self.resources.remove_texture(id)
    }

    pub fn remove_buffer(&mut self, id: impl Into<GpuResourceId>) -> Option<wgpu::Buffer> {
        self.resources.remove_buffer(id)
    }

    pub fn node<N: RenderGraphNodeBuilder>(&self, id: impl Into<GpuResourceId>) -> Option<&N> {
        let node = self.nodes.get(&id.into())?;
        node.downcast_ref()
    }

    pub fn node_mut<N: RenderGraphNodeBuilder>(
        &mut self,
        id: impl Into<GpuResourceId>,
    ) -> Option<&mut N> {
        let node = self.nodes.get_mut(&id.into())?;
        node.downcast_mut()
    }

    pub fn into_sub_graph(
        &mut self,
        surface_override: Option<impl Into<GpuResourceId>>,
    ) -> GpuResourceId {
        let id = GpuResourceId::new();
        let mut sub_graph = SubGraphBuilder::new();

        for (id, node) in self.nodes.drain() {
            sub_graph.nodes.insert(id, node);
        }

        for (id, dependencies) in self.dependencies.drain() {
            sub_graph.dependencies.insert(id, dependencies);
        }

        if let Some(surface_override) = surface_override {
            sub_graph.surface_override(surface_override.into());
        }

        self.nodes.insert(id, Box::new(sub_graph));
        id
    }

    pub fn build(self, world: &World) -> RenderGraph {
        let mut nodes = Vec::new();
        let mut hierarchy = Vec::new();
        let mut ids = self.nodes.keys().iter().collect::<DenseSet<_>>();

        while !ids.is_empty() {
            let mut level = Vec::new();
            for id in ids.iter() {
                let remove = match self.dependencies.get(id) {
                    Some(dependencies) => dependencies.iter().all(|id| !ids.contains(&id)),
                    None => true,
                };

                if remove {
                    level.push(self.nodes.index_of(id).unwrap());
                }
            }

            if !level.is_empty() {
                panic!("Circular dependency detected");
            }

            ids.retain(|id| {
                self.nodes
                    .index_of(id)
                    .map(|i| !level.contains(&i))
                    .unwrap_or(true)
            });

            hierarchy.push(level);
        }

        for (_, node) in self.nodes.into_iter() {
            nodes.push(node.build(&world));
        }

        RenderGraph {
            nodes,
            hierarchy,
            resources: self.resources,
        }
    }
}

pub struct RenderGraph {
    nodes: Vec<Box<dyn RenderGraphNode>>,
    hierarchy: Vec<Vec<usize>>,
    resources: RenderGraphResources,
}

impl RenderGraph {
    pub fn execute(&mut self, world: &World) {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();
        let surface = world.resource::<RenderSurface>();

        let surface = match surface.surface_texture() {
            Ok(texture) => texture,
            Err(_) => return,
        };

        let view = surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        for level in &self.hierarchy {
            let ctx = RenderGraphContext::new(
                &device,
                &queue,
                world,
                &self.resources,
                Arc::new(Mutex::new(Vec::<wgpu::CommandBuffer>::new())),
                &view,
            );

            for index in level {
                let node = &self.nodes[*index];
                node.execute(&ctx);
            }

            queue.submit(ctx.finish());
        }

        surface.present();
    }
}

pub struct TextureInfo {
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}

impl TextureInfo {
    pub fn new(format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> Self {
        Self { format, usage }
    }
}

pub struct RenderGraphResources {
    texture_infos: HashMap<GpuResourceId, TextureInfo>,
    buffer_infos: HashMap<GpuResourceId, BufferInfo>,
    textures: HashMap<GpuResourceId, wgpu::TextureView>,
    buffers: HashMap<GpuResourceId, wgpu::Buffer>,
}

impl RenderGraphResources {
    pub fn new() -> Self {
        Self {
            texture_infos: HashMap::new(),
            buffer_infos: HashMap::new(),
            textures: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn texture(&self, id: impl Into<GpuResourceId>) -> Option<&wgpu::TextureView> {
        self.textures.get(&id.into())
    }

    pub fn buffer(&self, id: impl Into<GpuResourceId>) -> Option<&wgpu::Buffer> {
        self.buffers.get(&id.into())
    }

    pub fn add_texture(&mut self, id: impl Into<GpuResourceId>, info: TextureInfo) {
        self.texture_infos.insert(id.into(), info);
    }

    pub fn add_buffer(&mut self, id: impl Into<GpuResourceId>, info: BufferInfo) {
        self.buffer_infos.insert(id.into(), info);
    }

    pub fn import_texture(&mut self, id: impl Into<GpuResourceId>, texture: wgpu::TextureView) {
        self.textures.insert(id.into(), texture);
    }

    pub fn import_buffer(&mut self, id: impl Into<GpuResourceId>, buffer: wgpu::Buffer) {
        self.buffers.insert(id.into(), buffer);
    }

    pub fn remove_texture(&mut self, id: impl Into<GpuResourceId>) -> Option<wgpu::TextureView> {
        self.textures.remove(&id.into())
    }

    pub fn remove_buffer(&mut self, id: impl Into<GpuResourceId>) -> Option<wgpu::Buffer> {
        self.buffers.remove(&id.into())
    }

    pub fn build(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        for (id, info) in self.texture_infos.iter() {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: info.format,
                usage: info.usage,
                view_formats: &[info.format],
            });

            self.textures.insert(
                *id,
                texture.create_view(&wgpu::TextureViewDescriptor::default()),
            );
        }

        for (id, info) in self.buffer_infos.iter() {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: info.size,
                usage: info.usage,
                mapped_at_creation: info.mapped_at_creation,
            });

            self.buffers.insert(*id, buffer);
        }
    }
}

pub struct Render {
    pub clear_color: Option<wgpu::Color>,
}

pub struct Renders {
    pub renders: Vec<Render>,
}

impl Resource for Renders {}
