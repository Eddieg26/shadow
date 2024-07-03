use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
};

use shadow_asset::{
    asset::{Asset, DefaultSettings},
    bytes::ToBytes,
    database::pipeline::{AssetError, AssetLoader, AssetPipeline, BasicProcessor, LoadContextType},
    plugin::{AssetExt, AssetPlugin},
};
use shadow_ecs::ecs::{
    core::{Component, Entity},
    event::{Event, Events},
    world::{
        events::{self, AddComponent, RemoveComponent, Spawn},
        query::Query,
        World,
    },
};
use shadow_game::{
    game::{Game, GameInstance},
    schedule::Init,
};

fn game_runner(mut game: GameInstance) {
    game.init();
    loop {
        game.update();
    }
}

pub struct TextFile {
    content: String,
}

impl Asset for TextFile {}

impl AssetLoader for TextFile {
    type Asset = Self;

    type Settings = DefaultSettings;

    fn load(
        ctx: &mut shadow_asset::database::pipeline::LoadContext<Self::Settings>,
    ) -> Result<Self::Asset, shadow_asset::database::pipeline::AssetError> {
        match ctx.ty() {
            LoadContextType::Processed { bytes } => Self::from_bytes(bytes)
                .ok_or(AssetError::Deserialize("Failed to deserialize".to_string())),
            LoadContextType::UnProcessed { bytes, .. } => Self::from_bytes(bytes)
                .ok_or(AssetError::Deserialize("Failed to deserialize".to_string())),
        }
    }

    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
}

impl ToBytes for TextFile {
    fn to_bytes(&self) -> Vec<u8> {
        self.content.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let content = String::from_utf8(bytes.to_vec()).ok()?;
        Some(TextFile { content })
    }
}

impl AssetPipeline for TextFile {
    type Loader = Self;
    type Processor = BasicProcessor<Self, DefaultSettings>;
    type Saver = Self;
}

fn main() {
    Game::new()
        .add_plugin(AssetPlugin::new("data"))
        .set_runner(game_runner)
        .register_pipeline::<TextFile>()
        .run();

    // let value = 0.1 + 0.2;
    // println!("Value: {}", value);
}
