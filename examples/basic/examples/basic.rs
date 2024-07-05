use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
};

use shadow_asset::{
    asset::{Asset, DefaultSettings},
    bytes::ToBytes,
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

fn main() {
    Game::new()
        // .add_plugin(AssetPlugin::new("data"))
        // .set_runner(game_runner)
        // .register_pipeline::<TextFile>()
        .run();

    // let value = 0.1 + 0.2;
    // println!("Value: {}", value);
}
