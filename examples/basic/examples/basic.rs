use shadow_ecs::{
    archetype::Archetypes,
    core::{Components, Entities, LocalResources, Resources},
    event::Events,
    system::observer::EventObservers,
    system::schedule::Phase,
    world::{
        events::{
            AddChildren, AddComponents, Despawn, RemoveChildren, RemoveComponents, SetParent, Spawn,
        },
        World,
    },
};
use shadow_game::{
    game::Game,
    phases::{Execute, Init, Update},
};

pub struct AssetInit;

impl Phase for AssetInit {}

fn game_runner(game: &mut Game) {
    game.init();
    loop {
        game.update();
    }
}

fn main() {
    Game::new()
        .add_system(Init, asset_init)
        .add_system(Update, update)
        .set_runner(game_runner)
        .run();
}

fn asset_init() {
    println!("Asset init");
}

fn update() {
    println!("Update");
}
