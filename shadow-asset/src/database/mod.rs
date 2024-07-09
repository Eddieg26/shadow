use config::AssetConfig;
use library::{AssetLibraryRef, AssetLibraryRefMut, AssetLibraryShared};
use shadow_ecs::ecs::core::Resource;
use std::sync::Arc;
use task::{AssetQueueRef, AssetQueueShared, AssetTaskExecutor};

pub mod config;
pub mod library;
pub mod observers;
pub mod registry;
pub mod task;

#[derive(Clone)]
pub struct AssetDatabase {
    config: Arc<AssetConfig>,
    library: AssetLibraryShared,
    tasks: AssetQueueShared,
}

impl AssetDatabase {
    pub fn new(config: AssetConfig) -> AssetDatabase {
        AssetDatabase {
            config: Arc::new(config),
            library: Arc::default(),
            tasks: Arc::default(),
        }
    }

    pub fn config(&self) -> &AssetConfig {
        &self.config
    }

    pub fn library(&self) -> AssetLibraryRef {
        AssetLibraryRef::from(self.library.read().unwrap())
    }

    pub(crate) fn library_mut(&self) -> AssetLibraryRefMut {
        AssetLibraryRefMut::from(self.library.write().unwrap())
    }

    pub(crate) fn tasks(&self) -> AssetQueueRef {
        self.tasks.lock().unwrap()
    }

    pub(crate) fn pop_task(&self) -> Option<Box<dyn AssetTaskExecutor>> {
        self.tasks.lock().unwrap().pop()
    }
}

impl Resource for AssetDatabase {}
