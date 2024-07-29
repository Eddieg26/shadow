use super::AssetId;
use crate::{AssetConfig, AssetFileSystem, LocalFileSystem};
use events::AssetEvents;
use importer::AssetImporters;
use shadow_ecs::core::Resource;
use state::{AssetLibrary, AssetStates};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub mod events;
pub mod importer;
pub mod observers;
pub mod state;

#[derive(Clone)]
pub struct AssetDatabase {
    filesystem: AssetFileSystem,
    library: Arc<RwLock<AssetLibrary>>,
    states: Arc<RwLock<AssetStates>>,
    importers: Arc<RwLock<AssetImporters>>,
    events: Arc<Mutex<AssetEvents>>,
}

impl AssetDatabase {
    pub fn new() -> Self {
        Self {
            filesystem: AssetFileSystem::new(AssetConfig::new(), LocalFileSystem),
            library: Arc::new(RwLock::new(AssetLibrary::new())),
            states: Arc::new(RwLock::new(AssetStates::new())),
            importers: Arc::new(RwLock::new(AssetImporters::new())),
            events: Arc::new(Mutex::new(AssetEvents::new())),
        }
    }

    pub fn filesystem(&self) -> &AssetFileSystem {
        &self.filesystem
    }

    pub fn library(&self) -> RwLockReadGuard<AssetLibrary> {
        self.library.read().unwrap()
    }

    pub fn states(&self) -> RwLockReadGuard<AssetStates> {
        self.states.read().unwrap()
    }

    pub fn importers(&self) -> RwLockReadGuard<AssetImporters> {
        self.importers.read().unwrap()
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.library().contains(id)
    }

    pub fn loaded(&self, id: &AssetId) -> bool {
        self.states().loaded(id)
    }

    pub fn asset_path(&self, id: &AssetId) -> Option<PathBuf> {
        self.library().id_path(id).cloned()
    }

    pub fn path_id(&self, path: &Path) -> Option<AssetId> {
        self.library().path_id(path).copied()
    }

    pub fn dependencies(&self, id: &AssetId) -> Option<Vec<AssetId>> {
        let states = self.states();
        states
            .dependencies(id)
            .map(|deps| deps.iter().copied().collect())
    }

    pub(crate) fn library_mut(&self) -> RwLockWriteGuard<AssetLibrary> {
        self.library.write().unwrap()
    }

    pub(crate) fn states_mut(&self) -> RwLockWriteGuard<AssetStates> {
        self.states.write().unwrap()
    }

    pub(crate) fn importers_mut(&self) -> RwLockWriteGuard<AssetImporters> {
        self.importers.write().unwrap()
    }

    pub(crate) fn events(&self) -> std::sync::MutexGuard<AssetEvents> {
        self.events.lock().unwrap()
    }
}

impl Resource for AssetDatabase {}
