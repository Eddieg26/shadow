use crate::phases::Update;
use shadow_ecs::{
    core::{DenseMap, Resource},
    system::schedule::Phase,
    world::World,
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

    pub fn add<A: SubApp>(&mut self) {
        self.worlds.insert(TypeId::of::<A>(), World::new());
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
            main_world.init_sub_world(&mut world);
            apps.apps.insert(id, SubAppWorld::new(world));
        }
        apps
    }
}

impl Into<SubApps> for SubWorlds {
    fn into(self) -> SubApps {
        let mut apps = SubApps::new();
        for (id, world) in self.worlds {
            apps.apps.insert(id, SubAppWorld::new(world));
        }
        apps
    }
}

pub struct SubAppWorld {
    world: Arc<RwLock<World>>,
}

impl SubAppWorld {
    pub fn new(mut world: World) -> Self {
        world.add_phase::<Extract>();
        world.add_phase::<Update>();
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
