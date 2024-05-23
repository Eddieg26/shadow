use super::Game;

pub struct Plugins {
    plugins: Vec<Box<dyn Plugin>>,
}

impl Plugins {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        self.plugins.push(Box::new(plugin));
        self
    }
}

pub trait Plugin: 'static {
    fn dependencies(&self) -> Plugins {
        Plugins::new()
    }
    fn start(&mut self, game: &mut Game);
    fn run(&mut self, game: &mut Game);
    fn stop(&mut self, game: &mut Game);
}
