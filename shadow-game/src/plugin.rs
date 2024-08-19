use crate::game::Game;
use shadow_ecs::core::DenseMap;
use std::any::TypeId;

pub struct Plugins {
    plugins: DenseMap<TypeId, Box<dyn Plugin>>,
}

impl Plugins {
    pub fn new() -> Self {
        Self {
            plugins: DenseMap::new(),
        }
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        self.plugins.insert(TypeId::of::<P>(), Box::new(plugin));
        self
    }

    pub fn add_boxed_plugin(&mut self, type_id: TypeId, plugin: Box<dyn Plugin>) -> &mut Self {
        self.plugins.insert(type_id, plugin);
        self
    }

    pub fn append(&mut self, mut plugins: Plugins) -> &mut Self {
        for (type_id, plugin) in plugins.plugins.drain() {
            if !self.plugins.contains(&type_id) {
                self.plugins.insert(type_id, plugin);
            }
        }
        self
    }

    pub fn contains(&self, type_id: &TypeId) -> bool {
        self.plugins.contains(type_id)
    }

    pub(crate) fn drain(&mut self) -> impl Iterator<Item = (TypeId, Box<dyn Plugin>)> + '_ {
        self.plugins.drain()
    }

    pub(crate) fn dependencies(&mut self) -> Plugins {
        let mut plugins = Plugins::new();
        for (type_id, plugin) in self.plugins.drain() {
            let mut dependencies = plugin.dependencies();
            plugins.append(dependencies.dependencies());
            plugins.plugins.insert(type_id, plugin);
        }
        plugins
    }

    pub(crate) fn run(&mut self, game: &mut Game) {
        for plugin in self.plugins.values_mut() {
            plugin.run(game);
        }
    }

    pub(crate) fn finish(&mut self, game: &mut Game) {
        for plugin in self.plugins.values_mut() {
            plugin.finish(game);
        }
    }
}

pub trait Plugin: 'static {
    fn dependencies(&self) -> Plugins {
        Plugins::new()
    }
    fn start(&self, _: &mut Game) {}
    fn run(&mut self, _: &mut Game) {}
    fn finish(&mut self, _: &mut Game) {}
}
