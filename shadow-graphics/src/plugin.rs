use shadow_ecs::world::{
    event::{Event, Events},
    World,
};
use shadow_game::{
    game::Game,
    plugin::{Plugin, Plugins},
};
use shadow_window::{events::WindowCreated, plugin::WindowPlugin, window::Window};

use crate::core::{
    device::{RenderDevice, RenderInstance, RenderQueue},
    surface::RenderSurface,
};

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn dependencies(&self) -> shadow_game::plugin::Plugins {
        let mut plugins = Plugins::new();
        plugins.add_plugin(WindowPlugin);
        plugins
    }

    fn run(&mut self, _: &mut Game) {}
}

fn on_window_created(_: &[WindowCreated], window: &Window, events: &Events) {
    let create = || async {
        let instance = RenderInstance::create();

        let surface = match RenderSurface::create(&instance, window).await {
            Ok(surface) => surface,
            Err(_) => todo!(),
        };

        let (device, queue) = match RenderDevice::create(surface.adapter()).await {
            Ok(res) => res,
            Err(_) => todo!(),
        };

        events.add(SurfaceCreated::new(surface, device, queue));
    };

    pollster::block_on(create());
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
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.add_resource(self.surface);
        world.add_resource(self.device);
        world.add_resource(self.queue);

        Some(())
    }
}
