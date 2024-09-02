use super::plugin::Plugins;
use crate::{
    app::{SubApp, SubApps, SubEvents, SubWorlds},
    phases::{Execute, Shutdown, Startup},
    plugin::Plugin,
};
use ecs::{
    core::{Component, LocalResource, Resource},
    system::{
        observer::IntoObserver,
        schedule::{Phase, PhaseRunner, SystemGroup},
        IntoSystem,
    },
    world::{
        event::{Event, Events},
        World,
    },
};

pub struct Game {
    world: World,
    sub_apps: SubWorlds,
    plugins: Plugins,
    runner: Option<Box<dyn GameRunner>>,
}

impl Game {
    pub fn new() -> Self {
        let mut world = World::new();
        world.add_phase::<Startup>();
        world.add_phase::<Execute>();
        world.add_phase::<Shutdown>();

        Self {
            world,
            sub_apps: SubWorlds::new(),
            plugins: Plugins::new(),
            runner: Some(Box::new(default_runner)),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
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

    pub fn has_resource<R: Resource>(&self) -> bool {
        self.world.has_resource::<R>()
    }

    pub fn has_local_resource<R: LocalResource>(&self) -> bool {
        self.world.has_local_resource::<R>()
    }

    pub fn sub_app<S: SubApp>(&mut self) -> Option<&World> {
        self.sub_apps.get::<S>()
    }

    pub fn sub_app_mut<S: SubApp>(&mut self) -> Option<&mut World> {
        self.sub_apps.get_mut::<S>()
    }

    pub fn register<C: Component>(&mut self) -> &mut Self {
        self.world.register::<C>();
        self
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.world.register_event::<E>();
        self
    }

    pub fn init_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.world.add_resource(R::default());
        self
    }

    pub fn add_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.add_resource(resource);
        self
    }

    pub fn init_local_resource<R: LocalResource + Default>(&mut self) -> &mut Self {
        self.world.add_local_resource(R::default());
        self
    }

    pub fn add_local_resource<R: LocalResource>(&mut self, resource: R) -> &mut Self {
        self.world.add_local_resource(resource);
        self
    }

    pub fn add_sub_app<S: SubApp>(&mut self) -> &mut Self {
        let events = {
            let app = self.sub_apps.add::<S>(self.world.sub());
            SubEvents::<S>::new(app.events().clone())
        };

        self.add_resource(events);
        self
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.world.remove_resource::<R>()
    }

    pub fn remove_local_resource<R: LocalResource>(&mut self) -> Option<R> {
        self.world.remove_local_resource::<R>()
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

    pub fn add_sub_phase<Main: Phase, Sub: Phase>(&mut self) -> &mut Self {
        self.world.add_sub_phase::<Main, Sub>();
        self
    }

    pub fn insert_phase_before<Main: Phase, Before: Phase>(&mut self) -> &mut Self {
        self.world.insert_phase_before::<Main, Before>();
        self
    }

    pub fn insert_phase_after<Main: Phase, After: Phase>(&mut self) -> &mut Self {
        self.world.insert_phase_after::<Main, After>();
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
        let ty = std::any::TypeId::of::<P>();
        if !self.plugins.contains(&ty) {
            let mut plugins = plugin.dependencies().dependencies();
            for (id, plugin) in plugins.drain() {
                if !self.plugins.contains(&id) {
                    plugin.start(self);
                    self.plugins.add_boxed_plugin(id, plugin);
                }
            }

            plugin.start(self);
            self.plugins.add_boxed_plugin(ty, Box::new(plugin));
        }

        self
    }

    pub fn set_runner<R: GameRunner + 'static>(&mut self, runner: R) -> &mut Self {
        self.runner = Some(Box::new(runner));
        self
    }

    pub fn events(&self) -> &Events {
        self.world.events()
    }

    pub fn run(&mut self) {
        let mut plugins = self.plugins.dependencies();
        plugins.run(self);
        plugins.finish(self);

        self.world.build();

        let runner = self.runner.take().unwrap_or(Box::new(default_runner));
        let world = std::mem::take(&mut self.world);
        let sub_apps = std::mem::take(&mut self.sub_apps).into_apps(&world);
        let mut game = GameInstance::new(world, sub_apps);
        runner.run(&mut game);
    }

    pub fn start(&mut self) {
        self.world.run(Startup);
    }

    pub fn update(&mut self) {
        self.world.run(Execute);
    }

    pub fn shutdown(&mut self) {
        self.world.run(Shutdown);
    }

    pub fn flush(&mut self) {
        self.world.flush();
    }

    pub fn flush_events<E: Event>(&mut self) {
        self.world.flush_events::<E>();
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GameInstance {
    world: Option<World>,
    sub_apps: SubApps,
}

impl GameInstance {
    pub fn new(world: World, sub_apps: SubApps) -> Self {
        Self {
            world: Some(world),
            sub_apps,
        }
    }

    pub fn start(&mut self) {
        self.world_mut().run(Startup);
    }

    pub fn update(&mut self) {
        if let Some(mut world) = self.world.take() {
            world.run(Execute);
            self.world = Some(self.sub_apps.update(world));
        }
    }

    pub fn shutdown(&mut self) {
        self.world_mut().run(Shutdown);
    }

    pub fn world(&self) -> &World {
        self.world.as_ref().expect("World not found")
    }

    pub fn world_mut(&mut self) -> &mut World {
        self.world.as_mut().expect("World not found")
    }
}

pub trait GameRunner {
    fn run(&self, game: &mut GameInstance);
}

impl<F: Fn(&mut GameInstance)> GameRunner for F {
    fn run(&self, game: &mut GameInstance) {
        self(game)
    }
}

pub fn default_runner(game: &mut GameInstance) {
    game.start();
    game.update();
    game.shutdown();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Release,
}
