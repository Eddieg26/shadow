use std::any::TypeId;

use super::{observer::EventObservers, IntoSystem, System};
use crate::ecs::{
    event::{meta::EventMetas, Event},
    storage::dense::DenseMap,
    world::World,
};

pub struct Schedule {
    systems: Vec<System>,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn add_system<M>(&mut self, system: impl IntoSystem<M>) -> &mut Self {
        self.systems.push(system.into_system());
        self
    }

    pub fn append(&mut self, mut schedule: Schedule) -> &mut Self {
        self.systems.append(&mut schedule.systems);
        self
    }

    pub fn run(&self, world: &World) {
        for system in self.systems.iter() {
            system.run(world);
        }
    }
}

pub trait Phase: Send + Sync + 'static {
    fn run(&mut self, runner: impl Fn()) {
        runner();
    }
}

pub struct PhaseRunner {
    ty: TypeId,
    run_phase: Box<dyn FnMut(&World, &Schedule)>,
}

impl PhaseRunner {
    fn new<P: Phase>(mut phase: P) -> Self {
        Self {
            ty: TypeId::of::<P>(),
            run_phase: Box::new(move |world, s| phase.run(|| s.run(world))),
        }
    }

    pub fn ty(&self) -> &TypeId {
        &self.ty
    }

    pub fn run(&mut self, world: &mut World, schedule: &Schedule, observers: &EventObservers) {
        (self.run_phase)(world, schedule);
        self.flush(world, observers);
    }

    fn flush(&self, world: &mut World, observers: &EventObservers) {
        let metas = world.resource::<EventMetas>().clone();
        for mut event in world.events().drain() {
            let meta = metas.get(&event.ty());
            meta.invoke(&mut event, world)
        }

        observers.run(world);
    }
}

pub struct Schedules {
    schedules: DenseMap<TypeId, Schedule>,
}

impl Schedules {
    pub fn new() -> Self {
        Self {
            schedules: DenseMap::new(),
        }
    }

    pub fn add_schedule<P: Phase>(&mut self, schedule: Schedule) -> &mut Self {
        let ty = TypeId::of::<P>();
        let runner = self.schedules.get_mut(&ty).expect("Phase not found");
        runner.append(schedule);
        self
    }

    pub fn add_system<M, P: Phase>(&mut self, system: impl IntoSystem<M>) -> &mut Self {
        let ty = TypeId::of::<P>();
        let runner = self.schedules.get_mut(&ty).expect("Phase not found");
        runner.add_system(system);
        self
    }

    pub fn get<P: Phase>(&self) -> Option<&Schedule> {
        let ty = TypeId::of::<P>();
        self.schedules.get(&ty)
    }

    pub fn schedule(&self, phase: &TypeId) -> Option<&Schedule> {
        self.schedules.get(&phase)
    }
}

pub trait PhaseOrder: 'static {
    fn phases() -> Phases;
}

pub struct Phases {
    phases: DenseMap<TypeId, PhaseRunner>,
}

impl Phases {
    pub fn new() -> Self {
        Self {
            phases: DenseMap::new(),
        }
    }

    pub fn add<P: Phase>(&mut self, phase: P) -> &mut Self {
        let ty = TypeId::of::<P>();
        self.phases.insert(ty, PhaseRunner::new(phase));
        self
    }

    pub fn add_before<P: Phase, B: Phase>(&mut self, phase: P) -> &mut Self {
        let a = TypeId::of::<P>();
        let b = TypeId::of::<B>();
        let phase = PhaseRunner::new(phase);
        self.phases.insert_before(a, phase, b);
        self
    }

    pub fn add_after<P: Phase, B: Phase>(&mut self, phase: P) -> &mut Self {
        let a = TypeId::of::<P>();
        let b = TypeId::of::<B>();
        let phase = PhaseRunner::new(phase);
        self.phases.insert_before(a, phase, b);
        self
    }

    pub fn run(&mut self, world: &mut World, schedules: &Schedules, observers: &EventObservers) {
        for phase in self.phases.values_mut() {
            if let Some(schedule) = schedules.schedule(&phase.ty()) {
                phase.run(world, schedule, observers);
            }
        }
    }

    pub fn flush_event<E: Event>(&self, world: &mut World, observers: &EventObservers) {
        let metas = world.resource::<EventMetas>().clone();
        for mut event in world.events().drain_by_type::<E>() {
            let meta = metas.get(event.ty());
            meta.invoke(&mut event, world)
        }

        observers.run(world);
    }
}
