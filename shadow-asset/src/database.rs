use crate::{
    asset::{AssetId, AssetPath},
    config::AssetConfig,
    loader::AssetLoader,
    registry::AssetRegistry,
};
use shadow_ecs::ecs::{core::Resource, event::Events};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, RwLock},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetStatus {
    None,
    Importing,
    Imported,
    Loading,
    Loaded,
    Processing,
    Processed,
    Failed,
}

impl AssetStatus {
    pub fn none(&self) -> bool {
        matches!(self, AssetStatus::None)
    }

    pub fn unloaded(&self) -> bool {
        matches!(self, AssetStatus::None | AssetStatus::Failed)
    }

    pub fn importing(&self) -> bool {
        matches!(self, AssetStatus::Importing)
    }

    pub fn imported(&self) -> bool {
        matches!(self, AssetStatus::Imported)
    }

    pub fn loading(&self) -> bool {
        matches!(self, AssetStatus::Loading)
    }

    pub fn loaded(&self) -> bool {
        matches!(self, AssetStatus::Loaded)
    }

    pub fn processing(&self) -> bool {
        matches!(self, AssetStatus::Processing)
    }

    pub fn processed(&self) -> bool {
        matches!(self, AssetStatus::Processed)
    }

    pub fn failed(&self) -> bool {
        matches!(self, AssetStatus::Failed)
    }
}

#[derive(Clone)]
pub struct AssetDatabase {
    statuses: Arc<RwLock<HashMap<AssetId, AssetStatus>>>,
    imports: Arc<RwLock<HashSet<PathBuf>>>,
    registry: AssetRegistry,
    config: AssetConfig,
    events: Events,
}

impl AssetDatabase {
    pub fn new(config: &AssetConfig, events: &Events) -> Self {
        Self {
            statuses: Arc::default(),
            imports: Arc::default(),
            registry: AssetRegistry::new(),
            config: config.clone(),
            events: events.clone(),
        }
    }

    pub fn config(&self) -> &AssetConfig {
        &self.config
    }

    pub fn registery(&self) -> &AssetRegistry {
        &self.registry
    }

    pub fn register<L: AssetLoader>(&mut self) {
        self.registry.register::<L>();
    }

    pub fn status(&self, id: &AssetId) -> AssetStatus {
        self.statuses
            .read()
            .unwrap()
            .get(id)
            .copied()
            .unwrap_or(AssetStatus::None)
    }

    pub fn set_status(&self, id: &AssetId, status: AssetStatus) -> Option<AssetStatus> {
        self.statuses.write().unwrap().insert(*id, status)
    }

    pub(crate) fn add_import_path(&self, path: PathBuf) {
        self.imports.write().unwrap().insert(path);
    }

    pub(crate) fn remove_import_path(&self, path: &PathBuf) -> bool {
        self.imports.write().unwrap().remove(path)
    }

    pub(crate) fn is_importing_path(&self, path: &PathBuf) -> bool {
        self.imports.read().unwrap().contains(path)
    }
}

impl Resource for AssetDatabase {}

pub mod ext {
    use super::AssetDatabase;
    use crate::{
        asset::{Asset, AssetPath},
        events::import::ImportAsset,
    };

    pub trait AssetDatabaseExt {
        fn import<A: Asset>(&self, path: impl Into<AssetPath>);
    }

    impl AssetDatabaseExt for AssetDatabase {
        fn import<A: Asset>(&self, path: impl Into<AssetPath>) {
            self.events.add(ImportAsset::<A>::new(path));
        }
    }
}
