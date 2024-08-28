use crate::{
    core::{
        device::{RenderDevice, RenderInstance, RenderQueue},
        surface::RenderSurface,
    },
    renderer::{
        draw::{Draw, DrawCalls, RenderCalls},
        graph::{resources::RenderTargetDesc, RenderGraph, RenderGraphBuilder},
    },
};
use ecs::world::{
    event::{Event, Events},
    World,
};
use game::{
    app::{Extract, MainWorld},
    game::Game,
    phases::{PostUpdate, Update},
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
        plugins.add_plugin(WindowPlugin);
        plugins
    }

    fn start(&self, game: &mut Game) {
        // game.add_sub_app::<RenderApp>();
        game.add_draw_calls::<()>(|| vec![]);
        game.add_resource(RenderCalls::new());
        game.add_resource(RenderGraphBuilder::new());
        game.register_event::<SurfaceCreated>();
        // game.observe::<Resized, _>(|resized: &[Resized], events: &SubEvents<RenderApp>| {
        //     resized.last().map(|resized| events.add(*resized));
        // });

        // let app = game.sub_app_mut::<RenderApp>().unwrap();
        game.add_system(Extract, extract_render_calls);
        game.add_system(Update, update_render_graph);
        game.add_system(PostUpdate, clear_render_calls);
        game.add_resource(RenderCalls::new());
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

        // let app = game.sub_app_mut::<RenderApp>().unwrap();
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

fn extract_render_calls(main_world: &MainWorld, calls: &mut RenderCalls) {
    let main = main_world.resource_mut::<RenderCalls>();
    calls.extract(main);
}

fn clear_render_calls(calls: &mut RenderCalls) {
    calls.clear();
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

// pub struct RenderApp;

// impl SubApp for RenderApp {}

pub trait GraphicsExt {
    fn add_draw_calls<D: Draw>(&mut self, partition: impl Fn() -> D::Partition);
}

impl GraphicsExt for Game {
    fn add_draw_calls<D: Draw>(&mut self, partition: impl Fn() -> D::Partition) {
        self.add_resource(DrawCalls::<D>::new(partition()));

        // let app = match self.sub_app_mut::<RenderApp>() {
        //     Some(app) => app,
        //     None => {
        //         self.add_sub_app::<RenderApp>();
        //         self.sub_app_mut::<RenderApp>().unwrap()
        //     }
        // };

        self.add_resource(DrawCalls::<D>::new(partition()));
        self.add_system(Extract, extract_draw_calls::<D>);
        self.add_system(PostUpdate, clear_draw_calls::<D>);
    }
}
