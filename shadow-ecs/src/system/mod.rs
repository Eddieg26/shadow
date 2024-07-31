use self::access::WorldAccess;
use super::{
    core::{Entities, LocalResource, Resource},
    event::Events,
    world::World,
};
use crate::{
    core::ResourceType,
    system::access::{Access, WorldAccessType},
};

pub mod access;
pub mod graph;
pub mod observer;
pub mod runner;
pub mod schedule;

pub use runner::*;

pub struct System {
    function: Box<dyn for<'a> Fn(&'a World) + Send + Sync>,
    reads: Vec<WorldAccessType>,
    writes: Vec<WorldAccessType>,
    before: Vec<System>,
    after: Vec<System>,
}

impl System {
    fn new<F>(function: F, reads: Vec<WorldAccessType>, writes: Vec<WorldAccessType>) -> Self
    where
        F: for<'a> Fn(&'a World) + Send + Sync + 'static,
    {
        Self {
            function: Box::new(function),
            reads,
            writes,
            before: vec![],
            after: vec![],
        }
    }

    pub fn reads(&self) -> &[WorldAccessType] {
        &self.reads
    }

    pub fn writes(&self) -> &[WorldAccessType] {
        &self.writes
    }

    pub(crate) fn systems(&mut self) -> (Vec<System>, Vec<System>) {
        let before = std::mem::take(&mut self.before);
        let after = std::mem::take(&mut self.after);
        (before, after)
    }

    pub fn run(&self, world: &World) {
        (self.function)(world);
    }
}

pub trait IntoSystem<M> {
    fn into_system(self) -> System;
    fn before<Marker>(self, system: impl IntoSystem<Marker>) -> System;
    fn after<Marker>(self, system: impl IntoSystem<Marker>) -> System;
}

impl IntoSystem<()> for System {
    fn into_system(self) -> System {
        self
    }

    fn before<Marker>(mut self, system: impl IntoSystem<Marker>) -> System {
        self.before.push(system.into_system());
        self
    }

    fn after<Marker>(mut self, system: impl IntoSystem<Marker>) -> System {
        self.after.push(system.into_system());
        self
    }
}

impl<F: Fn() + Send + Sync + 'static> IntoSystem<F> for F {
    fn into_system(self) -> System {
        let system = System::new(
            move |_| {
                (self)();
            },
            vec![],
            vec![],
        );

        system
    }

    fn before<Marker>(self, other: impl IntoSystem<Marker>) -> System {
        let mut system = System::new(
            move |_| {
                (self)();
            },
            vec![],
            vec![],
        );

        system.before.push(other.into_system());

        system
    }

    fn after<Marker>(self, other: impl IntoSystem<Marker>) -> System {
        let mut system = System::new(
            move |_| {
                (self)();
            },
            vec![],
            vec![],
        );

        system.after.push(other.into_system());

        system
    }
}

/// A collection of systems that can be run in sequence.
pub struct SystemSet {
    systems: Vec<System>,
}

impl SystemSet {
    pub fn new() -> Self {
        Self { systems: vec![] }
    }

    pub fn add_system<M>(&mut self, system: impl IntoSystem<M>) {
        self.systems.push(system.into_system());
    }

    pub fn append(&mut self, mut system_set: SystemSet) {
        self.systems.append(&mut system_set.systems);
    }

    pub fn reads(&self) -> Vec<WorldAccessType> {
        self.systems
            .iter()
            .flat_map(|system| system.reads().to_vec())
            .collect()
    }

    pub fn writes(&self) -> Vec<WorldAccessType> {
        self.systems
            .iter()
            .flat_map(|system| system.writes().to_vec())
            .collect()
    }
}

impl IntoSystem<()> for SystemSet {
    fn into_system(self) -> System {
        let mut reads = vec![];
        let mut writes = vec![];

        for system in &self.systems {
            reads.extend(system.reads().to_vec());
            writes.extend(system.writes().to_vec());
        }

        let system = System::new(
            move |world| {
                for system in &self.systems {
                    system.run(world);
                }
            },
            reads,
            writes,
        );

        system
    }

    fn before<Marker>(self, other: impl IntoSystem<Marker>) -> System {
        let mut reads = vec![];
        let mut writes = vec![];

        for system in &self.systems {
            reads.extend(system.reads().to_vec());
            writes.extend(system.writes().to_vec());
        }

        let mut system = System::new(
            move |world| {
                for system in &self.systems {
                    system.run(world);
                }
            },
            reads,
            writes,
        );

        system.before.push(other.into_system());

        system
    }

    fn after<Marker>(self, other: impl IntoSystem<Marker>) -> System {
        let mut reads = vec![];
        let mut writes = vec![];

        for system in &self.systems {
            reads.extend(system.reads().to_vec());
            writes.extend(system.writes().to_vec());
        }

        let mut system = System::new(
            move |world| {
                for system in &self.systems {
                    system.run(world);
                }
            },
            reads,
            writes,
        );

        system.after.push(other.into_system());

        system
    }
}

pub trait SystemArg {
    type Item<'a>;

    fn get<'a>(world: &'a World) -> Self::Item<'a>;
    fn access() -> Vec<WorldAccess>;
}

impl SystemArg for &World {
    type Item<'a> = &'a World;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world
    }

    fn access() -> Vec<WorldAccess> {
        let ty = WorldAccessType::World;
        vec![WorldAccess::new(ty, Access::Read)]
    }
}

impl<R: Resource> SystemArg for &R {
    type Item<'a> = &'a R;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.resource::<R>()
    }

    fn access() -> Vec<WorldAccess> {
        let ty = WorldAccessType::Resource(ResourceType::new::<R>());
        vec![WorldAccess::new(ty, Access::Read)]
    }
}

impl<R: Resource> SystemArg for &mut R {
    type Item<'a> = &'a mut R;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.resource_mut::<R>()
    }

    fn access() -> Vec<WorldAccess> {
        let ty = WorldAccessType::Resource(ResourceType::new::<R>());
        vec![WorldAccess::new(ty, Access::Write)]
    }
}

impl SystemArg for &Events {
    type Item<'a> = &'a Events;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.events()
    }

    fn access() -> Vec<WorldAccess> {
        let ty = WorldAccessType::Resource(ResourceType::new::<Events>());
        vec![WorldAccess::new(ty, Access::Read)]
    }
}

pub struct Local<R: LocalResource> {
    _marker: std::marker::PhantomData<R>,
}
impl<R: LocalResource> SystemArg for &Local<R> {
    type Item<'a> = &'a R;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.local_resource::<R>()
    }

    fn access() -> Vec<WorldAccess> {
        let ty = WorldAccessType::LocalResource(ResourceType::new::<R>());
        vec![WorldAccess::new(ty, Access::Read)]
    }
}

impl<R: LocalResource> SystemArg for &mut Local<R> {
    type Item<'a> = &'a mut R;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.local_resource_mut::<R>()
    }

    fn access() -> Vec<WorldAccess> {
        let ty = WorldAccessType::LocalResource(ResourceType::new::<R>());
        vec![WorldAccess::new(ty, Access::Write)]
    }
}

impl SystemArg for &Entities {
    type Item<'a> = &'a Entities;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.entities()
    }

    fn access() -> Vec<WorldAccess> {
        vec![WorldAccess::new(WorldAccessType::None, Access::Read)]
    }
}

pub struct Cloned<C: Clone + Resource>(std::marker::PhantomData<C>);

impl<C: Clone + Resource> SystemArg for Cloned<C> {
    type Item<'a> = C;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.resource::<C>().clone()
    }

    fn access() -> Vec<WorldAccess> {
        vec![]
    }
}

pub type ArgItem<'a, A> = <A as SystemArg>::Item<'a>;

macro_rules! impl_into_system {
    ($($arg:ident),*) => {
        impl<F, $($arg: SystemArg),*> IntoSystem<(F, $($arg),*)> for F
        where
            for<'a> F: Fn($($arg),*) + Fn($(ArgItem<'a, $arg>),*) + Send + Sync + 'static,
        {
            fn into_system(self) -> System {
                let mut reads = vec![];
                let mut writes = vec![];
                let mut metas = vec![];

                $(metas.extend($arg::access());)*

                WorldAccess::pick(&mut reads, &mut writes, &metas);

                let system = System::new(move |world| {
                    (self)($($arg::get(world)),*);
                }, reads, writes);

                system
            }

            fn before<Marker>(self, other: impl IntoSystem<Marker>) -> System {
                let mut reads = vec![];
                let mut writes = vec![];
                let mut metas = vec![];

                $(metas.extend($arg::access());)*

                WorldAccess::pick(&mut reads, &mut writes, &metas);

                let mut system = System::new(move |world| {
                    (self)($($arg::get(world)),*);
                }, reads, writes);

                system.before.push(other.into_system());

                system
            }

            fn after<Marker>(self, other: impl IntoSystem<Marker>) -> System {
                let mut reads = vec![];
                let mut writes = vec![];
                let mut metas = vec![];

                $(metas.extend($arg::access());)*

                WorldAccess::pick(&mut reads, &mut writes, &metas);

                let mut system = System::new(move |world| {
                    (self)($($arg::get(world)),*);
                }, reads, writes);

                system.after.push(other.into_system());

                system
            }
        }

        impl<$($arg: SystemArg),*> SystemArg for ($($arg,)*) {
            type Item<'a> = ($($arg::Item<'a>,)*);

            fn get<'a>(world: &'a World) -> Self::Item<'a> {
                ($($arg::get(world),)*)
            }

            fn access() -> Vec<WorldAccess> {
                let mut metas = Vec::new();
                $(metas.extend($arg::access());)*
                metas
            }
        }
    };
}

impl_into_system!(A);
impl_into_system!(A, B);
impl_into_system!(A, B, C);
impl_into_system!(A, B, C, D);
impl_into_system!(A, B, C, D, E);
impl_into_system!(A, B, C, D, E, F2);
impl_into_system!(A, B, C, D, E, F2, G);
impl_into_system!(A, B, C, D, E, F2, G, H);
impl_into_system!(A, B, C, D, E, F2, G, H, I);
