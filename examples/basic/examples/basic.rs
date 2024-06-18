use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
};

use basic::PlainText;
use shadow_asset::{
    database::{config::AssetConfig, events::ImportAsset},
    plugin::{AssetPlugin, GameAssetExt},
};
use shadow_ecs::ecs::event::Event;
use shadow_game::game::{Game, GameInstance};

fn game_runner(mut game: GameInstance) {
    game.init();
    for _ in 0..10000 {
        game.update();
    }
    game.shutdown();
}

fn main() {
    let config = AssetConfig::new("data");
    Game::new()
        .set_runner(game_runner)
        .add_plugin(AssetPlugin::new(config))
        .register_pipeline::<PlainText>()
        .observe::<ImportAsset<PlainText>, _>(
            |imports: &[<ImportAsset<PlainText> as Event>::Output]| {
                for import in imports {
                    println!("Imported: {:?}", import.path());
                }
            },
        )
        .run();

    // let bytes = std::fs::read("cache/7569663888696199759").unwrap();
    // let pack = AssetPack::<PlainText, ()>::parse(&bytes).unwrap();
    // println!("Pack: {:?}", &pack.asset().text);

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
