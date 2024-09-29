use crate::{transform::Transform, update_transforms};
use game::{Plugin, PostUpdate};

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn start(&self, game: &mut game::Game) {
        game.register::<Transform>();
        game.add_system(PostUpdate, update_transforms);
    }
}
