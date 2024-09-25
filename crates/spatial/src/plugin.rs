use crate::transform::Transform;
use game::Plugin;

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn start(&self, game: &mut game::Game) {
        game.register::<Transform>();
    }
}
