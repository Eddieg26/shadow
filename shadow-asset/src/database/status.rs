use crate::{AssetId, AssetType};
use shadow_ecs::storage::dense::DenseSet;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStatus {
    Unloaded,
    Loading,
    Loaded,
}

pub struct AssetState {
    status: AssetStatus,
    ty: AssetType,
    dependencies: HashSet<AssetId>,
}

impl AssetState {
    pub fn new(ty: AssetType, status: AssetStatus) -> Self {
        Self {
            status,
            ty,
            dependencies: HashSet::new(),
        }
    }

    pub fn unknown() -> Self {
        Self {
            status: AssetStatus::Unloaded,
            ty: AssetType::UNKNOWN,
            dependencies: HashSet::new(),
        }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn status(&self) -> AssetStatus {
        self.status
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn set_dependencies(&mut self, dependencies: HashSet<AssetId>) {
        self.dependencies = dependencies;
    }

    pub fn removed_dependency(&mut self, id: &AssetId) -> bool {
        self.dependencies.remove(id)
    }
}

impl Default for AssetState {
    fn default() -> Self {
        Self::unknown()
    }
}

pub struct AssetTracker {
    assets: HashMap<AssetId, AssetState>,
}

impl AssetTracker {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn state(&self, id: &AssetId) -> Option<&AssetState> {
        self.assets.get(id)
    }

    pub fn state_mut(&mut self, id: &AssetId) -> Option<&mut AssetState> {
        self.assets.get_mut(id)
    }

    pub fn status(&self, id: &AssetId) -> AssetStatus {
        self.state(id)
            .map_or(AssetStatus::Unloaded, |state| state.status())
    }

    pub fn load(&mut self, id: AssetId, ty: AssetType) {
        self.assets
            .insert(id, AssetState::new(ty, AssetStatus::Loading));
    }

    pub fn loaded<'a>(&mut self, id: AssetId, dependencies: HashSet<AssetId>) {
        match self.state_mut(&id) {
            Some(state) => {
                state.status = AssetStatus::Loaded;
                state.set_dependencies(dependencies);
            }
            None => panic!("Asset not found: {:?}", id),
        }
    }

    pub fn unload(&mut self, id: &AssetId) -> Option<AssetState> {
        self.assets.remove(id)
    }

    pub fn dependents<'a>(&self, ids: impl Iterator<Item = &'a AssetId>) -> DenseSet<AssetId> {
        let mut dependents = DenseSet::new();

        for id in ids {
            for state in self.assets.values() {
                if state.dependencies().contains(id) {
                    dependents.insert(*id);
                }
            }
        }

        dependents
    }
}

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

    pub fn contains(&self, id: &AssetId) -> bool {
        self.paths.contains_key(id)
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
