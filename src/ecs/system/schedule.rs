use crate::ecs::{storage::dense::DenseMap, world::World};

use super::{IntoSystem, System};
use std::any::TypeId;

pub trait ScheduleId: Send + Sync + 'static {
    const NAME: &'static str;
}

pub struct DefaultId;

impl ScheduleId for DefaultId {
    const NAME: &'static str = "default";
}

pub trait IntoScheduleTag<M> {
    fn into_tag() -> ScheduleTag;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct ScheduleTag {
    name: &'static str,
    ty: TypeId,
}

impl ScheduleTag {
    pub fn new<S: ScheduleId>() -> Self {
        Self {
            name: S::NAME,
            ty: TypeId::of::<S>(),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn ty(&self) -> TypeId {
        self.ty
    }
}

impl PartialOrd for ScheduleTag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(other.name)
    }
}

impl<S: ScheduleId> IntoScheduleTag<S> for S {
    fn into_tag() -> ScheduleTag {
        ScheduleTag::new::<S>()
    }
}

pub struct Schedule {
    systems: Vec<System>,
}

impl Schedule {
    pub fn new() -> Self {
        Self { systems: vec![] }
    }

    pub fn default() -> Self {
        Self::new()
    }

    pub fn add_system<M>(&mut self, system: impl IntoSystem<M>) {
        self.systems.push(system.into_system());
    }

    pub fn append(&mut self, schedule: Schedule) {
        self.systems.extend(schedule.systems);
    }

    pub fn run(&self, world: &World) {
        self.systems.iter().for_each(|system| system.run(world));
    }
}

pub trait Phase: Send + Sync + 'static {
    fn name() -> &'static str;

    fn run(&mut self, world: &World, schdeules: &Schedules);
}

pub trait IntoPhaseTag<P> {
    fn into_tag() -> PhaseTag;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]

pub struct PhaseTag {
    name: &'static str,
    ty: TypeId,
}

impl PhaseTag {
    pub fn new<P: Phase>() -> Self {
        Self {
            name: P::name(),
            ty: TypeId::of::<P>(),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn ty(&self) -> TypeId {
        self.ty
    }
}

impl PartialOrd for PhaseTag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(other.name)
    }
}

impl<P: Phase> IntoPhaseTag<P> for P {
    fn into_tag() -> PhaseTag {
        PhaseTag::new::<P>()
    }
}

pub struct Schedules {
    schedules: DenseMap<ScheduleTag, Schedule>,
}

impl Schedules {
    pub fn new() -> Self {
        Self {
            schedules: DenseMap::new(),
        }
    }

    pub fn add(&mut self, tag: impl Into<ScheduleTag>, schedule: Schedule) {
        let tag = tag.into();
        if let Some(existing) = self.schedules.get_mut(&tag) {
            existing.append(schedule);
        } else {
            self.schedules.insert(tag, schedule);
        }
    }

    pub fn run(&self, tag: impl Into<ScheduleTag>, world: &World) {
        if let Some(schedule) = self.schedules.get(&tag.into()) {
            schedule.run(world);
        }
    }
}

pub struct Phases {
    phases: DenseMap<PhaseTag, Schedules>,
}

impl Phases {
    pub fn new() -> Self {
        Self {
            phases: DenseMap::new(),
        }
    }

    pub fn add<P: Phase>(&mut self, tag: impl Into<ScheduleTag>, schedule: Schedule) {
        let phase = PhaseTag::new::<P>();
        if let Some(existing) = self.phases.get_mut(&phase) {
            existing.add(tag, schedule);
        } else {
            let mut schedules = Schedules::new();
            schedules.add(tag, schedule);
            self.phases.insert(phase, schedules);
        }
    }

    pub fn run(&self, phase: impl Into<PhaseTag>, tag: impl Into<ScheduleTag>, world: &World) {
        if let Some(schedules) = self.phases.get(&phase.into()) {
            schedules.run(tag, world);
        }
    }
}

pub struct PhaseRunner {
    runner: Box<dyn Fn(&World, &Schedules)>,
}

impl PhaseRunner {
    pub fn new<P: Phase>(phase: P) -> Self {
        Self {
            runner: Box::new(move |world, schedules| {
                let mut phase = phase;
                phase.run(world, schedules);
            }),
        }
    }

    pub fn run(&self, world: &World, schedules: &Schedules) {
        (self.runner)(world, schedules);
    }
}

pub struct PhaseOrder {
    phases: DenseMap<PhaseTag, PhaseRunner>,
}

impl PhaseOrder {
    pub fn new() -> Self {
        Self {
            phases: DenseMap::new(),
        }
    }

    pub fn add<P: Phase>(&mut self, phase: P) {
        let tag = PhaseTag::new::<P>();
        self.phases
            .insert(tag, PhaseRunner::new(phase));
    }

    pub fn add_before<P: Phase, B: Phase>(&mut self, phase: P) {
        let before = PhaseTag::new::<B>();
        let ty = PhaseTag::new::<P>();
        self.phases.insert_before(ty, PhaseRunner::new(phase), before);
    }

    pub fn add_after<P: Phase, B: Phase>(&mut self, phase: P) {
        let after = PhaseTag::new::<B>();
        let ty = PhaseTag::new::<P>();
        self.phases.insert_after(ty, PhaseRunner::new(phase), after);
    }

    pub fn run(&mut self, world: &mut World, phases: Phases) {
        for tag in self.phases.keys() {
            if let Some(runner) = self.phases.get(tag) {
                runner.run(world, phases.phases.get(tag).unwrap());
            }
        } 
    }
}
