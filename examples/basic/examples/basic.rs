use shadow_asset::{
    asset::{Asset, AssetId, Assets, DefaultSettings},
    database::{
        events::{
            AssetEvent, AssetEventExecutor, AssetLoaded, ImportAsset, ImportFolder, LoadAsset,
        },
        AssetConfig, AssetDatabase,
    },
    io::{
        local::LocalFileSystem,
        vfs::{INode, VirtualFileSystem},
        AssetFileSystem, AssetIoError,
    },
    loader::{AssetSerializer, AssetError, AssetLoader, LoadedAssets},
    plugin::{AssetExt, AssetPlugin},
};
use shadow_ecs::world::event::Events;
use shadow_game::{
    game::Game,
    phases::{Init, PostInit, PreInit},
};
use std::path::PathBuf;

fn game_runner(game: &mut Game) {
    game.start();
    loop {
        game.update();
    }
}

pub struct PlainText {
    content: String,
}

impl PlainText {
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

impl Asset for PlainText {}

impl AssetSerializer for PlainText {
    type Asset = Self;
    type Error = AssetIoError;

    fn serialize(asset: &Self::Asset) -> Result<Vec<u8>, Self::Error> {
        Ok(asset.content.as_bytes().to_vec())
    }

    fn deserialize(data: &[u8]) -> Result<Self::Asset, Self::Error> {
        let content = String::from_utf8(data.to_vec())
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;

        Ok(Self::new(content))
    }
}

impl AssetLoader for PlainText {
    type Asset = Self;
    type Settings = DefaultSettings;
    type Error = AssetIoError;
    type Serializer = Self;

    fn load(
        _: &mut shadow_asset::loader::LoadContext<Self::Settings>,
        reader: &mut dyn shadow_asset::io::AssetReader,
    ) -> Result<Self::Asset, Self::Error> {
        reader.read_to_end()?;
        <Self::Serializer as AssetSerializer>::deserialize(&reader.flush()?)
    }

    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
}

fn main() {
    Game::new()
        .add_plugin(AssetPlugin)
        .register_loader::<PlainText>()
        .add_system(PostInit, create_text)
        .observe::<AssetLoaded<PlainText>, _>(on_text_loaded)
        .observe::<AssetError, _>(on_asset_error)
        .set_runner(game_runner)
        .run();
}

fn create_text(database: &AssetDatabase, events: &Events) {
    let config = database.config();
    let mut writer = config.writer(config.asset("text.txt"));
    writer.write("Hello, World!".as_bytes()).unwrap();
    writer.flush().unwrap();

    events.add(ImportAsset::new("text.txt"));
    events.add(LoadAsset::new("text.txt"));
}

fn on_text_loaded(ids: &[AssetId], assets: &Assets<PlainText>) {
    for id in ids {
        match assets.get(id) {
            Some(asset) => println!("Loaded: {}", asset.content),
            None => println!("Failed to load asset"),
        }
    }
}

fn on_asset_error(errors: &[AssetError]) {
    for error in errors {
        println!("Error: {:?}", error);
    }
}
