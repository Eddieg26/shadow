use super::{
    graph::{Graph, GraphNode},
    IntoSystem, System,
};
use crate::ecs::{
    task::{max_thread_count, ScopedTaskPool},
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

pub struct SystemGraph {
    graph: Graph<System>,
}

impl SystemGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    pub fn add_system<M>(&mut self, system: impl IntoSystem<M>) -> usize {
        let mut system = system.into_system();

        let (before, mut after) = system.systems();

        let after_ids = after
            .drain(..)
            .map(|s| self.add_system(s))
            .collect::<Vec<_>>();

        let id = self.graph.insert(system);

        after_ids
            .iter()
            .for_each(|a| self.graph.add_dependency(id, *a));

        for before in before {
            let before_id = self.graph.insert(before);
            self.graph.add_dependency(before_id, id);
        }

        id
    }

    pub fn build(&mut self) {
        self.graph.build();
    }
}

impl std::ops::Deref for SystemGraph {
    type Target = Graph<System>;

    fn deref(&self) -> &Self::Target {
        &self.graph
    }
}

impl std::fmt::Display for SystemGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.graph.fmt(f)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RunMode {
    Sequential,
    Parallel,
}

pub trait Runner: Send + Sync + 'static {
    fn run(&self, graph: &Graph<System>, world: &World);
}

pub struct SequentialRunner;

impl Runner for SequentialRunner {
    fn run(&self, graph: &Graph<System>, world: &World) {
        for system in graph.nodes() {
            system.run(world);
        }
    }
}

pub struct ParallelRunner;

impl Runner for ParallelRunner {
    fn run(&self, graph: &Graph<System>, world: &World) {
        let available_threads = max_thread_count();
        for row in graph.iter() {
            let num_threads = row.len().min(available_threads);

            let mut pool = ScopedTaskPool::new(num_threads);
            for system in row {
                pool.spawn(move || system.run(world));
            }

            pool.run();
        }
    }
}

pub struct SystemRunner(Box<dyn Runner>);

impl SystemRunner {
    pub fn new<S: Runner>(runner: S) -> SystemRunner {
        Self(Box::new(runner))
    }

    pub fn run(&self, systems: &SystemGraph, world: &World) {
        self.0.run(&systems.graph, world)
    }
}
