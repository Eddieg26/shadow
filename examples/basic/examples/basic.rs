use basic::PlainText;
use shadow_asset::{
    asset::{AssetId, AssetMetadata, AssetSettings, Assets},
    bytes::AsBytes,
    plugin::{
        events::{AssetLoaded, ImportAsset, LoadAsset},
        AssetPlugin, AssetPluginExt,
    },
};
use shadow_ecs::ecs::event::Events;
use shadow_game::{game::Game, schedule::PreUpdate};
use std::path::PathBuf;

fn main() {
    Game::new()
        .add_plugin(AssetPlugin)
        .register_loader::<PlainText>()
        .add_system(PreUpdate, |events: &Events| {
            events.add(LoadAsset::<PlainText>::new("test.txt"));
        })
        .observe::<ImportAsset<PlainText>, _>(|path: &[PathBuf]| {
            println!("Observe Import PlainText {:?}", path);
        })
        .observe::<AssetLoaded<PlainText>, _>(|ids: &[AssetId], assets: &Assets<PlainText>| {
            for id in ids {
                let asset = assets.get_unchecked(id);
                println!("Text {:?}", &asset.text);
            }
        })
        .run();

    // let meta = AssetMetadata::<()>::default();
    // println!("Id: {:?}", meta.id());
    // let bytes = meta.as_bytes();
    // println!("Bytes: {:?}", &bytes);
    // std::fs::write("assets/test.meta", &bytes).unwrap();

    // let bytes = std::fs::read("assets/test.meta").unwrap();
    // println!("Bytes: {:?}", &bytes);
    // let meta = AssetMetadata::<()>::from_bytes(&bytes).unwrap();
    // println!("Id: {:?}", meta.id());
}
