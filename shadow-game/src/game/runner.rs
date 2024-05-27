use super::GameInstance;

pub trait GameRunner {
    fn run(&self, game: GameInstance);
}

impl<F: Fn(GameInstance)> GameRunner for F {
    fn run(&self, game: GameInstance) {
        self(game)
    }
}

pub fn default_runner(mut game: GameInstance) {
    game.init();
    game.update();
    game.shutdown();
}
