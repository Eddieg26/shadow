use self::{
    core::{Component, LocalResource, Resource},
    event::Event,
    system::{
        observer::{EventObservers, IntoObserver, Observers},
        schedule::{Phase, PhaseOrder, Phases, Schedules},
        IntoSystem,
    },
    world::World,
};

pub mod archetype;
pub mod core;
pub mod event;
pub mod storage;
pub mod system;
pub mod world;

pub struct Ecs {
    world: World,
    phases: Phases,
    observers: EventObservers,
    schedules: Schedules,
}

impl Ecs {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            phases: Phases::new(),
            observers: EventObservers::new(),
            schedules: Schedules::new(),
        }
    }

    // pub fn phase_order<P: PhaseOrder>(&mut self) -> &mut Self {
    //     self.phases = P::phases();
    //     self
    // }

    // pub fn add_phase<P: Phase>(&mut self, phase: P) -> &mut Self {
    //     self.phases.add(phase);
    //     self
    // }

    // pub fn add_phase_before<P: Phase, B: Phase>(&mut self, phase: P) -> &mut Self {
    //     self.phases.add_before::<P, B>(phase);
    //     self
    // }

    // pub fn add_phase_after<P: Phase, B: Phase>(&mut self, phase: P) -> &mut Self {
    //     self.phases.add_after::<P, B>(phase);
    //     self
    // }

    pub fn add_system<P: Phase, M>(&mut self, system: impl IntoSystem<M>) -> &mut Self {
        self.schedules.add_system::<M, P>(system);
        self
    }

    pub fn add_observer<E: Event, M>(&mut self, observer: impl IntoObserver<E, M>) -> &mut Self {
        self.observers.add_observer::<E, M>(observer);
        self
    }

    pub fn add_observers<E: Event>(&mut self, observers: Observers<E>) -> &mut Self {
        self.observers.add_observers(observers);
        self
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
}
