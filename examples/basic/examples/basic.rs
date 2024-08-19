use std::hash::{Hash, Hasher};

use shadow_game::{
    game::Game,
    phases::{Init, Update},
    plugin::{Plugin, Plugins},
};
use shadow_graphics::{
    core::Color,
    plugin::GraphicsPlugin,
    renderer::{
        draw::{DrawCalls, Render, RenderCalls},
        graph::RenderGraphBuilder,
        nodes::render::{Attachment, RenderCommands, RenderPass, StoreOp, Subpass},
    },
};
use shadow_window::{events::WindowCreated, plugin::WindowPlugin};

pub struct ClearScreen {
    color: Color,
}

impl ClearScreen {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

impl Render for ClearScreen {
    fn texture(&self) -> Option<shadow_graphics::resources::ResourceId> {
        None
    }

    fn clear_color(&self) -> Option<Color> {
        Some(self.color)
    }
}

pub struct BasicPlugin;

impl Plugin for BasicPlugin {
    fn dependencies(&self) -> shadow_game::plugin::Plugins {
        let mut plugins = Plugins::new();
        plugins.add_plugin(GraphicsPlugin);
        plugins
    }

    fn start(&self, game: &mut Game) {
        let builder = game.resource_mut::<RenderGraphBuilder>();
        let pass = RenderPass::new().with_color(Attachment::Surface, None, StoreOp::Store, None);

        builder.add_node("basic", pass);
    }

    fn run(&mut self, game: &mut Game) {
        game.add_system(Update, |renders: &mut RenderCalls| {
            renders.add(ClearScreen::new(Color::blue()))
        });
    }

    fn finish(&mut self, _: &mut Game) {}
}

fn main() {
    Game::new()
        .add_plugin(BasicPlugin)
        .add_system(Init, || println!("Init"))
        .run();
}
