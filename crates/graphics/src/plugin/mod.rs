use crate::{
    camera::{Camera, RenderCamera},
    core::device::{RenderDevice, RenderInstance, RenderQueue},
    renderer::{
        draw::{Draw, DrawCallExtractor, DrawCalls},
        graph::{
            node::RenderGraphNode,
            resources::{RenderGraphTexture, RenderTarget},
            RenderGraph, RenderGraphBuilder,
        },
        surface::{RenderSurface, RenderSurfaceTexture},
    },
    resources::{
        mesh::{loaders::ObjImporter, Mesh, MeshBuffers, SubMesh},
        shader::{Shader, ShaderSource},
        texture::{render::RenderTexture, GpuTexture, Texture2d},
        AssetUsage, DiscardedAssets, ExtractResource, ExtractedResource, RenderAsset,
        RenderAssetExtractor, RenderAssetWorld, RenderAssets, RenderResourceExtractor,
    },
};
use asset::{
    plugin::{AssetExt, AssetPlugin},
    AssetAction, AssetActions, Assets,
};
use ecs::{
    system::{schedule::Phase, IntoSystem, StaticSystemArg, System},
    world::{
        event::{AddResource, Event, Events},
        query::Query,
        World,
    },
};
use game::{
    app::Extract,
    game::Game,
    phases::Update,
    plugin::{Plugin, Plugins},
    Main, SubApp, SubEvents,
};
use spatial::{plugin::SpatialPlugin, transform::Transform, Size};
use window::{
    events::{Resized, WindowCreated},
    plugin::WindowPlugin,
    window::Window,
};

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn dependencies(&self) -> Plugins {
        let mut plugins = Plugins::new();
        plugins.add_plugin(SpatialPlugin);
        plugins.add_plugin(AssetPlugin);
        plugins.add_plugin(WindowPlugin);
        plugins
    }

    fn start(&self, game: &mut Game) {
        game.add_sub_app::<RenderApp>();
        game.add_resource(RenderGraphBuilder::new());

        game.add_render_asset::<GpuTexture>();
        game.add_render_asset::<MeshBuffers>();
        game.add_render_asset::<Shader>();
        game.add_render_asset::<RenderTarget>();

        game.add_importer::<ShaderSource>();
        game.add_importer::<Texture2d>();
        game.add_importer::<RenderTexture>();
        game.add_importer::<ObjImporter>();

        game.register_asset::<Mesh>();
        game.register_asset::<SubMesh>();
        game.register::<Camera>();

        let app = game.sub_app_mut::<RenderApp>().unwrap();
        app.init_resource::<RenderAssets<RenderCamera>>();
        app.init_resource::<RenderSurfaceTexture>();
        app.add_sub_phase::<Extract, ExtractBindGroup>();
        app.add_sub_phase::<Extract, ExtractPipeline>();
        app.add_sub_phase::<Extract, ExtractDrawCalls>();
        app.add_sub_phase::<Extract, PostExtract>();
        app.add_sub_phase::<Update, PreRender>();
        app.add_sub_phase::<Update, Render>();
        app.add_sub_phase::<Update, Present>();
        app.add_sub_phase::<Update, PostRender>();
    }

    fn run(&mut self, game: &mut Game) {
        game.add_render_asset_extractor::<Mesh>();
        game.add_render_asset_extractor::<Shader>();
        game.observe::<WindowCreated, _>(on_window_created);
        game.observe::<Resized, _>(|resized: &[Resized], events: &SubEvents<RenderApp>| {
            let resized = resized.last().unwrap();
            events.add(*resized);
        });

        let app = game.sub_app_mut::<RenderApp>().unwrap();
        app.register_event::<SurfaceCreated>();
        app.observe::<SurfaceCreated, _>(SurfaceCreated::observer);
        app.observe::<Resized, _>(on_window_resized);
        app.add_system(Extract, extract_cameras);
        app.add_system(Render, update_render_graph);
        app.add_system(Present, present_surface);
    }

    fn finish(&mut self, game: &mut Game) {
        let render_graph = match game.remove_resource::<RenderGraphBuilder>() {
            Some(builder) => builder.build(),
            None => RenderGraph::default(),
        };

        let app = game.sub_app_mut::<RenderApp>().unwrap();
        app.add_resource(render_graph);
    }
}

pub struct RenderApp;
impl SubApp for RenderApp {}

pub struct PreRender;
impl Phase for PreRender {}

pub struct Render;
impl Phase for Render {}

pub struct Present;
impl Phase for Present {}

pub struct PostRender;
impl Phase for PostRender {}

pub struct ExtractBindGroup;
impl Phase for ExtractBindGroup {}

pub struct ExtractPipeline;
impl Phase for ExtractPipeline {}

pub struct ExtractDrawCalls;
impl Phase for ExtractDrawCalls {}

pub struct PostExtract;
impl Phase for PostExtract {}

fn clear_draw_calls<D: Draw>(calls: &mut DrawCalls<D>) {
    calls.clear();
}

fn update_render_graph(
    world: &World,
    cameras: &RenderAssets<RenderCamera>,
    graph: &mut RenderGraph,
    targets: &mut RenderAssets<RenderTarget>,
) {
    let surface = world.resource::<RenderSurface>();
    let texture = match surface.texture() {
        Ok(surface) => surface,
        Err(_) => return,
    };

    match targets.get_mut(&surface.id()) {
        Some(target) => {
            let view = texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            target.set_color(Some(RenderGraphTexture::from(view)));
        }
        None => return,
    }

    let surface_target = unsafe { targets.get(&surface.id()).unwrap_unchecked() };

    let device = world.resource::<RenderDevice>();
    let queue = world.resource::<RenderQueue>();
    let camera_count = cameras.len();

    for (current, (_, camera)) in cameras.iter().enumerate() {
        let target = match camera.target.and_then(|id| targets.get(&id.into())) {
            Some(target) => target,
            None => surface_target,
        };

        graph.run(world, device, queue, target, &camera, current, camera_count);
    }

    let target = unsafe { targets.get_mut(&surface.id()).unwrap_unchecked() };
    target.set_color(None);

    world.resource_mut::<RenderSurfaceTexture>().set(texture);
}

#[inline]
fn present_surface(texture: &mut RenderSurfaceTexture) {
    texture.present();
}

fn extract_cameras(
    sources: Main<Query<(&Camera, &Transform)>>,
    cameras: &mut RenderAssets<RenderCamera>,
) {
    for (camera, transform) in sources.into_inner() {
        let render_camera = RenderCamera::new(camera, transform.matrix(None));
        cameras.add(camera.depth, render_camera);
    }

    cameras.sort(|a, b| a.cmp(b));
}

fn extract_render_asset_render_world<R: RenderAssetExtractor>(
    sources: Main<&mut Assets<R::Source>>,
    actions: Main<&AssetActions<R::Source>>,
    assets: &mut RenderAssets<R::Target>,
    discarded: &mut DiscardedAssets<R::Source>,
    arg: StaticSystemArg<R::Arg>,
) {
    extract_render_asset::<R>(sources, actions, assets, discarded, arg);
}

fn extract_render_asset_main_world<R: RenderAssetExtractor>(
    sources: Main<&mut Assets<R::Source>>,
    actions: Main<&AssetActions<R::Source>>,
    mut assets: Main<&mut RenderAssets<R::Target>>,
    discarded: &mut DiscardedAssets<R::Source>,
    arg: StaticSystemArg<R::Arg>,
) {
    extract_render_asset::<R>(sources, actions, &mut assets, discarded, arg);
}

fn extract_render_asset<R: RenderAssetExtractor>(
    mut sources: Main<&mut Assets<R::Source>>,
    actions: Main<&AssetActions<R::Source>>,
    assets: &mut RenderAssets<R::Target>,
    discarded: &mut DiscardedAssets<R::Source>,
    arg: StaticSystemArg<R::Arg>,
) {
    let mut arg = arg.into_inner();
    for action in actions.iter() {
        match action {
            AssetAction::Added(id) => {
                let source = match sources.get_mut(&id) {
                    Some(source) => source,
                    None => continue,
                };

                match R::extract(id, source, &mut arg, assets) {
                    Some(AssetUsage::Discard) => discarded.insert(*id),
                    _ => false,
                };
            }
            AssetAction::Removed(id) => {
                R::remove(id, assets, &mut arg);
            }
            _ => (),
        }
    }
}

fn extract_render_resource<R: RenderResourceExtractor>(
    events: &Events,
    arg: StaticSystemArg<R::Arg>,
) {
    if let Some(resource) = R::extract(&arg) {
        events.add(AddResource::new(resource));
    }
}

fn extract_draw_calls<D: DrawCallExtractor>(
    calls: &mut DrawCalls<D::Draw>,
    arg: StaticSystemArg<D::Arg>,
) {
    D::extract(calls, &arg);
}

pub fn remove_discarded_assets<R: RenderAssetExtractor>(
    mut sources: Main<&mut Assets<R::Source>>,
    discarded: &mut DiscardedAssets<R::Source>,
) {
    for id in discarded.drain() {
        sources.remove(&id);
    }
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

    pub fn surface(&self) -> &RenderSurface {
        &self.surface
    }

    pub fn device(&self) -> &RenderDevice {
        &self.device
    }

    pub fn queue(&self) -> &RenderQueue {
        &self.queue
    }

    pub fn observer(
        _: &[()],
        surface: &RenderSurface,
        device: &RenderDevice,
        targets: &mut RenderAssets<RenderTarget>,
    ) {
        targets.add(surface.id(), RenderTarget::from_surface(device, surface));
    }
}

impl Event for SurfaceCreated {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.add_resource(self.surface);
        world.add_resource(self.device);
        world.add_resource(self.queue);

        Some(())
    }
}

fn on_window_created(_: &[WindowCreated], window: &Window, render_events: &SubEvents<RenderApp>) {
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

        (surface, device, queue)
    };

    let (surface, device, queue) = pollster::block_on(create());
    render_events.add(SurfaceCreated::new(surface, device, queue));
}

fn on_window_resized(
    resized: &[Resized],
    device: &RenderDevice,
    surface: &mut RenderSurface,
    targets: &mut RenderAssets<RenderTarget>,
    graph: &mut RenderGraph,
) {
    let resized = resized.last().unwrap();
    let size = Size::new(resized.width, resized.height);

    surface.resize(device, size);
    targets.add(surface.id(), RenderTarget::from_surface(device, surface));
    graph.set_size(size);
}

pub trait GraphicsExt {
    fn add_draw_calls<D: Draw>(&mut self) -> &mut Self;
    fn add_draw_call_extractor<D: DrawCallExtractor>(&mut self) -> &mut Self;
    fn add_render_asset<R: RenderAsset>(&mut self) -> &mut Self;
    fn add_render_asset_extractor<R: RenderAssetExtractor>(&mut self) -> &mut Self;
    fn add_render_resource_extractor<R: RenderResourceExtractor>(&mut self) -> &mut Self;
    fn add_render_node<R: RenderGraphNode>(&mut self, node: R) -> &mut Self;
}

impl GraphicsExt for Game {
    fn add_draw_calls<D: Draw>(&mut self) -> &mut Self {
        let app = self.sub_app_mut::<RenderApp>().unwrap();
        if !app.has_resource::<DrawCalls<D>>() {
            app.add_resource(DrawCalls::<D>::new());
            app.add_system(PostRender, clear_draw_calls::<D>);
        }

        self
    }

    fn add_draw_call_extractor<D: DrawCallExtractor>(&mut self) -> &mut Self {
        self.add_draw_calls::<D::Draw>();

        let app = self.sub_app_mut::<RenderApp>().unwrap();
        app.add_system(ExtractDrawCalls, extract_draw_calls::<D>);
        self
    }

    fn add_render_asset<R: RenderAsset>(&mut self) -> &mut Self {
        match R::world() {
            RenderAssetWorld::Main => {
                if !self.has_resource::<RenderAssets<R>>() {
                    self.init_resource::<RenderAssets<R>>();
                }
            }
            RenderAssetWorld::Render => {
                let app = self.sub_app_mut::<RenderApp>().unwrap();
                if !app.has_resource::<RenderAssets<R>>() {
                    app.init_resource::<RenderAssets<R>>();
                }
            }
        }

        self
    }

    fn add_render_asset_extractor<R: RenderAssetExtractor>(&mut self) -> &mut Self {
        self.add_render_asset::<R::Target>();
        self.register_asset::<R::Source>();

        let app = self.sub_app_mut::<RenderApp>().unwrap();
        app.add_system(PostExtract, remove_discarded_assets::<R>);
        app.init_resource::<DiscardedAssets<R::Source>>();

        let system: System = match R::Target::world() {
            RenderAssetWorld::Main => extract_render_asset_main_world::<R>.into_system(),
            RenderAssetWorld::Render => extract_render_asset_render_world::<R>.into_system(),
        };

        match R::extracted_resource() {
            Some(ExtractedResource::BindGroup) => app.add_system(ExtractBindGroup, system),
            Some(ExtractedResource::Pipeline) => app.add_system(ExtractPipeline, system),
            None => app.add_system(Extract, system),
        };

        self
    }

    fn add_render_resource_extractor<R: RenderResourceExtractor>(&mut self) -> &mut Self {
        let app = self.sub_app_mut::<RenderApp>().unwrap();
        app.register_event::<ExtractResource<R>>();
        app.register_event::<AddResource<R::Target>>();

        match R::extracted_resource() {
            Some(ExtractedResource::BindGroup) => {
                app.add_system(ExtractBindGroup, extract_render_resource::<R>);
                app.observe::<R::Event, _>(
                    |_: &[<R::Event as Event>::Output], events: &SubEvents<RenderApp>| {
                        // events.add_deferred(ExtractBindGroup.id(), ExtractResource::<R>::new());
                    },
                );
            }
            Some(ExtractedResource::Pipeline) => {
                app.add_system(ExtractPipeline, extract_render_resource::<R>);
                app.observe::<R::Event, _>(
                    |_: &[<R::Event as Event>::Output], events: &SubEvents<RenderApp>| {
                        // events.add_deferred(ExtractPipeline.id(), ExtractResource::<R>::new());
                    },
                );
            }
            None => {
                app.add_system(Extract, extract_render_resource::<R>);
                app.observe::<R::Event, _>(
                    |_: &[<R::Event as Event>::Output], events: &SubEvents<RenderApp>| {
                        // events.add_deferred(Extract.id(), ExtractResource::<R>::new());
                    },
                );
            }
        };

        self
    }

    fn add_render_node<R: RenderGraphNode>(&mut self, node: R) -> &mut Self {
        let builder = self.resource_mut::<RenderGraphBuilder>();
        builder.add_node(node);
        self
    }
}
