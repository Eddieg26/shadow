use crate::io::{vfs::VirtualFileSystem, AssetIo};
use events::AssetEvents;
use library::AssetLibrary;
use loaders::AssetLoaders;
use registry::AssetRegistry;
use shadow_ecs::core::Resource;
use state::AssetStates;
use std::{
    path::Path,
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub mod events;
pub mod library;
pub mod loaders;
pub mod observers;
pub mod registry;
pub mod state;

#[derive(Clone)]
pub struct AssetDatabase {
    io: Arc<AssetIo>,
    loaders: Arc<RwLock<AssetLoaders>>,
    registry: Arc<RwLock<AssetRegistry>>,
    library: Arc<RwLock<AssetLibrary>>,
    states: Arc<RwLock<AssetStates>>,
    events: Arc<Mutex<AssetEvents>>,
}

impl AssetDatabase {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            io: Arc::new(AssetIo::new(root, VirtualFileSystem::new())),
            loaders: Arc::new(RwLock::new(AssetLoaders::new())),
            registry: Arc::new(RwLock::new(AssetRegistry::new())),
            library: Arc::new(RwLock::new(AssetLibrary::new())),
            states: Arc::new(RwLock::new(AssetStates::new())),
            events: Arc::new(Mutex::new(AssetEvents::new())),
        }
    }

    pub fn io(&self) -> &AssetIo {
        &self.io
    }

    pub fn loaders(&self) -> RwLockReadGuard<AssetLoaders> {
        self.loaders.read().unwrap()
    }

    pub fn registry(&self) -> RwLockReadGuard<AssetRegistry> {
        self.registry.read().unwrap()
    }

    pub fn library(&self) -> RwLockReadGuard<AssetLibrary> {
        self.library.read().unwrap()
    }

    pub fn states(&self) -> RwLockReadGuard<AssetStates> {
        self.states.read().unwrap()
    }

    pub(crate) fn loaders_mut(&self) -> RwLockWriteGuard<AssetLoaders> {
        self.loaders.write().unwrap()
    }

    pub(crate) fn registry_mut(&self) -> RwLockWriteGuard<AssetRegistry> {
        self.registry.write().unwrap()
    }

    pub(crate) fn library_mut(&self) -> RwLockWriteGuard<AssetLibrary> {
        self.library.write().unwrap()
    }

    pub(crate) fn states_mut(&self) -> RwLockWriteGuard<AssetStates> {
        self.states.write().unwrap()
    }

    pub(crate) fn events(&self) -> MutexGuard<AssetEvents> {
        self.events.lock().unwrap()
    }
}

impl Resource for AssetDatabase {}
