use crate::{
    components::{Camera, RenderFrames},
    core::{
        device::{RenderDevice, RenderInstance, RenderQueue},
        surface::RenderSurface,
    },
    renderer::{
        draw::{Draw, DrawCalls},
        graph::{resources::RenderTargetDesc, RenderGraph, RenderGraphBuilder},
    },
    resources::{
        mesh::{model::ObjImporter, Mesh, MeshBuffers},
        shader::{Shader, ShaderSource},
        texture::{render::RenderTexture, GraphicsTexture, Texture2d},
        AssetUsage, RenderAsset, RenderAssetExtractor, RenderAssets,
    },
};
use asset::{
    plugin::{AssetExt, AssetPlugin},
    AssetAction, AssetActions, Assets,
};
use ecs::{
    system::SystemArg,
    world::{
        event::{Event, Events},
        World,
    },
};
use game::{
    app::{Extract, MainWorld},
    game::Game,
    phases::{First, Update},
    plugin::{Plugin, Plugins},
};
use window::{
    events::{Resized, WindowCreated},
    plugin::WindowPlugin,
    window::Window,
};

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn dependencies(&self) -> Plugins {
        let mut plugins = Plugins::new();
        plugins.add_plugin(AssetPlugin);
        plugins.add_plugin(WindowPlugin);
        plugins
    }

    fn start(&self, game: &mut Game) {        
        game.add_resource(RenderFrames::new());
        game.add_resource(RenderGraphBuilder::new());

        game.add_render_asset::<GraphicsTexture>();
        game.add_render_asset::<MeshBuffers>();
        game.add_render_asset::<Shader>();

        game.add_importer::<ShaderSource>();
        game.add_importer::<Texture2d>();
        game.add_importer::<RenderTexture>();
        game.add_importer::<ObjImporter>();

        game.register_asset::<Mesh>();
        game.register::<Camera>();
    }

    fn run(&mut self, game: &mut Game) {
        game.observe::<WindowCreated, _>(on_window_created);
        game.observe::<Resized, _>(on_window_resized);

        game.register_event::<SurfaceCreated>();
        game.add_system(Extract, extract_render_frames);
        game.add_system(Update, update_render_graph);
    }

    fn finish(&mut self, game: &mut Game) {
        let render_graph = match game.remove_resource::<RenderGraphBuilder>() {
            Some(builder) => builder.build(),
            None => RenderGraph::default(),
        };

        game.add_resource(render_graph);
    }
}

pub struct RenderApp;

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

pub fn extract_render_asset<R: RenderAssetExtractor>(
    main: &MainWorld,
    world: &World,
    assets: &mut RenderAssets<R::Target>,
) {
    let sources = main.resource_mut::<Assets<R::Source>>();
    let actions = main.resource_mut::<AssetActions<R::Source>>();
    for action in actions.iter() {
        match action {
            AssetAction::Added(id) => {
                let source = match sources.get_mut(&id) {
                    Some(source) => source,
                    None => continue,
                };

                let arg = <R::Arg<'static> as SystemArg>::get(world);

                if let Some(asset) = R::extract(source, &arg) {
                    assets.insert(id.into(), asset);
                    if R::usage(&source) == AssetUsage::Discard {
                        sources.remove(id);
                    }
                }
            }
            AssetAction::Updated(id) => {
                let source = match sources.get_mut(&id) {
                    Some(source) => source,
                    None => continue,
                };

                let asset = match assets.get_mut(&id.into()) {
                    Some(asset) => asset,
                    None => continue,
                };

                let arg = <R::Arg<'static> as SystemArg>::get(world);
                R::update(source, asset, &arg);
            }
            AssetAction::Removed(id) => {
                assets.remove(&id.into());
            }
            _ => (),
        }
    }
}

pub trait GraphicsExt {
    fn add_draw_calls<D: Draw>(&mut self, partition: impl Fn() -> D::Partition);
    fn add_render_asset<R: RenderAsset>(&mut self);
    fn add_render_asset_extractor<R: RenderAssetExtractor>(&mut self);
}

impl GraphicsExt for Game {
    fn add_draw_calls<D: Draw>(&mut self, partition: impl Fn() -> D::Partition) {
        self.add_resource(DrawCalls::<D>::new(partition()));
        self.add_resource(DrawCalls::<D>::new(partition()));
        self.add_system(Extract, extract_draw_calls::<D>);
        self.add_system(First, clear_draw_calls::<D>);
    }

    fn add_render_asset<R: RenderAsset>(&mut self) {
        if !self.has_resource::<RenderAssets<R>>() {
            self.init_resource::<RenderAssets<R>>();
        }
    }

    fn add_render_asset_extractor<R: RenderAssetExtractor>(&mut self) {
        self.add_render_asset::<R::Target>();
        self.register_asset::<R::Source>();

        self.add_system(Extract, extract_render_asset::<R>);
    }
}
