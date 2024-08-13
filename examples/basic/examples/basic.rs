use shadow_game::game::Game;
use shadow_window::{
    plugin::{WindowExt, WindowPlugin},
    window::WindowConfig,
};

fn main() {
    Game::new()
        .add_plugin(WindowPlugin)
        .add_window(WindowConfig::new("Basic Game"))
        .run();
}
