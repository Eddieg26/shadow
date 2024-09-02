use std::{
    hash::{Hash, Hasher},
    u32,
};

use asset::{
    database::AssetConfig,
    importer::{AssetImporter, ImportContext},
    io::local::{LocalAsset, LocalFileSystem},
    AssetSettings,
};
use game::{
    game::Game,
    phases::{Init, Update},
    plugin::{Plugin, Plugins},
};
use graphics::{
    camera::{ClearFlag, RenderFrame, RenderFrames},
    core::{Color, VertexAttribute},
    plugin::GraphicsPlugin,
    renderer::{
        graph::RenderGraphBuilder,
        nodes::render::{Attachment, RenderPass, StoreOp},
    },
    resources::{
        mesh::{
            model::{MeshLoadSettings, ObjLoader},
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

fn main() {
    // Game::new()
    //     .add_plugin(BasicPlugin)
    //     .add_system(Init, || println!("Init"))
    //     .run();

    let config = AssetConfig::new(LocalFileSystem::new(""));
    let settings = AssetSettings::<MeshLoadSettings>::default();
    let mut ctx = ImportContext::new(&config, &settings);
    let mut reader = LocalAsset::new("cube.obj");
    match ObjLoader::import(&mut ctx, &mut reader) {
        Ok(model) => println!("Mesh: {:?}", model.meshes().len()),
        Err(e) => println!("Error: {:?}", e),
    }

    let (_, sub_assets) = ctx.finish();
    for imported in sub_assets {
        let mesh = imported.asset::<Mesh>();
        let positions = mesh.attribute(VertexAttribute::Position).unwrap();
        let normals = mesh.attribute(VertexAttribute::Normal).unwrap();
        let uvs = mesh.attribute(VertexAttribute::TexCoord0).unwrap();
        println!("Positions: {:?}", positions.len());
        println!("Normals: {:?}", normals.len());
        println!("UVs: {:?}", uvs.len());
    }
}
