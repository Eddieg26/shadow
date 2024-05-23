use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use super::{
    graph::{Graph, GraphNode},
    IntoSystem, System,
};
use crate::ecs::{
    task::{JobBarrier, ScopedTaskPool},
    world::World,
};

impl GraphNode for System {
    fn is_dependency(&self, other: &Self) -> bool {
        let mut writes = other.writes().iter();
        if writes.any(|ty| self.reads().contains(ty) || self.writes().contains(ty)) {
            true
        } else if other.reads().iter().any(|ty| self.writes().contains(ty)) {
            true
        } else {
            false
        }
    }
}

pub struct Systems {
    graph: Graph<System>,
    runner: SystemRunner,
}

impl Systems {
    pub fn new(mode: RunMode) -> Self {
        Self {
            runner: match mode {
                RunMode::Sequential => SystemRunner::new(SequentialRunner),
                RunMode::Parallel => SystemRunner::new(ParallelRunner),
            },
            graph: Graph::new(),
        }
    }

    pub fn add_system<M>(&mut self, system: impl IntoSystem<M>) -> usize {
        let mut system = system.into_system();

        let (before, mut after) = system.systems();

        let after_ids = after
            .drain(0..)
            .map(|s| self.add_system(s))
            .collect::<Vec<_>>();

        let id = self.graph.insert(system);

        after_ids
            .iter()
            .for_each(|a| self.graph.add_depenency(id, *a));

        for before in before {
            let before_id = self.graph.insert(before);
            self.graph.add_depenency(before_id, id);
        }

        id
    }

    pub fn build(&mut self) {
        self.graph.build();
    }

    pub fn run(&self, world: &World) {
        if !self.graph.is_built() {
            println!("System graph not built.")
        } else {
            self.runner.run(self, world);
        }
    }
}

impl std::fmt::Display for Systems {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.graph.fmt(f)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RunMode {
    Sequential,
    Parallel,
}

pub trait Runner: 'static {
    fn run(&self, graph: &Graph<System>, world: &World);
}

pub struct SequentialRunner;

impl Runner for SequentialRunner {
    fn run(&self, graph: &Graph<System>, world: &World) {
        for systems in graph.iter() {
            for system in systems {
                system.run(world);
            }
        }
    }
}

pub struct ParallelRunner;

impl Runner for ParallelRunner {
    fn run(&self, graph: &Graph<System>, world: &World) {
        let available_threads = std::thread::available_parallelism()
            .unwrap_or(NonZeroUsize::new(1).unwrap())
            .into();
        for row in graph.iter() {
            let num_threads = row.len().min(available_threads);

            ScopedTaskPool::new(num_threads, |sender| {
                let (barrier, lock) = JobBarrier::new(row.len());
                let barrier = Arc::new(Mutex::new(barrier));

                for system in &row {
                    let barrier = barrier.clone();

                    sender.send(move || {
                        system.run(world);

                        barrier.lock().unwrap().notify();
                    });
                }

                sender.join();

                lock.wait(barrier.lock().unwrap());
            });
        }
    }
}

pub struct SystemRunner(Box<dyn Runner>);

impl SystemRunner {
    pub fn new<S: Runner>(runner: S) -> SystemRunner {
        Self(Box::new(runner))
    }

    pub fn run(&self, systems: &Systems, world: &World) {
        self.0.run(&systems.graph, world)
    }
}
