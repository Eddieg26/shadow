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
use wgpu::TextureFormat;

#[derive(Clone)]
pub struct RenderGraphContext<'a> {
    surface: &'a RenderSurface,
    device: &'a RenderDevice,
    queue: &'a RenderQueue,
    resources: &'a RenderGraphResources,
    world: &'a World,
    buffers: Arc<Mutex<Vec<wgpu::CommandBuffer>>>,
    target_override: Option<GpuResourceId>,
}

impl<'a> RenderGraphContext<'a> {
    pub fn new(
        surface: &'a RenderSurface,
        device: &'a RenderDevice,
        queue: &'a RenderQueue,
        world: &'a World,
        resources: &'a RenderGraphResources,
        buffers: Arc<Mutex<Vec<wgpu::CommandBuffer>>>,
    ) -> Self {
        Self {
            surface,
            device,
            queue,
            world,
            resources,
            buffers,
            target_override: None,
        }
    }

    pub fn with_target_override(&self, id: GpuResourceId) -> Self {
        let mut ctx = self.clone();
        ctx.target_override = Some(id);
        ctx
    }

    pub fn surface(&self) -> &'a RenderSurface {
        self.surface
    }

    pub fn device(&self) -> &'a RenderDevice {
        self.device
    }

    pub fn queue(&self) -> &'a RenderQueue {
        self.queue
    }

    pub fn resources(&self) -> &'a RenderGraphResources {
        self.resources
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.world.resource::<R>()
    }

    pub fn target(&self) -> Option<&RenderTarget> {
        match self.target_override {
            Some(id) => self.resources.render_target(id),
            None => self.resources.render_target(self.surface.id()),
        }
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
            Some(id) => Cow::Owned(ctx.with_target_override(id)),
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

    pub fn add_render_target(
        &mut self,
        device: &wgpu::Device,
        id: impl Into<GpuResourceId>,
        info: RenderTargetInfo,
    ) {
        self.resources.add_render_target(device, id, info);
    }

    pub fn add_texture(&mut self, id: impl Into<GpuResourceId>, info: TextureInfo) {
        self.resources.add_texture(id, info);
    }

    pub fn add_buffer(&mut self, id: impl Into<GpuResourceId>, info: BufferInfo) {
        self.resources.add_buffer(id, info);
    }

    pub fn remove_render_target(&mut self, id: impl Into<GpuResourceId>) -> Option<RenderTarget> {
        self.resources.remove_render_target(id)
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
        let id = GpuResourceId::gen();
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

        let texture = self.resources.set_surface_texture(surface).unwrap();

        for level in &self.hierarchy {
            let ctx = RenderGraphContext::new(
                surface,
                &device,
                &queue,
                world,
                &self.resources,
                Arc::new(Mutex::new(Vec::<wgpu::CommandBuffer>::new())),
            );

            for index in level {
                let node = &self.nodes[*index];
                node.execute(&ctx);
            }

            queue.submit(ctx.finish());
        }

        texture.present();

        self.resources.clear_surface_texture(surface);
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

pub struct RenderTargetInfo {
    width: u32,
    height: u32,
    format: Option<TextureFormat>,
    depth_format: Option<TextureFormat>,
    mipmaps: bool,
}

pub struct RenderTarget {
    width: u32,
    height: u32,
    color_texture: Option<wgpu::TextureView>,
    depth_texture: Option<wgpu::TextureView>,
    textures: HashMap<GpuResourceId, wgpu::TextureView>,
}

impl RenderTarget {
    pub fn create(
        device: &wgpu::Device,
        info: RenderTargetInfo,
        textures: &HashMap<GpuResourceId, TextureInfo>,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: info.width,
            height: info.height,
            depth_or_array_layers: 1,
        };

        let mip_level_count = match info.mipmaps {
            true => size.max_mips(wgpu::TextureDimension::D2),
            false => 1,
        };

        let color_texture = info.format.map(|format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size,
                    mip_level_count,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[format],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        });

        let depth_texture = info.depth_format.map(|format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size,
                    mip_level_count,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[format],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        });

        let textures = textures.iter().map(|(id, info)| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: info.format,
                usage: info.usage,
                view_formats: &[info.format],
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            (*id, view)
        });

        Self {
            width: info.width,
            height: info.height,
            color_texture,
            depth_texture,
            textures: textures.collect(),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn color_texture(&self) -> Option<&wgpu::TextureView> {
        self.color_texture.as_ref()
    }

    pub fn depth_texture(&self) -> Option<&wgpu::TextureView> {
        self.depth_texture.as_ref()
    }

    pub fn texture(&self, id: impl Into<GpuResourceId>) -> Option<&wgpu::TextureView> {
        self.textures.get(&id.into())
    }

    pub(crate) fn set_surface_texture(
        &mut self,
        surface: &RenderSurface,
    ) -> Option<wgpu::SurfaceTexture> {
        let surface = surface.surface_texture().ok()?;
        let view = surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.color_texture = Some(view);
        Some(surface)
    }

    pub(crate) fn clear_surface_texture(&mut self) {
        self.color_texture = None;
    }
}

pub struct RenderGraphResources {
    texture_infos: HashMap<GpuResourceId, TextureInfo>,
    buffer_infos: HashMap<GpuResourceId, BufferInfo>,
    targets: HashMap<GpuResourceId, RenderTarget>,
    buffers: HashMap<GpuResourceId, wgpu::Buffer>,
}

impl RenderGraphResources {
    pub fn new() -> Self {
        Self {
            texture_infos: HashMap::new(),
            buffer_infos: HashMap::new(),
            targets: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn render_target(&self, id: impl Into<GpuResourceId>) -> Option<&RenderTarget> {
        self.targets.get(&id.into())
    }

    pub fn texture(
        &self,
        target: impl Into<GpuResourceId>,
        id: impl Into<GpuResourceId>,
    ) -> Option<&wgpu::TextureView> {
        self.targets
            .get(&target.into())
            .and_then(|target| target.texture(id))
    }

    pub fn buffer(&self, id: impl Into<GpuResourceId>) -> Option<&wgpu::Buffer> {
        self.buffers.get(&id.into())
    }

    pub fn add_render_target(
        &mut self,
        device: &wgpu::Device,
        id: impl Into<GpuResourceId>,
        info: RenderTargetInfo,
    ) -> Option<RenderTarget> {
        let render_target = RenderTarget::create(device, info, &self.texture_infos);
        self.targets.insert(id.into(), render_target)
    }

    pub fn add_texture(&mut self, id: impl Into<GpuResourceId>, info: TextureInfo) {
        self.texture_infos.insert(id.into(), info);
    }

    pub fn add_buffer(&mut self, id: impl Into<GpuResourceId>, info: BufferInfo) {
        self.buffer_infos.insert(id.into(), info);
    }

    pub fn remove_render_target(&mut self, id: impl Into<GpuResourceId>) -> Option<RenderTarget> {
        self.targets.remove(&id.into())
    }

    pub(crate) fn set_surface_texture(
        &mut self,
        surface: &RenderSurface,
    ) -> Option<wgpu::SurfaceTexture> {
        let id = surface.id();
        let target = self.targets.get_mut(&id)?;
        target.set_surface_texture(surface)
    }

    pub(crate) fn clear_surface_texture(&mut self, surface: &RenderSurface) -> Option<()> {
        let id = surface.id();
        let target = self.targets.get_mut(&id)?;
        Some(target.clear_surface_texture())
    }
}

pub struct Render {
    pub clear_color: Option<wgpu::Color>,
    pub target: Option<GpuResourceId>,
}

pub struct Renders {
    pub renders: Vec<Render>,
}

impl Resource for Renders {}
