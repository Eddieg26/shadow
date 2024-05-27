use crate::asset::{Asset, AssetDependency, AssetId, AssetType};
use shadow_ecs::ecs::{core::Resource, storage::dense::DenseSet};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStatus {
    Unloaded,
    Loading,
    Loaded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetEntry {
    ty: AssetType,
    status: AssetStatus,
}

impl AssetEntry {
    pub fn new<A: Asset>(status: AssetStatus) -> Self {
        Self {
            ty: AssetType::of::<A>(),
            status,
        }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn status(&self) -> AssetStatus {
        self.status
    }

    pub fn set_status(&mut self, status: AssetStatus) {
        self.status = status;
    }
}

pub struct LoadedResult {
    pub finished: Vec<AssetDependency>,
    pub unloaded: Vec<AssetDependency>,
    pub loading: DenseSet<AssetId>,
}

impl LoadedResult {
    pub fn new() -> Self {
        Self {
            finished: vec![],
            unloaded: vec![],
            loading: DenseSet::new(),
        }
    }
}

#[derive(Clone)]
pub struct AssetDatabase {
    assets: Arc<RwLock<HashMap<AssetId, AssetEntry>>>,
    path_map: Arc<RwLock<HashMap<PathBuf, AssetId>>>,
    dependencies: Arc<RwLock<HashMap<AssetId, DenseSet<AssetId>>>>,
    dependents: Arc<RwLock<HashMap<AssetId, DenseSet<AssetId>>>>,
}

impl AssetDatabase {
    pub fn new() -> Self {
        Self {
            assets: Arc::new(RwLock::new(HashMap::new())),
            path_map: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            dependents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_entry(&self, id: &AssetId) -> Option<AssetEntry> {
        self.assets.read().unwrap().get(id).copied()
    }

    pub fn get_status(&self, id: &AssetId) -> AssetStatus {
        self.get_entry(id)
            .map(|entry| entry.status())
            .unwrap_or(AssetStatus::Unloaded)
    }

    pub fn load<A: Asset>(&self, id: AssetId) {
        self.assets
            .write()
            .unwrap()
            .insert(id, AssetEntry::new::<A>(AssetStatus::Loading));
    }

    pub fn loaded(&self, id: AssetId, dependencies: Vec<AssetDependency>) -> LoadedResult {
        self.set_status(id, AssetStatus::Loaded);
        let mut result = LoadedResult::new();

        for dependency in dependencies {
            let status = self.get_status(&dependency.id());
            if status == AssetStatus::Unloaded || status == AssetStatus::Loading {
                result.unloaded.push(dependency);
            }

            let mut dependents = self.dependents.write().unwrap();
            dependents
                .entry(dependency.id())
                .or_insert_with(DenseSet::new)
                .insert(id);
        }

        if result.unloaded.is_empty() && result.loading.is_empty() {
            let entry = self.get_entry(&id).unwrap();
            result.finished.push(AssetDependency::new(id, entry.ty()));
            let mut dependents = self.dependents.write().unwrap();
            if let Some(dependents) = dependents.remove(&id) {
                for dependent in dependents.iter() {
                    let mut dependencies = self.dependencies.write().unwrap();
                    if let Some(set) = dependencies.get_mut(dependent) {
                        set.remove(&id);
                        if set.is_empty() {
                            dependencies.remove(dependent);
                            let entry = self.get_entry(dependent).unwrap();
                            result.finished.push(AssetDependency::new(id, entry.ty()));
                        }
                    }
                }
            }
        } else {
            let unloaded = result.unloaded.iter().map(|dep| dep.id()).collect();
            self.dependencies.write().unwrap().insert(id, unloaded);
        }

        result
    }

    pub fn failed(&self, id: AssetId) {
        self.set_status(id, AssetStatus::Failed);
    }

    pub fn unload(&self, id: AssetId) -> Option<AssetEntry> {
        self.assets.write().unwrap().remove(&id)
    }

    fn set_status(&self, id: AssetId, status: AssetStatus) {
        self.assets
            .write()
            .unwrap()
            .entry(id)
            .and_modify(|entry| entry.set_status(status));
    }

    pub fn set_path_id(&self, path: PathBuf, id: AssetId) {
        self.path_map.write().unwrap().insert(path, id);
    }

    pub fn get_path_id(&self, path: &PathBuf) -> Option<AssetId> {
        self.path_map.read().unwrap().get(path).copied()
    }
}

impl Resource for AssetDatabase {}
