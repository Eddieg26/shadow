use super::AssetId;
use events::AssetEvents;
use importer::AssetImporters;
use shadow_ecs::core::Resource;
use status::{AssetStatus, AssetTracker};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub mod events;
pub mod importer;
pub mod observers;
pub mod status;

#[derive(Clone)]
pub struct AssetDatabase {
    library: Arc<RwLock<AssetLibrary>>,
    importers: Arc<RwLock<AssetImporters>>,
    events: Arc<Mutex<AssetEvents>>,
    tracker: Arc<RwLock<AssetTracker>>,
}

impl AssetDatabase {
    pub fn new() -> Self {
        Self {
            library: Arc::new(RwLock::new(AssetLibrary::new())),
            importers: Arc::new(RwLock::new(AssetImporters::new())),
            events: Arc::new(Mutex::new(AssetEvents::new())),
            tracker: Arc::new(RwLock::new(AssetTracker::new())),
        }
    }

    pub fn library(&self) -> RwLockReadGuard<AssetLibrary> {
        self.library.read().unwrap()
    }

    pub fn importers(&self) -> RwLockReadGuard<AssetImporters> {
        self.importers.read().unwrap()
    }

    pub fn status(&self, id: &AssetId) -> AssetStatus {
        self.tracker.read().unwrap().status(id)
    }

    pub(crate) fn library_mut(&self) -> RwLockWriteGuard<AssetLibrary> {
        self.library.write().unwrap()
    }

    pub(crate) fn events(&self) -> std::sync::MutexGuard<AssetEvents> {
        self.events.lock().unwrap()
    }

    pub(crate) fn tracker(&self) -> RwLockReadGuard<AssetTracker> {
        self.tracker.read().unwrap()
    }

    pub(crate) fn tracker_mut(&self) -> RwLockWriteGuard<AssetTracker> {
        self.tracker.write().unwrap()
    }
}

impl Resource for AssetDatabase {}

pub struct AssetLibrary {
    ids: HashMap<PathBuf, AssetId>,
    paths: HashMap<AssetId, PathBuf>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        AssetLibrary {
            ids: HashMap::new(),
            paths: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: AssetId, path: PathBuf) -> (Option<AssetId>, Option<PathBuf>) {
        let ret_id = self.ids.insert(path.clone(), id);
        let ret_path = self.paths.insert(id, path);

        (ret_id, ret_path)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<PathBuf> {
        let path = self.paths.remove(id)?;
        self.ids.remove(&path);

        Some(path)
    }

    pub fn id_path(&self, id: &AssetId) -> Option<&PathBuf> {
        self.paths.get(id)
    }

    pub fn path_id(&self, path: &Path) -> Option<&AssetId> {
        self.ids.get(path)
    }
}
