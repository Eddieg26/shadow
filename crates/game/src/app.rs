use crate::phases::Update;
use ecs::{
    core::{DenseMap, Resource},
    system::{access::WorldAccess, schedule::Phase, ArgItem, SystemArg},
    world::{event::Events, World},
};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

pub struct Extract;

impl Phase for Extract {}

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

pub struct Main<'w, S: SystemArg>(ArgItem<'w, S>);

impl<'w, S: SystemArg> std::ops::Deref for Main<'w, S> {
    type Target = ArgItem<'w, S>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'w, 's, S: SystemArg> std::ops::DerefMut for Main<'w, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'w, P: SystemArg> Main<'w, P> {
    pub fn into_inner(self) -> ArgItem<'w, P> {
        self.0
    }
}

impl<S: SystemArg + 'static> SystemArg for Main<'_, S> {
    type Item<'world> = Main<'world, S>;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        Main(S::get(world))
    }

    fn access() -> Vec<WorldAccess> {
        S::access()
    }
}

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

pub trait SubApp: Send + Sync + 'static {}

#[derive(Default)]
pub struct SubAppBuilders {
    worlds: DenseMap<TypeId, World>,
}

impl SubAppBuilders {
    pub fn new() -> Self {
        Self {
            worlds: DenseMap::new(),
        }
    }

    pub fn add<A: SubApp>(&mut self, main: &World) -> &mut World {
        let ty = TypeId::of::<A>();
        if !self.worlds.contains(&ty) {
            let mut world = main.sub();
            world.add_phase::<Extract>();
            world.add_phase::<Update>();
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
            let metas = main_world.events().metas();
            for (ty, meta) in metas.iter() {
                meta.add_outputs(&mut world);
                world.events().metas_mut().insert(*ty, *meta);
            }

            let main_events = MainEvents::from(main_world.events().clone());
            world.add_resource(main_events);
            apps.apps.insert(id, SubAppWorld::new(world));
        }
        apps
    }
}

pub struct SubAppWorld {
    world: Arc<RwLock<World>>,
}

impl SubAppWorld {
    pub fn new(world: World) -> Self {
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
            world.flush();
            world.run(Extract);
            world
                .remove_resource::<MainWorld>()
                .expect("Main world not found")
        };

        let world = self.world.clone();
        tasks.spawn(move || {
            let mut world = world.write().unwrap();
            world.run(Update);
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
