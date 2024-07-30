use shadow_ecs::ecs::{event::Events, world::events::Spawn};
use shadow_game::{
    game::{Game, GameInstance},
    plugin::PhaseExt,
    schedule::{DefaultPhaseRunner, Init, Phase},
};

pub struct AssetInit;

impl Phase for AssetInit {
    type Runner = DefaultPhaseRunner;

    fn runner() -> Self::Runner {
        DefaultPhaseRunner
    }
}

fn game_runner(mut game: GameInstance) {
    game.init();
    loop {
        game.update();
    }
}

fn main() {
    Game::new()
        .add_sub_phase::<Init, AssetInit>()
        .add_system(AssetInit, asset_init)
        .set_runner(game_runner)
        .run();
}

fn asset_init(events: &Events) {
    events.add(Spawn::new())
}
