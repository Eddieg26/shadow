use super::plugin::Plugins;
use crate::{
    phases::{Execute, Init, Shutdown},
    plugin::Plugin,
};
use shadow_ecs::{
    core::{Component, LocalResource, Resource},
    system::{
        observer::IntoObserver,
        schedule::{Phase, PhaseRunner, SystemGroup},
        IntoSystem,
    },
    world::{event::Event, World},
};

pub struct Game {
    world: World,
    plugins: Plugins,
    runner: Option<Box<dyn GameRunner>>,
}

impl Game {
    pub fn new() -> Self {
        let mut world = World::new();
        world.add_phase::<Init>();
        world.add_phase::<Execute>();
        world.add_phase::<Shutdown>();

        Self {
            world,
            plugins: Plugins::new(),
            runner: Some(Box::new(default_runner)),
        }
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.world.resource::<R>()
    }

    pub fn resource_mut<R: Resource>(&mut self) -> &mut R {
        self.world.resource_mut::<R>()
    }

    pub fn local_resource<R: LocalResource>(&self) -> &R {
        self.world.local_resource::<R>()
    }

    pub fn local_resource_mut<R: LocalResource>(&mut self) -> &mut R {
        self.world.local_resource_mut::<R>()
    }

    pub fn try_resource<R: Resource>(&self) -> Option<&R> {
        self.world.try_resource::<R>()
    }

    pub fn try_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.world.try_resource_mut::<R>()
    }

    pub fn try_local_resource<R: LocalResource>(&self) -> Option<&R> {
        self.world.try_local_resource::<R>()
    }

    pub fn try_local_resource_mut<R: LocalResource>(&mut self) -> Option<&mut R> {
        self.world.try_local_resource_mut::<R>()
    }

    pub fn register<C: Component>(&mut self) -> &mut Self {
        self.world.register::<C>();
        self
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.world.register_event::<E>();
        self
    }

    pub fn add_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.add_resource(resource);
        self
    }

    pub fn add_local_resource<R: LocalResource>(&mut self, resource: R) -> &mut Self {
        self.world.add_local_resource(resource);
        self
    }

    pub fn observe<E: Event, M>(&mut self, observer: impl IntoObserver<E, M>) -> &mut Self {
        self.world.observe(observer);
        self
    }

    pub fn add_system<M>(&mut self, phase: impl Phase, system: impl IntoSystem<M>) -> &mut Self {
        self.world.add_system(phase, system);
        self
    }

    pub fn add_phase<P: Phase>(&mut self) -> &mut Self {
        self.world.add_phase::<P>();
        self
    }

    pub fn add_sub_phase<P: Phase, Q: Phase>(&mut self) -> &mut Self {
        self.world.add_sub_phase::<P, Q>();
        self
    }

    pub fn insert_phase_before<P: Phase, Q: Phase>(&mut self) -> &mut Self {
        self.world.insert_phase_before::<P, Q>();
        self
    }

    pub fn insert_phase_after<P: Phase, Q: Phase>(&mut self) -> &mut Self {
        self.world.insert_phase_after::<P, Q>();
        self
    }

    pub fn add_phase_runner<P: Phase>(&mut self, runner: impl PhaseRunner) -> &mut Self {
        self.world.add_phase_runner::<P>(runner);
        self
    }

    pub fn add_system_group<G: SystemGroup>(&mut self) -> &mut Self {
        self.world.add_system_group::<G>();
        self
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        self.plugins.add_plugin(plugin);
        self
    }

    pub fn set_runner<R: GameRunner + 'static>(&mut self, runner: R) -> &mut Self {
        self.runner = Some(Box::new(runner));
        self
    }

    pub fn run(&mut self) {
        let mut plugins = self.plugins.dependencies();
        plugins.start(self);
        plugins.run(self);
        plugins.finish(self);

        self.world.build();

        let runner = self.runner.take().unwrap_or(Box::new(default_runner));
        runner.run(self);
    }

    pub fn init(&mut self) {
        self.world.run(Init);
    }

    pub fn update(&mut self) {
        self.world.run(Execute);
    }

    pub fn shutdown(&mut self) {
        self.world.run(Shutdown);
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

pub trait GameRunner {
    fn run(&self, game: &mut Game);
}

impl<F: Fn(&mut Game)> GameRunner for F {
    fn run(&self, game: &mut Game) {
        self(game)
    }
}

pub fn default_runner(game: &mut Game) {
    game.init();
    game.update();
    game.shutdown();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Release,
}
