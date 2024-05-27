use shadow_ecs::ecs::{
    storage::dense::DenseMap,
    system::{IntoSystem, RunMode, Systems},
    world::World,
};
use std::{
    any::TypeId,
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex},
};

pub use phases::*;

pub trait Phase: Sized + 'static {
    type Runner: PhaseRunner;

    fn ty(&self) -> PhaseType {
        PhaseType::new::<Self>()
    }

    fn runner() -> Self::Runner;

    fn schedule() -> Schedule {
        Schedule::new()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhaseType(u64);

impl PhaseType {
    pub fn new<P: Phase>() -> Self {
        let type_id = TypeId::of::<P>();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        type_id.hash(&mut hasher);
        Self(hasher.finish())
    }
}

pub struct PhaseSystems {
    systems: DenseMap<PhaseType, Systems>,
}

impl PhaseSystems {
    pub fn new() -> Self {
        Self {
            systems: DenseMap::new(),
        }
    }

    pub fn add_system<M>(&mut self, phase: impl Phase, system: impl IntoSystem<M>) {
        let ty = phase.ty();
        if let Some(systems) = self.systems.get_mut(&ty) {
            systems.add_system(system);
        } else {
            let mut systems = Systems::new(RunMode::Sequential);
            systems.add_system(system);
            self.systems.insert(ty, systems);
        }
    }

    pub fn get<P: Phase>(&self) -> Option<&Systems> {
        let ty = PhaseType::new::<P>();
        self.systems.get(&ty)
    }

    pub fn get_dyn(&self, ty: &PhaseType) -> Option<&Systems> {
        self.systems.get(ty)
    }

    pub fn build(&mut self) {
        for systems in self.systems.values_mut() {
            systems.build();
        }
    }
}

pub struct RunContext<'a> {
    systems: &'a SystemDatabase,
    world: &'a mut World,
    schedule: &'a Schedule,
    phase: PhaseType,
}

impl<'a> RunContext<'a> {
    pub fn new(
        systems: &'a SystemDatabase,
        world: &'a mut World,
        schedule: &'a Schedule,
        phase: PhaseType,
    ) -> Self {
        Self {
            systems,
            world,
            schedule,
            phase,
        }
    }

    pub fn run(&mut self) {
        for systems in self.systems.get_phase_systems(&self.phase) {
            systems.run(self.world);
        }

        self.schedule.run(&self.systems, self.world);
    }
}

pub trait PhaseRunner {
    fn run(&mut self, ctx: RunContext);
}

pub struct DefaultPhaseRunner;

impl PhaseRunner for DefaultPhaseRunner {
    fn run(&mut self, mut ctx: RunContext) {
        ctx.run();
    }
}

pub struct ErasedPhase {
    ty: PhaseType,
    schedule: Schedule,
    runner: Arc<Mutex<Box<dyn PhaseRunner>>>,
}

impl ErasedPhase {
    pub fn new<P: Phase>() -> Self {
        Self {
            ty: PhaseType::new::<P>(),
            schedule: P::schedule(),
            runner: Arc::new(Mutex::new(Box::new(P::runner()))),
        }
    }

    pub fn ty(&self) -> PhaseType {
        self.ty
    }

    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    pub fn add_phase<Q: Phase>(&mut self) {
        self.schedule.add_phase::<Q>();
    }

    pub fn insert_before<Q: Phase, R: Phase>(&mut self) -> bool {
        self.schedule.insert_before::<Q, R>()
    }

    pub fn insert_after<Q: Phase, R: Phase>(&mut self) -> bool {
        self.schedule.insert_after::<Q, R>()
    }

    pub fn run(&self, systems: &SystemDatabase, world: &mut World) {
        let ctx = RunContext::new(systems, world, &self.schedule, self.ty);
        self.runner.lock().unwrap().run(ctx);
    }
}

pub struct Schedule {
    phases: DenseMap<PhaseType, ErasedPhase>,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            phases: DenseMap::new(),
        }
    }

    pub fn has(&self, ty: PhaseType) -> bool {
        self.phases.contains(&ty)
    }

    pub fn get<P: Phase>(&self) -> Option<&ErasedPhase> {
        let ty = PhaseType::new::<P>();
        self.phases.get(&ty)
    }

    pub fn get_mut<P: Phase>(&mut self) -> Option<&mut ErasedPhase> {
        let ty = PhaseType::new::<P>();
        self.phases.get_mut(&ty)
    }

    pub fn add_phase<P: Phase>(&mut self) {
        let phase = ErasedPhase::new::<P>();
        self.phases.insert(phase.ty, phase);
    }

    pub fn insert_before<P: Phase, Q: Phase>(&mut self) -> bool {
        let phase = ErasedPhase::new::<P>();
        if self.has(phase.ty) {
            let before_ty = PhaseType::new::<Q>();
            self.phases.insert_before(phase.ty, phase, before_ty);
            true
        } else {
            self.phases
                .values_mut()
                .iter_mut()
                .any(|phase| phase.insert_before::<P, Q>())
        }
    }

    pub fn insert_after<P: Phase, Q: Phase>(&mut self) -> bool {
        let phase = ErasedPhase::new::<P>();
        if self.has(phase.ty) {
            let after_ty = PhaseType::new::<Q>();
            self.phases.insert_after(phase.ty, phase, after_ty);
            true
        } else {
            self.phases
                .values_mut()
                .iter_mut()
                .any(|phase| phase.insert_after::<P, Q>())
        }
    }

    pub fn run(&self, systems: &SystemDatabase, world: &mut World) {
        for phase in self.phases.values() {
            phase.run(systems, world);
            world.flush();
        }
    }
}

pub struct SystemDatabase {
    systems: DenseMap<u64, PhaseSystems>,
}

impl SystemDatabase {
    pub fn new() -> Self {
        Self {
            systems: DenseMap::new(),
        }
    }

    pub fn add_systems(&mut self, id: impl Hash, systems: PhaseSystems) {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        self.systems.insert(hasher.finish(), systems);
    }

    pub fn remove_systems(&mut self, id: impl Hash) {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        self.systems.remove(&hasher.finish());
    }

    pub fn get(&self, id: impl Hash) -> Option<&PhaseSystems> {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        self.systems.get(&hasher.finish())
    }

    pub fn get_mut(&mut self, id: impl Hash) -> Option<&mut PhaseSystems> {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        self.systems.get_mut(&hasher.finish())
    }

    pub fn get_phase_systems(&self, ty: &PhaseType) -> Vec<&Systems> {
        self.systems
            .values()
            .iter()
            .filter_map(|systems| systems.get_dyn(ty))
            .collect()
    }

    pub fn build(&mut self) {
        for systems in self.systems.values_mut() {
            systems.build();
        }
    }
}

pub struct MainSchedule {
    schedule: Schedule,
    systems: SystemDatabase,
}

impl MainSchedule {
    pub fn new() -> Self {
        let mut schedule = Schedule::new();
        schedule.add_phase::<phases::Init>();
        schedule.add_phase::<phases::Execute>();
        schedule.add_phase::<phases::Shutdown>();

        {
            let execute = schedule.get_mut::<phases::Execute>().unwrap();
            execute.add_phase::<phases::Start>();
            execute.add_phase::<phases::Main>();
            execute.add_phase::<phases::End>();
        }

        let mut systems = SystemDatabase::new();
        systems.add_systems("global", PhaseSystems::new());

        Self { schedule, systems }
    }

    pub fn add_phase<P: Phase>(&mut self) {
        self.schedule.add_phase::<P>();
    }

    pub fn add_system<M>(&mut self, phase: impl Phase, system: impl IntoSystem<M>) {
        self.systems
            .get_mut("global")
            .unwrap()
            .add_system(phase, system);
    }

    pub fn add_systems(&mut self, id: impl Hash, systems: PhaseSystems) {
        self.systems.add_systems(id, systems);
    }

    pub fn remove_systems(&mut self, id: impl Hash) {
        self.systems.remove_systems(id);
    }

    pub fn insert_before<P: Phase, Q: Phase>(&mut self) -> bool {
        self.schedule.insert_before::<P, Q>()
    }

    pub fn insert_after<P: Phase, Q: Phase>(&mut self) -> bool {
        self.schedule.insert_after::<P, Q>()
    }

    pub fn build(&mut self) {
        self.systems.build();
    }

    pub fn run<P: Phase>(&self, world: &mut World) -> Option<()> {
        let phase = self.schedule.get::<P>()?;
        phase.run(&self.systems, world);
        Some(())
    }
}

pub mod phases {
    use super::{Phase, PhaseRunner};

    pub struct Init;
    impl Phase for Init {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }

    pub struct Execute;
    impl Phase for Execute {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }

    pub struct Shutdown;
    impl Phase for Shutdown {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }

    pub struct PreFixedUpdate;
    impl Phase for PreFixedUpdate {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }

    pub struct FixedUpdate;
    impl Phase for FixedUpdate {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }

    pub struct PostFixedUpdate;
    impl Phase for PostFixedUpdate {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }

    pub struct FixedRunner;

    impl PhaseRunner for FixedRunner {
        fn run(&mut self, mut ctx: super::RunContext) {
            ctx.run();
        }
    }

    pub struct Fixed;
    impl Phase for Fixed {
        type Runner = FixedRunner;

        fn runner() -> Self::Runner {
            FixedRunner
        }

        fn schedule() -> super::Schedule {
            let mut schedule = super::Schedule::new();
            schedule.add_phase::<PreFixedUpdate>();
            schedule.add_phase::<FixedUpdate>();
            schedule.add_phase::<PostFixedUpdate>();
            schedule
        }
    }

    pub struct PreUpdate;
    impl Phase for PreUpdate {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }

    pub struct Update;
    impl Phase for Update {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }

    pub struct PostUpdate;
    impl Phase for PostUpdate {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }

    pub struct Main;
    impl Phase for Main {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }

        fn schedule() -> super::Schedule {
            let mut schedule = super::Schedule::new();
            schedule.add_phase::<Fixed>();
            schedule.add_phase::<PreUpdate>();
            schedule.add_phase::<Update>();
            schedule.add_phase::<PostUpdate>();
            schedule
        }
    }

    pub struct Start;
    impl Phase for Start {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }

    pub struct End;
    impl Phase for End {
        type Runner = super::DefaultPhaseRunner;

        fn runner() -> Self::Runner {
            super::DefaultPhaseRunner
        }
    }
}
