use std::{
    hash::{Hash, Hasher},
    u32,
};

use asset::{
    io::local::LocalAsset,
    loader::{AssetLoader, LoadContext},
    AssetSettings,
};
use game::{
    game::Game,
    phases::{Init, Update},
    plugin::{Plugin, Plugins},
};
use graphics::{
    camera::{ClearFlag, RenderFrame, RenderFrames},
    core::Color,
    plugin::GraphicsPlugin,
    renderer::{
        graph::RenderGraphBuilder,
        nodes::render::{Attachment, RenderPass, StoreOp},
    },
    resources::texture::{Texture, Texture2d, Texture2dSettings},
};
use window::{events::WindowCreated, plugin::WindowPlugin};

pub struct BasicPlugin;

impl Plugin for BasicPlugin {
    fn dependencies(&self) -> Plugins {
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
        game.add_system(Update, |renders: &mut RenderFrames| {
            let mut frame = RenderFrame::default();
            frame.camera.clear = Some(ClearFlag::Color(Color::green()));
            renders.add(frame)
        });
    }

    fn finish(&mut self, _: &mut Game) {}
}

fn main() {
    // Game::new()
    //     .add_plugin(BasicPlugin)
    //     .add_system(Init, || println!("Init"))
    //     .run();
}
