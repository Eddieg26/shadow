use std::any::TypeId;

use super::{runner::GameRunner, scene::Scene, schedule::Phase, Game, GamePhaseExt};
use shadow_ecs::ecs::{
    core::{Component, LocalResource, Resource},
    event::Event,
    storage::dense::DenseMap,
    system::{observer::IntoObserver, IntoSystem},
};

pub struct PluginContext<'a> {
    game: &'a mut Game,
}

impl<'a> PluginContext<'a> {
    pub fn new(game: &'a mut Game) -> Self {
        Self { game }
    }

    pub fn register<C: Component>(&mut self) -> &mut Self {
        self.game.register::<C>();
        self
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.game.register_event::<E>();
        self
    }

    pub fn add_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.game.add_resource(resource);
        self
    }

    pub fn add_local_resource<R: LocalResource>(&mut self, resource: R) -> &mut Self {
        self.game.add_local_resource(resource);
        self
    }

    pub fn observe<E: Event, M>(&mut self, observer: impl IntoObserver<E, M>) -> &mut Self {
        self.game.observe(observer);
        self
    }

    pub fn add_system<M>(&mut self, phase: impl Phase, system: impl IntoSystem<M>) -> &mut Self {
        self.game.add_system(phase, system);
        self
    }

    pub fn add_scene<S: Scene>(&mut self, scene: S) -> &mut Self {
        self.game.add_scene(scene);
        self
    }

    pub fn set_runner<R: GameRunner + 'static>(&mut self, runner: R) -> &mut Self {
        self.game.set_runner(runner);
        self
    }
}

pub trait PluginPhaseExt {
    fn add_phase<P: Phase>(&mut self) -> &mut Self;
    fn insert_phase_before<P: Phase, Q: Phase>(&mut self) -> &mut Self;
    fn insert_phase_after<P: Phase, Q: Phase>(&mut self) -> &mut Self;
}

impl<'a> PluginPhaseExt for PluginContext<'a> {
    fn add_phase<P: Phase>(&mut self) -> &mut Self {
        self.game.add_phase::<P>();
        self
    }

    fn insert_phase_before<P: Phase, Q: Phase>(&mut self) -> &mut Self {
        self.game.insert_phase_before::<P, Q>();
        self
    }

    fn insert_phase_after<P: Phase, Q: Phase>(&mut self) -> &mut Self {
        self.game.insert_phase_after::<P, Q>();
        self
    }
}

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

    pub fn append(&mut self, mut plugins: Plugins) -> &mut Self {
        for (type_id, plugin) in plugins.plugins.drain() {
            if !self.plugins.contains(&type_id) {
                self.plugins.insert(type_id, plugin);
            }
        }
        self
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

    pub(crate) fn start(&mut self, ctx: &mut PluginContext) {
        for plugin in self.plugins.values_mut() {
            plugin.start(ctx);
        }
    }

    pub(crate) fn run(&mut self, ctx: &mut PluginContext) {
        for plugin in self.plugins.values_mut() {
            plugin.run(ctx);
        }
    }

    pub(crate) fn finish(&mut self, ctx: &mut PluginContext) {
        for plugin in self.plugins.values_mut() {
            plugin.finish(ctx);
        }
    }
}

pub trait Plugin: 'static {
    fn dependencies(&self) -> Plugins {
        Plugins::new()
    }
    fn start(&mut self, _ctx: &mut PluginContext) {}
    fn run(&mut self, ctx: &mut PluginContext);
    fn finish(&mut self, _ctx: &mut PluginContext) {}
}
