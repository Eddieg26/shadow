use crate::phases::{PostUpdate, PreUpdate, Update};
use ecs::{
    core::{DenseMap, Resource},
    system::schedule::{Phase, Schedule},
    world::{event::Events, World},
};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

pub struct Extract;

impl Phase for Extract {}

pub struct SubWorldUpdate;

impl Phase for SubWorldUpdate {
    fn schedule() -> Schedule {
        let mut schedule = Schedule::from::<Self>();
        schedule.add_schedule(Schedule::from::<PreUpdate>());
        schedule.add_schedule(Schedule::from::<Update>());
        schedule.add_schedule(Schedule::from::<PostUpdate>());

        schedule
    }
}

pub struct MainWorld(World);

impl From<World> for MainWorld {
    fn from(world: World) -> Self {
        Self(world)
    }
}

impl Into<World> for MainWorld {
    fn into(self) -> World {
        self.0
    }
}

impl std::ops::Deref for MainWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Resource for MainWorld {}

pub struct MainEvents(Events);

impl From<Events> for MainEvents {
    fn from(events: Events) -> Self {
        Self(events)
    }
}

impl std::ops::Deref for MainEvents {
    type Target = Events;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resource for MainEvents {}

pub trait SubApp: 'static {}

#[derive(Default)]
pub struct SubWorlds {
    worlds: DenseMap<TypeId, World>,
}

impl SubWorlds {
    pub fn new() -> Self {
        Self {
            worlds: DenseMap::new(),
        }
    }

    pub fn add<A: SubApp>(&mut self, world: World) -> &mut World {
        let ty = TypeId::of::<A>();
        if !self.worlds.contains(&ty) {
            self.worlds.insert(ty, world);
        }
        self.worlds.get_mut(&ty).unwrap()
    }

    pub fn get<A: SubApp>(&self) -> Option<&World> {
        self.worlds.get(&TypeId::of::<A>())
    }

    pub fn get_mut<A: SubApp>(&mut self) -> Option<&mut World> {
        self.worlds.get_mut(&TypeId::of::<A>())
    }

    pub fn into_apps(self, main_world: &World) -> SubApps {
        let mut apps = SubApps::new();
        for (id, mut world) in self.worlds {
            Self::init_sub_worlds(main_world, &mut world);

            let main_events = MainEvents::from(main_world.events().clone());
            world.add_resource(main_events);
            apps.apps.insert(id, SubAppWorld::new(world));
        }
        apps
    }

    fn init_sub_worlds(main: &World, sub: &mut World) {
        let metas = main.events().metas();
        for (ty, meta) in metas.iter() {
            meta.add_outputs(sub);
            sub.events().metas_mut().insert(*ty, *meta);
        }
    }
}

pub struct SubAppWorld {
    world: Arc<RwLock<World>>,
}

impl SubAppWorld {
    pub fn new(mut world: World) -> Self {
        world.add_phase::<Extract>();
        world.add_phase::<SubWorldUpdate>();
        Self {
            world: Arc::new(RwLock::new(world)),
        }
    }

    pub fn update(&mut self, main_world: World) -> World {
        let main_world = MainWorld::from(main_world);
        let tasks = main_world.tasks().clone();

        let main_world = {
            let mut world = self.world.write().unwrap();
            world.add_resource(main_world);
            world.run(Extract);
            world
                .remove_resource::<MainWorld>()
                .expect("Main world not found")
        };

        let world = self.world.clone();
        tasks.spawn(move || {
            let mut world = world.write().unwrap();
            world.run(SubWorldUpdate);
        });

        main_world.into()
    }
}

pub struct SubApps {
    apps: DenseMap<TypeId, SubAppWorld>,
}

impl SubApps {
    pub fn new() -> Self {
        Self {
            apps: DenseMap::new(),
        }
    }

    pub fn get<S: SubApp>(&self) -> Option<&SubAppWorld> {
        self.apps.get(&TypeId::of::<S>())
    }

    pub fn get_mut<S: SubApp>(&mut self) -> Option<&mut SubAppWorld> {
        self.apps.get_mut(&TypeId::of::<S>())
    }

    pub fn update(&mut self, main_world: World) -> World {
        let mut main_world = main_world;
        for app in self.apps.values_mut() {
            main_world = app.update(main_world);
        }
        main_world
    }
}

pub struct SubEvents<S: SubApp> {
    events: Events,
    _marker: std::marker::PhantomData<S>,
}

impl<S: SubApp> SubEvents<S> {
    pub fn new(events: Events) -> Self {
        Self {
            events,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S: SubApp> std::ops::Deref for SubEvents<S> {
    type Target = Events;

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl<S: SubApp> std::ops::DerefMut for SubEvents<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.events
    }
}

impl<S: SubApp> Resource for SubEvents<S> {}
