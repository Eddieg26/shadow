use crate::{
    camera::RenderFrames,
    core::{
        device::{RenderDevice, RenderInstance, RenderQueue},
        surface::RenderSurface,
    },
    renderer::{
        draw::{Draw, DrawCalls},
        graph::{resources::RenderTargetDesc, RenderGraph, RenderGraphBuilder},
    },
    resources::{
        mesh::{Mesh, MeshBuffers},
        shader::{ShaderSource, Shaders},
        texture::GraphicsTextures,
        RenderAsset, RenderAssetUsage, RenderResource,
    },
};
use asset::{
    database::events::{AssetLoaded, AssetUnloaded},
    plugin::AssetExt,
    AssetId, Assets,
};
use ecs::{
    core::Resource,
    system::SystemArg,
    world::{
        event::{Event, Events},
        World,
    },
};
use game::{
    app::{Extract, MainWorld},
    game::Game,
    phases::{PostUpdate, Update},
    plugin::{Plugin, Plugins},
};
use std::marker::PhantomData;
use window::{
    events::{Resized, WindowCreated},
    plugin::WindowPlugin,
    window::Window,
};

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn dependencies(&self) -> Plugins {
        let mut plugins = Plugins::new();
        plugins.add_plugin(WindowPlugin);
        plugins
    }

    fn start(&self, game: &mut Game) {
        game.add_draw_calls::<()>(|| vec![]);
        game.add_resource(RenderFrames::new());
        game.add_resource(RenderGraphBuilder::new());
        game.add_render_resource(GraphicsTextures::new());
        game.add_render_resource(MeshBuffers::new());
        game.add_render_resource(Shaders::new());

        //TODO: Register Texture, Mesh, Material Loaders
        game.register_asset::<Mesh>();
        game.add_importer::<ShaderSource>();
        game.register_render_asset::<ShaderSource>();
        game.register_event::<SurfaceCreated>();

        game.add_system(Extract, extract_render_frames);
        game.add_system(Update, update_render_graph);
    }

    fn run(&mut self, game: &mut Game) {
        game.observe::<WindowCreated, _>(on_window_created);
        game.observe::<Resized, _>(on_window_resized);
    }

    fn finish(&mut self, game: &mut Game) {
        let render_graph = match game.remove_resource::<RenderGraphBuilder>() {
            Some(builder) => builder.build(),
            None => RenderGraph::default(),
        };

        game.add_resource(render_graph);
    }
}

fn on_window_created(
    _: &[WindowCreated],
    window: &Window,
    events: &Events,
    graph: &mut RenderGraph,
) {
    let create = || async {
        let instance = RenderInstance::create();

        let mut surface = match RenderSurface::create(&instance, window).await {
            Ok(surface) => surface,
            Err(_) => todo!(),
        };

        let (device, queue) = match RenderDevice::create(surface.adapter()).await {
            Ok(res) => res,
            Err(_) => todo!(),
        };

        surface.configure(&device);

        let desc = RenderTargetDesc::new(
            surface.width(),
            surface.height(),
            surface.format(),
            surface.depth_format(),
        );

        let id = surface.id();

        events.add(SurfaceCreated::new(surface, device.clone(), queue));

        (id, desc, device)
    };

    let (id, desc, device) = pollster::block_on(create());
    graph.add_render_target(&device, id, desc);
}

fn on_window_resized(events: &[Resized], device: &RenderDevice, surface: &mut RenderSurface) {
    if let Some(event) = events.last() {
        surface.resize(&device, event.width, event.height);
    }
}

fn extract_draw_calls<D: Draw>(main_world: &MainWorld, calls: &mut DrawCalls<D>) {
    let main = main_world.resource_mut::<DrawCalls<D>>();
    calls.extract(main);
}

fn clear_draw_calls<D: Draw>(calls: &mut DrawCalls<D>) {
    calls.clear();
}

fn extract_render_frames(main_world: &MainWorld, frames: &mut RenderFrames) {
    let main = main_world.resource_mut::<RenderFrames>();
    frames.extract(main);
}

fn update_render_graph(world: &World, graph: &mut RenderGraph) {
    graph.run(world);
}

pub struct SurfaceCreated {
    surface: RenderSurface,
    device: RenderDevice,
    queue: RenderQueue,
}

impl SurfaceCreated {
    pub fn new(surface: RenderSurface, device: RenderDevice, queue: RenderQueue) -> Self {
        Self {
            surface,
            device,
            queue,
        }
    }
}

impl Event for SurfaceCreated {
    type Output = (RenderDevice, RenderQueue);

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.add_resource(self.surface);
        world.add_resource(self.device.clone());
        world.add_resource(self.queue.clone());

        Some((self.device, self.queue))
    }
}

pub struct RenderAssetActions<A: RenderAsset> {
    extract: Vec<AssetId>,
    remove: Vec<AssetId>,
    _phantom: PhantomData<A>,
}

impl<A: RenderAsset> RenderAssetActions<A> {
    pub fn new() -> Self {
        Self {
            extract: Vec::new(),
            remove: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn extract(&mut self, id: AssetId) {
        self.extract.push(id);
    }

    pub fn remove(&mut self, id: AssetId) {
        self.remove.push(id);
    }
}

impl<A: RenderAsset> Resource for RenderAssetActions<A> {}
impl<A: RenderAsset> Default for RenderAssetActions<A> {
    fn default() -> Self {
        Self {
            extract: Default::default(),
            remove: Default::default(),
            _phantom: Default::default(),
        }
    }
}

pub fn on_asset_unloaded<R: RenderAsset>(
    assets: &[<AssetUnloaded<R::Asset> as Event>::Output],
    actions: &mut RenderAssetActions<R>,
) {
    for asset in assets {
        actions.remove(asset.id());
    }
}

pub fn on_asset_loaded<R: RenderAsset>(
    assets: &[<AssetLoaded<R::Asset> as Event>::Output],
    actions: &mut RenderAssetActions<R>,
) {
    for asset in assets {
        actions.remove(*asset);
    }
}

pub fn extract_render_asset<R: RenderAsset>(main: &MainWorld, world: &World) {
    let mut arg = <R::Arg<'static> as SystemArg>::get(world);
    let actions = main.resource_mut::<RenderAssetActions<R>>();
    let assets = main.resource_mut::<Assets<R::Asset>>();
    for id in actions.extract.drain(..) {
        let usage = match assets.get_mut(&id) {
            Some(asset) => R::extract(&id, asset, &mut arg),
            None => continue,
        };

        match usage {
            Ok(usage) => {
                if usage == RenderAssetUsage::Discard {
                    assets.remove(&id);
                }
            }
            Err(_) => continue,
        }
    }
}

pub fn remove_render_asset<'a, R: RenderAsset>(main: &MainWorld, world: &World) {
    let mut arg = <R::Arg<'static> as SystemArg>::get(world);
    let actions = main.resource_mut::<RenderAssetActions<R>>();

    for id in actions.remove.drain(..) {
        R::remove(id, &mut arg);
    }
}

pub trait GraphicsExt {
    fn add_draw_calls<D: Draw>(&mut self, partition: impl Fn() -> D::Partition);
    fn add_render_resource<R: RenderResource>(&mut self, resource: R);
    fn register_render_asset<R: RenderAsset>(&mut self);
}

impl GraphicsExt for Game {
    fn add_draw_calls<D: Draw>(&mut self, partition: impl Fn() -> D::Partition) {
        self.add_resource(DrawCalls::<D>::new(partition()));
        self.add_resource(DrawCalls::<D>::new(partition()));
        self.add_system(Extract, extract_draw_calls::<D>);
        self.add_system(PostUpdate, clear_draw_calls::<D>);
    }

    fn add_render_resource<R: RenderResource>(&mut self, resource: R) {
        self.add_resource(resource);
    }

    fn register_render_asset<R: RenderAsset>(&mut self) {
        self.register_asset::<R::Asset>();
        self.init_resource::<RenderAssetActions<R>>();
        self.add_system(Extract, remove_render_asset::<R>);
        self.add_system(Extract, extract_render_asset::<R>);
    }
}
