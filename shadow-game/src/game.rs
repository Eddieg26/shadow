use crate::plugin::{Plugin, PluginContext};

use super::{
    plugin::Plugins,
    scene::{Scene, SceneTracker, Scenes},
    schedule::{Execute, Init, MainSchedule, Phase, Shutdown},
};
use shadow_ecs::ecs::{
    core::{Component, LocalResource, Resource},
    event::Event,
    system::{observer::IntoObserver, IntoSystem},
    world::World,
};

pub struct Game {
    world: World,
    plugins: Plugins,
    schedule: MainSchedule,
    scenes: Scenes,
    runner: Box<dyn GameRunner>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            plugins: Plugins::new(),
            schedule: MainSchedule::new(),
            scenes: Scenes::new(),
            runner: Box::new(default_runner),
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
        self.schedule.add_system(phase, system);
        self
    }

    pub fn add_scene<S: Scene>(&mut self, scene: S) -> &mut Self {
        self.scenes.add(scene);
        self
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        self.plugins.add_plugin(plugin);
        self
    }

    pub fn set_runner<R: GameRunner + 'static>(&mut self, runner: R) -> &mut Self {
        self.runner = Box::new(runner);
        self
    }

    pub fn run(&mut self) {
        let mut plugins = self.plugins.dependencies();
        let mut ctx = PluginContext::new(self);
        plugins.start(&mut ctx);
        plugins.run(&mut ctx);
        plugins.finish(&mut ctx);

        let runner = std::mem::replace(&mut self.runner, Box::new(default_runner));
        let mut game = std::mem::take(self);
        game.add_resource(SceneTracker::new());
        game.schedule.build();
        runner.run(GameInstance::new(game));
    }

    fn init(&mut self) {
        self.update_scene();
        self.schedule.run::<Init>(&mut self.world);
    }

    fn update(&mut self) {
        self.schedule.run::<Execute>(&mut self.world);
        self.update_scene();
    }

    fn shutdown(&mut self) {
        self.schedule.run::<Shutdown>(&mut self.world);
    }

    fn update_scene(&mut self) {
        let tracker = self.world.resource_mut::<SceneTracker>();
        match (tracker.next(), tracker.current()) {
            (Some(next), None) => {
                if let Some(scene) = self.scenes.get(next) {
                    self.schedule.add_systems(next, scene.systems());
                }
                tracker.swap();
            }
            (Some(next), Some(current)) => {
                if let Some(scene) = self.scenes.get(next) {
                    self.schedule.add_systems(next, scene.systems());
                }
                self.schedule.remove_systems(current);
                tracker.swap();
            }
            _ => (),
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GameInstance {
    game: Game,
}

impl GameInstance {
    pub(crate) fn new(game: Game) -> Self {
        Self { game }
    }

    pub fn init(&mut self) {
        self.game.init();
    }

    pub fn update(&mut self) {
        self.game.update();
    }

    pub fn shutdown(&mut self) {
        self.game.shutdown();
    }

    pub fn world(&self) -> &World {
        &self.game.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.game.world
    }

    pub fn flush(&mut self) {
        self.game.world.flush();
    }

    pub fn flush_events<E: Event>(&mut self) {
        self.game.world.flush_events::<E>();
    }
}

pub trait GamePhaseExt: 'static {
    fn add_phase<P: Phase>(&mut self) -> &mut Self;
    fn insert_phase_before<P: Phase, Q: Phase>(&mut self) -> &mut Self;
    fn insert_phase_after<P: Phase, Q: Phase>(&mut self) -> &mut Self;
}

impl GamePhaseExt for Game {
    fn add_phase<P: Phase>(&mut self) -> &mut Self {
        self.schedule.add_phase::<P>();
        self
    }

    fn insert_phase_before<P: Phase, Q: Phase>(&mut self) -> &mut Self {
        self.schedule.insert_before::<P, Q>();
        self
    }

    fn insert_phase_after<P: Phase, Q: Phase>(&mut self) -> &mut Self {
        self.schedule.insert_after::<P, Q>();
        self
    }
}

pub trait GameRunner {
    fn run(&self, game: GameInstance);
}

impl<F: Fn(GameInstance)> GameRunner for F {
    fn run(&self, game: GameInstance) {
        self(game)
    }
}

pub fn default_runner(mut game: GameInstance) {
    game.init();
    game.update();
    game.shutdown();
}
