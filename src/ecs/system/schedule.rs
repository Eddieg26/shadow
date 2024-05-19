use crate::ecs::{storage::dense::DenseMap, world::World};
use super::{IntoSystem, System};

pub struct Schedule {
    systems: Vec<System>,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn add_system<M>(&mut self, system: impl IntoSystem<M>) {
        self.systems.push(system.into_system());
    }

    pub fn run(&self, world: &World) {
        for system in self.systems.iter() {
            system.run(world);
        }
    }
}

pub trait Phase: 'static {
    const NAME: &'static str;
}

pub struct Schedules {
    schedules: DenseMap<&'static str, Schedule>,
}

impl Schedules {
    pub fn new() -> Self {
        Self {
            schedules: DenseMap::new(),
        }
    }

    pub fn add_schedule<P: Phase>(&mut self, schedule: Schedule) -> &mut Self {
        self.schedules.insert(P::NAME, schedule);
        self
    }

    pub fn add_system<P: Phase, M>(&mut self, system: impl IntoSystem<M>) -> &mut Self {
        if let Some(schedule) = self.schedules.get_mut(&P::NAME) {
            schedule.add_system(system);
        } else {
            let mut schedule = Schedule::new();
            schedule.add_system(system);
            self.schedules.insert(P::NAME, schedule);
        }
        self
    }

    pub fn run<P: Phase>(&self, world: &World) {
        if let Some(schedule) = self.schedules.get(&P::NAME) {
            schedule.run(world);
        }
    }
}
