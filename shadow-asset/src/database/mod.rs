use crate::asset::AssetId;
use config::AssetConfig;
use events::{ImportAsset, ImportFolder};
use library::AssetLibrary;
use shadow_ecs::ecs::{core::Resource, event::Events};
use std::path::{Path, PathBuf};
use tracker::{AssetTracker, ImportStatus, LoadStatus};

pub mod config;
pub mod events;
pub mod library;
pub mod observers;
pub mod pipeline;
pub mod tracker;

#[derive(Clone)]
pub struct AssetDatabase {
    config: AssetConfig,
    events: Events,
    library: AssetLibrary,
    load_tracker: AssetTracker<LoadStatus>,
    import_tracker: AssetTracker<ImportStatus>,
}

impl AssetDatabase {
    pub fn new(config: AssetConfig, events: &Events) -> Self {
        AssetDatabase {
            config,
            events: events.clone(),
            library: AssetLibrary::new(),
            load_tracker: AssetTracker::new(),
            import_tracker: AssetTracker::new(),
        }
    }

    pub fn config(&self) -> &AssetConfig {
        &self.config
    }

    pub fn library(&self) -> &AssetLibrary {
        &self.library
    }

    pub fn load_status(&self, id: &AssetId) -> LoadStatus {
        self.load_tracker.status(id)
    }

    pub fn import_status(&self, path: &PathBuf) -> ImportStatus {
        let path = path.into();
        self.import_tracker.status(&path)
    }

    pub fn import(&self, path: impl AsRef<Path>) {
        self.events.add(ImportAsset::new(path));
    }

    pub fn import_folder(&self, path: impl Into<PathBuf>) {
        self.events.add(ImportFolder::new(path));
    }
}

impl AssetDatabase {
    fn set_load_status(&self, id: AssetId, status: LoadStatus) {
        self.load_tracker.set_status(id, status);
    }

    fn clear_load_statuses(&self) {
        self.load_tracker.clear();
    }

    fn set_import_status(&self, path: PathBuf, status: ImportStatus) {
        let path = path.into();
        self.import_tracker.set_status(path, status);
    }

    fn clear_import_statuses(&self) {
        self.import_tracker.clear();
    }
}

impl Resource for AssetDatabase {}
