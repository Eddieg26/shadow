use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
    u32,
};

use asset::{
    database::{events::AssetLoaded, AssetConfig, AssetDatabase},
    embed_asset,
    importer::{AssetImporter, ImportContext, StringError},
    io::local::{LocalAsset, LocalFileSystem},
    plugin::AssetExt,
    Asset, AssetActions, AssetId, AssetSettings, Assets, DefaultSettings,
};
use ecs::{
    core::Resource,
    system::schedule::{Phase, Root},
};
use game::{
    game::Game,
    phases::{Init, Update},
    plugin::{Plugin, Plugins},
    Execute, First, GameInstance, PostInit,
};
use graphics::{
    camera::{ClearFlag, RenderFrame, RenderFrames},
    core::Color,
    plugin::{GraphicsExt, GraphicsPlugin, RenderApp},
    renderer::{
        draw::{Draw, DrawCalls},
        graph::{
            context::RenderContext,
            nodes::render::{
                Attachment, RenderCommands, RenderGroup, RenderPass, RenderPassContext, StoreOp,
            },
            RenderGraphBuilder,
        },
    },
    resources::{
        buffer::{BufferFlags, UniformBuffer},
        mesh::{
            model::{MeshLoadSettings, ObjImporter},
            Mesh,
        },
        texture::{Texture, Texture2d, Texture2dSettings},
        RenderAsset, RenderAssetExtractor,
    },
};
use window::{events::WindowCreated, plugin::WindowPlugin};

const TEST_ID: AssetId = AssetId::raw(100);
const CUBE_ID: AssetId = AssetId::raw(200);
const SHADER_ID: AssetId = AssetId::raw(300);

pub struct DrawModel {
    id: AssetId,
    model: ModelData,
}

impl Draw for DrawModel {
    type Partition = Vec<Self>;
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct ModelData([f32; 16]);

impl From<glam::Mat4> for ModelData {
    fn from(value: glam::Mat4) -> Self {
        Self(value.to_cols_array())
    }
}

pub struct DrawModelNode {
    model: UniformBuffer<ModelData>,
}

impl DrawModelNode {
    pub fn new() -> Self {
        Self {
            model: UniformBuffer::new(ModelData::from(glam::Mat4::IDENTITY), BufferFlags::COPY_DST),
        }
    }
}

impl RenderGroup<DrawModel> for DrawModelNode {
    fn render(&mut self, ctx: &RenderPassContext<DrawModel>, commands: &mut RenderCommands) {
        for draw in ctx.draws().iter() {
            self.model.update(draw.model);
            self.model.commit(ctx.device(), ctx.queue());
        }
    }
}

pub struct BasicPlugin;

impl Plugin for BasicPlugin {
    fn dependencies(&self) -> Plugins {
        let mut plugins = Plugins::new();
        plugins.add_plugin(GraphicsPlugin);
        plugins
    }

    fn start(&self, game: &mut Game) {
        let builder = game.resource_mut::<RenderGraphBuilder>();
        let pass = RenderPass::new("basic")
            .with_color(Attachment::Surface, None, StoreOp::Store, None)
            .add_subpass()
            .with_render_group::<DrawModel>(
                0,
                |ctx: &RenderPassContext<DrawModel>, commands: &mut RenderCommands| {},
            );

        builder.add_node(pass);
    }

    fn run(&mut self, game: &mut Game) {
        embed_asset!(game, TEST_ID, "test.txt");
        embed_asset!(game, CUBE_ID, "cube.obj");
        embed_asset!(game, SHADER_ID, "shader.wgsl");

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
    Game::new().add_plugin(BasicPlugin).run();
}
