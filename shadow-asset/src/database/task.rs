use super::events::LoadAsset;
use crate::asset::Asset;
use shadow_ecs::ecs::{event::Events, storage::dense::DenseMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DatabaseStatus {
    Importing,
    LoadingAssets,
    Saving,
    Loading,
    Ready,
}

pub trait DatabaseTask: 'static {
    fn status(&self) -> DatabaseStatus;
    fn run(&self, events: &Events);
}

impl<A: Asset> DatabaseTask for LoadAsset<A> {
    fn status(&self) -> DatabaseStatus {
        DatabaseStatus::LoadingAssets
    }

    fn run(&self, events: &Events) {
        events.add(LoadAsset::<A>::new(self.path().clone()));
    }
}

pub struct DatabaseTasks {
    tasks: DenseMap<DatabaseStatus, Vec<Box<dyn DatabaseTask>>>,
}

impl DatabaseTasks {
    pub fn new() -> Self {
        DatabaseTasks {
            tasks: DenseMap::new(),
        }
    }

    pub fn add<T: DatabaseTask>(&mut self, task: T) {
        let status = task.status();
        if let Some(tasks) = self.tasks.get_mut(&status) {
            tasks.push(Box::new(task));
        } else {
            self.tasks.insert(status, vec![Box::new(task)]);
        }
    }

    pub fn pop(&mut self) -> Option<(DatabaseStatus, Vec<Box<dyn DatabaseTask>>)> {
        self.tasks.pop_front()
    }
}
