use shadow_asset::{
    asset::{Asset, DefaultSettings},
    database::loaders::AssetLoaders,
    io::{
        local::LocalFileSystem,
        vfs::{INode, VirtualFileSystem},
        AssetFileSystem, AssetIo, AssetIoError, FileSystem,
    },
    loader::{AssetCacher, AssetLoader, LoadedAssets},
};
use shadow_game::game::Game;
use std::path::PathBuf;

fn game_runner(game: &mut Game) {
    game.init();
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

impl AssetCacher for PlainText {
    type Asset = Self;
    type Settings = DefaultSettings;
    type Error = AssetIoError;

    fn cache(asset: &Self::Asset, _: &Self::Settings) -> Result<Vec<u8>, Self::Error> {
        Ok(asset.content.as_bytes().to_vec())
    }

    fn load(data: &[u8]) -> Result<Self::Asset, Self::Error> {
        let content = String::from_utf8(data.to_vec())
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;

        Ok(Self::new(content))
    }
}

impl AssetLoader for PlainText {
    type Asset = Self;
    type Settings = DefaultSettings;
    type Error = AssetIoError;
    type Cacher = Self;

    fn load(
        _: &mut shadow_asset::loader::LoadContext<Self::Settings>,
        reader: &mut dyn shadow_asset::io::AssetReader,
    ) -> Result<Self::Asset, Self::Error> {
        reader.read_to_end()?;
        <Self::Cacher as AssetCacher>::load(&reader.flush()?)
    }

    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
}

fn main() {
    let io = AssetIo::new("", VirtualFileSystem::new());
    let mut assets = LoadedAssets::new();
    let mut loaders = AssetLoaders::new();
    loaders.add_loader::<PlainText>();

    let text = PlainText::new("Hello, world!".to_string());

    let mut writer = io.writer(io.assets());
    writer.create_dir().unwrap();
    let mut writer = io.writer(io.artifacts());
    writer.create_dir().unwrap();

    let mut writer = io.writer(PathBuf::from("assets/test.txt"));
    writer.write(text.content.as_bytes()).unwrap();
    writer.flush().unwrap();

    let loader = loaders.get::<PlainText>().unwrap();
    let id = match loader.import("assets/test.txt", &loaders, &io, &mut assets) {
        Ok(imported) => {
            let asset = imported.asset::<PlainText>();
            println!("Imported {}", asset.content);
            Some(imported.meta().id())
        }
        Err(e) => {
            println!("Error: {}", e);
            None
        }
    };

    if let Some(id) = id {
        match loader.load(id, &loaders, &io, &mut assets, false) {
            Ok(loaded) => {
                let asset = loaded.cast::<PlainText>();
                println!("Loaded {}", asset.content);
            }
            Err(_) => todo!(),
        }
    }

    println!("{:?}", io.filesystem());
}
