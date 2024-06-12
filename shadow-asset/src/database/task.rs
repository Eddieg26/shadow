use shadow_ecs::ecs::{event::Events, storage::dense::DenseMap};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DatabaseState {
    Importing,
    LoadingAssets,
    Saving,
    Loading,
    Ready,
}

pub trait DatabaseTask: Send + Sync + 'static {
    fn run(&self, events: &Events);
}

#[derive(Clone)]
pub struct DatabaseTasks {
    tasks: Arc<Mutex<DenseMap<DatabaseState, Vec<Box<dyn DatabaseTask>>>>>,
}

impl DatabaseTasks {
    pub fn new() -> Self {
        DatabaseTasks {
            tasks: Arc::default(),
        }
    }

    pub fn add<T: DatabaseTask>(&self, state: DatabaseState, task: T) {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(tasks) = tasks.get_mut(&state) {
            tasks.push(Box::new(task));
        } else {
            tasks.insert(state, vec![Box::new(task)]);
        }
    }

    pub fn pop(&self) -> Option<(DatabaseState, Vec<Box<dyn DatabaseTask>>)> {
        self.tasks.lock().unwrap().pop_front()
    }
}
