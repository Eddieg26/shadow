use shadow_ecs::{
    archetype::Archetypes,
    core::{Component, Components, Entities, LocalResources, Resources},
    event::{Event, Events},
    system::{observer::EventObservers, schedule::Phase},
    world::{
        events::{
            AddChildren, AddComponent, AddComponents, Despawn, RemoveChildren, RemoveComponent,
            RemoveComponents, SetParent, Spawn,
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
        .register::<Player>()
        .add_system(Init, spawn_player)
        .observe::<Spawn, _>(on_spawn_player)
        .observe::<AddComponent<Player>, _>(on_add_player_components)
        .observe::<Despawn, _>(on_despawn_player)
        .observe::<RemoveComponent<Player>, _>(on_remove_player_components)
        .set_runner(game_runner)
        .run();
}

struct Player;
impl Component for Player {}

fn spawn_player(events: &Events) {
    events.add(Spawn::new().with(Player));
}

fn on_spawn_player(entities: &[<Spawn as Event>::Output]) {
    for entity in entities {
        println!("Player spawned: {:?}", entity);
    }
}

fn on_add_player_components(entities: &[<AddComponent<Player> as Event>::Output], events: &Events) {
    for entity in entities {
        println!("Player added: {:?}", entity);
        events.add(Despawn::new(*entity));
    }
}

fn on_despawn_player(entities: &[<Despawn as Event>::Output]) {
    for entity in entities {
        println!("Player despawned: {:?}", entity);
    }
}

fn on_remove_player_components(entities: &[<RemoveComponent<Player> as Event>::Output]) {
    for removed in entities {
        println!("Player removed: {:?}", removed.entity);
    }
}
