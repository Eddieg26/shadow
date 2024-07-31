use super::{IntoSystem, ParallelRunner, RunMode, SequentialRunner, SystemGraph, SystemRunner};
use crate::ecs::{
    storage::dense::{DenseMap, DenseSet},
    world::World,
};
use std::{
    any::TypeId,
    hash::{Hash, Hasher},
};

pub trait Phase: Sized + 'static {
    fn id(&self) -> ScheduleId {
        ScheduleId::new::<Self>()
    }

    fn schedule() -> Schedule {
        Schedule::from::<Self>()
    }
}

pub struct RunContext<'a> {
    world: &'a mut World,
    systems: Vec<&'a SystemGraph>,
    runner: &'a SystemRunner,
}

impl<'a> RunContext<'a> {
    pub fn new(
        world: &'a mut World,
        systems: Vec<&'a SystemGraph>,
        runner: &'a SystemRunner,
    ) -> Self {
        Self {
            world,
            systems,
            runner,
        }
    }

    pub fn run(&self) {
        for system in self.systems.iter() {
            self.runner.run(system, self.world);
        }
    }
}

pub trait PhaseRunner: Send + Sync + 'static {
    fn run(&self, ctx: RunContext);
}

pub struct DefaultPhaseRunner;

impl PhaseRunner for DefaultPhaseRunner {
    fn run(&self, ctx: RunContext) {
        ctx.run();
    }
}

pub struct Root;

impl Phase for Root {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ScheduleId(u64);

impl ScheduleId {
    pub fn new<P: Phase>() -> Self {
        let type_id = TypeId::of::<P>();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        type_id.hash(&mut hasher);
        Self(hasher.finish())
    }
}

pub struct Schedule {
    id: ScheduleId,
    children: DenseMap<ScheduleId, Schedule>,
}

impl Schedule {
    pub fn new(id: ScheduleId) -> Self {
        Self {
            id,
            children: DenseMap::new(),
        }
    }

    pub fn from<P: Phase>() -> Self {
        Self::new(ScheduleId::new::<P>())
    }

    pub fn id(&self) -> ScheduleId {
        self.id
    }

    pub fn has(&self, id: &ScheduleId) -> bool {
        self.children.contains(id)
    }

    pub fn get(&self, id: &ScheduleId) -> Option<&Schedule> {
        match self.children.get(id) {
            Some(child) => Some(child),
            None => self.children.values().iter().find_map(|c| c.get(id)),
        }
    }

    pub fn get_mut(&mut self, id: &ScheduleId) -> Option<&mut Schedule> {
        let ptr: *mut Self = self;

        unsafe {
            let len = self.children.len();

            if let Some(child) = self.children.get_mut(id) {
                return Some(child);
            } else {
                for _ in 0..len {
                    let ptr_mut: &mut Self = &mut *ptr;
                    if let Some(child) = ptr_mut.children.get_mut(id) {
                        return Some(child);
                    }
                }

                None
            }
        }
    }

    pub fn children(&self) -> &[ScheduleId] {
        self.children.keys()
    }

    pub fn schedules(&self) -> &[Schedule] {
        self.children.values()
    }

    pub fn add_schedule(&mut self, schedule: Schedule) {
        self.children.insert(schedule.id, schedule);
    }

    pub fn add_child<Main: Phase, Sub: Phase>(&mut self) -> bool {
        let main = ScheduleId::new::<Main>();
        if main == self.id {
            let sub = ScheduleId::new::<Sub>();
            self.children.insert(sub, Sub::schedule());
            true
        } else {
            let mut children = self.children.values_mut().iter_mut();
            children.any(|schedule| schedule.add_child::<Main, Sub>())
        }
    }

    pub fn insert_before<Main: Phase, Before: Phase>(&mut self) -> bool {
        let before = ScheduleId::new::<Before>();
        if self.has(&before) {
            let main = ScheduleId::new::<Main>();
            self.children.insert_before(main, Main::schedule(), before);
            true
        } else {
            let mut children = self.children.values_mut().iter_mut();
            children.any(|schedule| schedule.insert_before::<Main, Before>())
        }
    }

    pub fn insert_after<Main: Phase, After: Phase>(&mut self) -> bool {
        let after = ScheduleId::new::<After>();
        if self.has(&after) {
            let main = ScheduleId::new::<Main>();
            self.children.insert_after(main, Main::schedule(), after);
            true
        } else {
            let mut children = self.children.values_mut().iter_mut();
            children.any(|schedule| schedule.insert_after::<Main, After>())
        }
    }

    pub fn run_child(&self, id: ScheduleId, world: &mut World, systems: &Systems) {
        if let Some(child) = self.children.get(&id) {
            child.run(world, systems);
        }
    }

    pub fn run(&self, world: &mut World, systems: &Systems) {
        let system_runner = &systems.runner;
        let phase_runner = systems
            .phase_runner(&self.id)
            .unwrap_or(&DefaultPhaseRunner);

        let graphs = systems.systems(&self.id);
        let ctx = RunContext::new(world, graphs, system_runner);
        phase_runner.run(ctx);

        world.flush();

        for child in self.children.values() {
            child.run(world, systems);
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum SystemTag {
    Global,
    Other(String),
}

impl From<&str> for SystemTag {
    fn from(value: &str) -> Self {
        SystemTag::Other(value.to_string())
    }
}

impl From<String> for SystemTag {
    fn from(value: String) -> Self {
        SystemTag::Other(value)
    }
}

pub trait SystemGroup: 'static {
    fn name() -> &'static str;
    fn systems() -> SystemGraphs;
}

pub struct SystemGraphs {
    graphs: DenseMap<ScheduleId, SystemGraph>,
}

impl SystemGraphs {
    pub fn new() -> Self {
        Self {
            graphs: DenseMap::new(),
        }
    }

    pub fn get(&self, id: &ScheduleId) -> Option<&SystemGraph> {
        self.graphs.get(id)
    }

    pub fn add_system<M>(&mut self, phase: impl Phase, system: impl IntoSystem<M>) {
        if let Some(graph) = self.graphs.get_mut(&phase.id()) {
            graph.add_system(system);
        } else {
            let mut graph = SystemGraph::new();
            graph.add_system(system);
            self.graphs.insert(phase.id(), graph);
        }
    }

    pub fn build(&mut self) {
        for graph in self.graphs.values_mut() {
            graph.build();
        }
    }

    pub fn run(&self, id: ScheduleId, world: &mut World, runner: &SystemRunner) {
        if let Some(graph) = self.graphs.get(&id) {
            runner.run(graph, world);
        }
    }
}

pub struct PhaseRunners {
    runners: DenseMap<ScheduleId, Box<dyn PhaseRunner>>,
}

impl PhaseRunners {
    pub fn new() -> Self {
        Self {
            runners: DenseMap::new(),
        }
    }

    pub fn add<P: Phase>(&mut self, runner: impl PhaseRunner) {
        let id = ScheduleId::new::<P>();
        self.runners.insert(id, Box::new(runner));
    }

    pub fn get(&self, id: &ScheduleId) -> Option<&dyn PhaseRunner> {
        self.runners.get(id).map(|runner| &**runner)
    }
}

pub struct Systems {
    schedule: Schedule,
    phases: PhaseRunners,
    active: DenseMap<SystemTag, SystemGraphs>,
    mode: RunMode,
    runner: SystemRunner,
}

impl Systems {
    pub fn new(mode: RunMode) -> Self {
        let mut active = DenseMap::new();
        active.insert(SystemTag::Global, SystemGraphs::new());

        let runner = match mode {
            RunMode::Sequential => SystemRunner::new(SequentialRunner),
            RunMode::Parallel => SystemRunner::new(ParallelRunner),
        };

        let schedule = Schedule::new(ScheduleId::new::<Root>());

        Self {
            active,
            phases: PhaseRunners::new(),
            mode,
            runner,
            schedule,
        }
    }

    pub fn mode(&self) -> RunMode {
        self.mode
    }

    pub fn is_active(&self, tag: &SystemTag) -> bool {
        self.active.contains(tag)
    }

    pub fn active(&self) -> &[SystemTag] {
        self.active.keys()
    }

    pub fn systems(&self, id: &ScheduleId) -> Vec<&SystemGraph> {
        let mut systems = vec![];
        for group in self.active.values() {
            if let Some(graph) = group.get(id) {
                systems.push(graph);
            }
        }

        systems
    }

    pub fn phase_runner(&self, id: &ScheduleId) -> Option<&dyn PhaseRunner> {
        self.phases.get(id)
    }

    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    pub fn add_phase<P: Phase>(&mut self) {
        self.schedule.add_schedule(P::schedule());
    }

    pub fn add_sub_phase<Main: Phase, Sub: Phase>(&mut self) -> bool {
        self.schedule.add_child::<Main, Sub>()
    }

    pub fn insert_phase_before<Main: Phase, Before: Phase>(&mut self) -> bool {
        self.schedule.insert_before::<Main, Before>()
    }

    pub fn insert_phase_after<Main: Phase, After: Phase>(&mut self) -> bool {
        self.schedule.insert_after::<Main, After>()
    }

    pub fn add_system<M>(&mut self, phase: impl Phase, system: impl IntoSystem<M>) {
        let systems = self.active.get_mut(&SystemTag::Global).unwrap();
        systems.add_system(phase, system);
    }

    pub fn add_phase_runner<P: Phase>(&mut self, runner: impl PhaseRunner) {
        self.phases.add::<P>(runner);
    }

    pub fn activate(&mut self, tag: SystemTag, systems: SystemGraphs) {
        self.active.insert(tag, systems);
    }

    pub fn deactivate(&mut self, tag: &SystemTag) {
        match tag {
            SystemTag::Global => None,
            _ => self.active.remove(&tag),
        };
    }

    pub fn build(&mut self) {
        for systems in self.active.values_mut() {
            systems.build();
        }
    }

    pub fn run(&self, id: ScheduleId, world: &mut World) {
        self.schedule.run_child(id, world, self)
    }
}

pub struct SystemsInfo {
    builders: DenseMap<SystemTag, fn() -> SystemGraphs>,
    activate: DenseSet<SystemTag>,
    deactivate: DenseSet<SystemTag>,
}

impl SystemsInfo {
    pub fn new() -> Self {
        Self {
            builders: DenseMap::new(),
            activate: DenseSet::new(),
            deactivate: DenseSet::new(),
        }
    }

    pub fn add_system_group<G: SystemGroup>(&mut self) {
        self.builders.insert(G::name().into(), G::systems);
    }

    pub fn activate(&mut self, tag: SystemTag) {
        self.activate.insert(tag);
    }

    pub fn deactivate(&mut self, tag: SystemTag) {
        self.deactivate.insert(tag);
    }

    pub fn update(&mut self, systems: &mut Systems) {
        for tag in self.deactivate.drain() {
            systems.deactivate(&tag);
        }

        for tag in self.activate.drain() {
            if let Some(builder) = self.builders.get(&tag) {
                let mut graphs = builder();
                graphs.build();
                systems.activate(tag, graphs);
            }
        }
    }
}
