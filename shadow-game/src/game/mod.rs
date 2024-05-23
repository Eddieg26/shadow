use shadow_ecs::ecs::world::World;

pub mod plugin;
pub mod runner;
pub mod schedule;

pub struct Game {
    world: World,
    plugins: Vec<Box<dyn plugin::Plugin>>,
}
