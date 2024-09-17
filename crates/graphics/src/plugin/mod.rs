use crate::{
    camera::{Camera, CameraBuffer, RenderFrames},
    core::device::{RenderDevice, RenderInstance, RenderQueue},
    renderer::{
        draw::{Draw, DrawCalls},
        graph::{
            resources::{RenderGraphTexture, RenderTarget},
            RenderGraph, RenderGraphBuilder,
        },
        surface::{RenderSurface, RenderSurfaceTexture},
    },
    resources::{
        mesh::{model::ObjImporter, Mesh, MeshBuffers},
        shader::{Shader, ShaderSource},
        texture::{render::RenderTexture, GpuTexture, Texture2d},
        AssetUsage, ExtractedResource, RenderAsset, RenderAssetExtractor, RenderAssets,
    },
};
use asset::{
    plugin::{AssetExt, AssetPlugin},
    AssetAction, AssetActions, Assets,
};
use ecs::{
    system::{schedule::Phase, SystemArg},
    world::{event::Event, World},
};
use game::{
    app::{Extract, MainWorld},
    game::Game,
    phases::Update,
    plugin::{Plugin, Plugins},
    SubApp, SubEvents,
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
        game.add_sub_app::<RenderApp>();
        game.init_resource::<RenderFrames>();
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
        game.register::<Camera>();

        let app = game.sub_app_mut::<RenderApp>().unwrap();
        app.init_resource::<RenderFrames>();
        app.init_resource::<RenderSurfaceTexture>();
        app.add_sub_phase::<Extract, ExtractBindGroup>();
        app.add_sub_phase::<Extract, ExtractPipeline>();
        app.add_sub_phase::<Update, PreRender>();
        app.add_sub_phase::<Update, Render>();
        app.add_sub_phase::<Update, Present>();
        app.add_sub_phase::<Update, PostRender>();
    }

    fn run(&mut self, game: &mut Game) {
        game.observe::<WindowCreated, _>(on_window_created);
        game.observe::<Resized, _>(|resized: &[Resized], events: &SubEvents<RenderApp>| {
            let resized = resized.last().unwrap();
            events.add(*resized);
        });

        let app = game.sub_app_mut::<RenderApp>().unwrap();
        app.init_resource::<CameraBuffer>();
        app.register_event::<SurfaceCreated>();
        app.observe::<SurfaceCreated, _>(SurfaceCreated::observer);
        app.observe::<Resized, _>(on_window_resized);
        app.add_system(Extract, extract_render_frames);
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

fn update_render_graph(
    world: &World,
    graph: &mut RenderGraph,
    targets: &mut RenderAssets<RenderTarget>,
    camera: &mut CameraBuffer,
) {
    let surface = world.resource::<RenderSurface>();
    let texture = match surface.texture() {
        Ok(surface) => surface,
        Err(_) => return,
    };

    if let Some(target) = targets.get_mut(&surface.id()) {
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        target.set_color(Some(RenderGraphTexture::from(view)));
    } else {
        return;
    }

    let surface_target = unsafe { targets.get(&surface.id()).unwrap_unchecked() };

    let device = world.resource::<RenderDevice>();
    let queue = world.resource::<RenderQueue>();
    for frame in world.resource::<RenderFrames>() {
        let target = match frame.camera.target.and_then(|id| targets.get(&id.into())) {
            Some(target) => target,
            None => surface_target,
        };

        camera.update(frame.buffer);
        camera.commit(device, queue);

        graph.run(world, device, queue, frame, target);
    }

    let target = unsafe { targets.get_mut(&surface.id()).unwrap_unchecked() };
    target.set_color(None);

    world.resource_mut::<RenderSurfaceTexture>().set(texture);
}

#[inline]
fn present_surface(texture: &mut RenderSurfaceTexture) {
    texture.present();
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
                let render_id = <R::Target as RenderAsset>::Id::from(*id);
                if let Some(asset) = R::extract(source, &arg) {
                    assets.add((render_id).into(), asset);
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

                let render_id = <R::Target as RenderAsset>::Id::from(*id);
                let asset = match assets.get_mut(&render_id) {
                    Some(asset) => asset,
                    None => continue,
                };

                let arg = <R::Arg<'static> as SystemArg>::get(world);
                R::update(source, asset, &arg);
            }
            AssetAction::Removed(id) => {
                let render_id = <R::Target as RenderAsset>::Id::from(*id);
                assets.remove(&render_id);
            }
            _ => (),
        }
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

fn on_window_resized(resized: &[Resized], device: &RenderDevice, surface: &mut RenderSurface) {
    let resized = resized.last().unwrap();

    surface.resize(device, resized.width, resized.height);
}

pub trait GraphicsExt {
    fn add_draw_calls<D: Draw>(&mut self, partition: impl Fn() -> D::Partition) -> &mut Self;
    fn add_render_asset<R: RenderAsset>(&mut self) -> &mut Self;
    fn add_render_asset_extractor<R: RenderAssetExtractor>(&mut self) -> &mut Self;
}

impl GraphicsExt for Game {
    fn add_draw_calls<D: Draw>(&mut self, partition: impl Fn() -> D::Partition) -> &mut Self {
        self.add_resource(DrawCalls::<D>::new(partition()));

        let app = self.sub_app_mut::<RenderApp>().unwrap();
        app.add_resource(DrawCalls::<D>::new(partition()));
        app.add_system(Extract, extract_draw_calls::<D>);
        app.add_system(PostRender, clear_draw_calls::<D>);

        self
    }

    fn add_render_asset<R: RenderAsset>(&mut self) -> &mut Self {
        let app = self.sub_app_mut::<RenderApp>().unwrap();
        if !app.has_resource::<RenderAssets<R>>() {
            app.init_resource::<RenderAssets<R>>();
        }

        self
    }

    fn add_render_asset_extractor<R: RenderAssetExtractor>(&mut self) -> &mut Self {
        self.add_render_asset::<R::Target>();
        self.register_asset::<R::Source>();

        let app = self.sub_app_mut::<RenderApp>().unwrap();
        match R::extracted_resource() {
            Some(ExtractedResource::BindGroup) => {
                app.add_system(ExtractBindGroup, extract_render_asset::<R>)
            }
            Some(ExtractedResource::Pipeline) => {
                app.add_system(ExtractPipeline, extract_render_asset::<R>)
            }
            None => app.add_system(Extract, extract_render_asset::<R>),
        };

        self
    }
}
