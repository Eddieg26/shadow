use crate::{AssetId, AssetType};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStatus {
    Unloaded,
    Loading,
    Loaded,
}

pub struct AssetState {
    status: AssetStatus,
    ty: AssetType,
    dependents: Vec<AssetId>,
    dependencies: Vec<AssetId>,
}

impl AssetState {
    pub fn new(ty: AssetType, status: AssetStatus) -> Self {
        Self {
            status,
            ty,
            dependents: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn set_dependencies(&mut self, dependencies: Vec<AssetId>) {
        self.dependencies = dependencies;
    }

    pub fn add_dependent(&mut self, id: AssetId) {
        self.dependents.push(id);
    }

    pub fn remove_dependent(&mut self, id: &AssetId) {
        self.dependents.retain(|&i| i != *id);
    }

    pub fn removed_dependency(&mut self, id: &AssetId) {
        self.dependencies.retain(|&i| i != *id);
    }

    pub fn status(&self) -> AssetStatus {
        self.status
    }

    pub fn dependents(&self) -> &[AssetId] {
        &self.dependents
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
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

    pub fn get(&self, id: &AssetId) -> Option<&AssetState> {
        self.assets.get(id)
    }

    pub fn status(&self, id: &AssetId) -> AssetStatus {
        self.get(id)
            .map_or(AssetStatus::Unloaded, |state| state.status())
    }

    pub fn unload(&mut self, id: &AssetId) -> Option<AssetState> {
        let state = self.assets.remove(id)?;

        for dependency in state.dependencies() {
            if let Some(dependent) = self.assets.get_mut(dependency) {
                dependent.remove_dependent(id);
            }
        }

        for dependent in state.dependents() {
            if let Some(dependency) = self.assets.get_mut(dependent) {
                dependency.removed_dependency(id);
            }
        }

        Some(state)
    }

    pub fn load(&mut self, id: AssetId) {
        match self.assets.get_mut(&id) {
            Some(state) => {
                state.status = AssetStatus::Loading;
            }
            None => {
                let state = AssetState::new(AssetType::UNKNOWN, AssetStatus::Loading);
                self.assets.insert(id, state);
            }
        }
    }

    pub fn loaded<'a>(
        &mut self,
        id: AssetId,
        ty: AssetType,
        dependencies: impl IntoIterator<Item = &'a AssetId>,
    ) {
        let dependencies = dependencies
            .into_iter()
            .filter_map(|dep| {
                let state = self.assets.get_mut(dep)?;
                state.add_dependent(id);
                Some(*dep)
            })
            .collect::<Vec<_>>();

        match self.assets.get_mut(&id) {
            Some(state) => {
                state.status = AssetStatus::Loaded;
                state.ty = ty;
                state.set_dependencies(dependencies);
            }
            None => {
                let mut state = AssetState::new(ty, AssetStatus::Loaded);
                state.set_dependencies(dependencies);
                self.assets.insert(id, state);
            }
        }
    }
}
