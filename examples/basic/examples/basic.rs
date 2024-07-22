use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
    time::Instant,
};

use shadow_asset::{
    asset::Asset,
    importer::{AssetImporter, AssetSaver, CustomError},
    DefaultSettings, IntoBytes,
};
use shadow_ecs::ecs::{
    core::{Component, Entity},
    event::{Event, Events},
    system::RunMode,
    world::{
        events::{self, AddComponent, RemoveComponent, Spawn},
        query::Query,
        World,
    },
};
use shadow_game::{
    game::{Game, GameInstance},
    schedule::{Init, PostUpdate, Update},
};

fn game_runner(mut game: GameInstance) {
    game.init();
    loop {
        game.update();
    }
}

pub struct PlainText {
    content: String,
}

impl Asset for PlainText {}

impl IntoBytes for PlainText {
    fn into_bytes(&self) -> Vec<u8> {
        self.content.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(PlainText {
            content: String::from_utf8(bytes.to_vec()).ok()?,
        })
    }
}

impl AssetSaver for PlainText {
    type Asset = PlainText;
    type Settings = DefaultSettings;

    fn save(asset: &Self::Asset, _: &shadow_asset::AssetMetadata<Self::Settings>) -> Vec<u8> {
        asset.into_bytes()
    }

    fn load(bytes: &[u8]) -> Self::Asset {
        PlainText::from_bytes(bytes).unwrap()
    }
}

impl AssetImporter for PlainText {
    type Asset = PlainText;
    type Settings = DefaultSettings;
    type Saver = PlainText;
    type Error = CustomError;

    fn import(
        context: &mut shadow_asset::importer::LoadContext<Self::Settings>,
    ) -> Result<Self::Asset, CustomError> {
        let content = String::from_bytes(context.bytes()).ok_or("Failed to load content")?;
        Ok(PlainText { content })
    }

    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
}

fn main() {
    // Game::new()
    //     .add_plugin(AssetPlugin::new("data"))
    //     .set_runner(game_runner)
    //     .register_importer::<PlainText>()
    //     .run();
}
