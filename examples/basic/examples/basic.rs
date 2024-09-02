use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
    u32,
};

use asset::{
    database::{AssetConfig, AssetDatabase},
    embed_asset,
    importer::{AssetImporter, ImportContext, StringError},
    io::local::{LocalAsset, LocalFileSystem},
    plugin::AssetExt,
    Asset, AssetSettings, Assets, DefaultSettings,
};
use ecs::system::schedule::{Phase, Root};
use game::{
    game::Game,
    phases::{Init, Update},
    plugin::{Plugin, Plugins},
    Execute, GameInstance, PostInit,
};
use graphics::{
    components::{ClearFlag, RenderFrame, RenderFrames},
    core::{Color, VertexAttribute},
    plugin::GraphicsPlugin,
    renderer::{
        graph::RenderGraphBuilder,
        nodes::render::{Attachment, RenderPass, StoreOp},
    },
    resources::{
        mesh::{
            model::{MeshLoadSettings, ObjImporter},
            Mesh,
        },
        texture::{Texture, Texture2d, Texture2dSettings},
    },
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

#[derive(Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlainText(String);

impl Asset for PlainText {}

impl AssetImporter for PlainText {
    type Asset = Self;
    type Settings = DefaultSettings;
    type Error = StringError;

    fn import(
        _: &mut ImportContext<Self::Settings>,
        reader: &mut dyn asset::io::AssetReader,
    ) -> Result<Self::Asset, Self::Error> {
        let content = reader
            .read_to_string()
            .map_err(|e| StringError(e.to_string()))?;
        Ok(PlainText(content))
    }

    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
}

fn main() {
    let mut game = Game::new();
    game.add_plugin(GraphicsPlugin)
        .add_importer::<ObjImporter>()
        .add_importer::<PlainText>()
        .add_system(PostInit, |assets: &Assets<PlainText>| {
            for (id, asset) in assets.iter() {
                println!("Asset: {:?} {:?}", id, asset);
            }
        })
        .add_system(PostInit, |assets: &Assets<Mesh>| {
            for (id, asset) in assets.iter() {
                println!("Asset: {:?} {:?}", id, asset.attributes().len());
            }
        });

    embed_asset!(&mut game, "test.txt");
    embed_asset!(&mut game, "cube.obj");

    game.run();
}
