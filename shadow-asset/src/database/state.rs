use crate::{AssetId, AssetType};
use shadow_ecs::storage::dense::DenseSet;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

pub struct AssetState {
    ty: AssetType,
    dependencies: HashSet<AssetId>,
}

impl AssetState {
    pub fn new(ty: AssetType, dependencies: HashSet<AssetId>) -> Self {
        Self { ty, dependencies }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn removed_dependency(&mut self, id: &AssetId) -> bool {
        self.dependencies.remove(id)
    }
}

pub struct AssetStates {
    assets: HashMap<AssetId, AssetState>,
}

impl AssetStates {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn loaded(&self, id: &AssetId) -> bool {
        self.assets.contains_key(id)
    }

    pub fn get(&self, id: &AssetId) -> Option<&AssetState> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut AssetState> {
        self.assets.get_mut(id)
    }

    pub fn load<'a>(&mut self, id: AssetId, ty: AssetType, dependencies: HashSet<AssetId>) {
        self.assets.insert(id, AssetState::new(ty, dependencies));
    }

    pub fn unload(&mut self, id: &AssetId) -> Option<AssetState> {
        self.assets.remove(id)
    }

    pub fn dependencies(&self, id: &AssetId) -> Option<&HashSet<AssetId>> {
        self.assets.get(id).map(|state| state.dependencies())
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
