use shadow_game::game::Game;
use shadow_window::plugin::WindowPlugin;

fn main() {
    Game::new().add_plugin(WindowPlugin).run();
}
