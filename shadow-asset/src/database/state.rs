use crate::asset::{AssetId, AssetType};
use shadow_ecs::core::DenseMap;
use std::collections::HashSet;

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
}

pub struct AssetStates {
    states: DenseMap<AssetId, AssetState>,
}

impl AssetStates {
    pub fn new() -> Self {
        Self {
            states: DenseMap::new(),
        }
    }

    pub fn get(&self, id: &AssetId) -> Option<&AssetState> {
        self.states.get(id)
    }

    pub fn load(&mut self, id: AssetId, state: AssetState) {
        self.states.insert(id, state);
    }

    pub fn unload(&mut self, id: &AssetId) -> Option<AssetState> {
        self.states.remove(id)
    }

    pub fn loaded(&self) -> &[AssetId] {
        self.states.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &AssetState)> {
        self.states.iter()
    }

    pub fn dependents<'a>(&self, id: &AssetId) -> HashSet<AssetId> {
        let mut dependents = HashSet::new();

        for (state_id, state) in self.states.iter() {
            if state.dependencies().contains(id) {
                dependents.insert(*state_id);
            }
        }

        dependents
    }
}
