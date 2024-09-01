use crate::asset::{Asset, AssetId, AssetType};
use ecs::core::DenseMap;
use std::collections::HashSet;

pub struct AssetState {
    ty: AssetType,
    dependencies: HashSet<AssetId>,
    parent: Option<AssetId>,
}

impl AssetState {
    pub fn new<A: Asset>(dependencies: HashSet<AssetId>, parent: Option<AssetId>) -> Self {
        Self {
            ty: AssetType::of::<A>(),
            dependencies,
            parent,
        }
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

    pub fn is_loaded(&self, id: &AssetId) -> bool {
        self.states.contains(id)
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

    pub fn dependents(&self, id: &AssetId) -> HashSet<AssetId> {
        let mut dependents = HashSet::new();

        for (state_id, state) in self.states.iter() {
            if state.dependencies().contains(id) {
                dependents.insert(*state_id);
            }
        }

        dependents
    }

    pub fn children(&self, id: &AssetId) -> HashSet<AssetId> {
        let mut children = HashSet::new();

        for (state_id, state) in self.states.iter() {
            if state.parent == Some(*id) {
                children.insert(*state_id);
            }
        }

        children
    }

    pub fn children_and_dependents(&self, id: &AssetId) -> (HashSet<AssetId>, HashSet<AssetId>) {
        let mut children = HashSet::new();
        let mut dependents = HashSet::new();

        for (state_id, state) in self.states.iter() {
            if state.dependencies().contains(id) {
                dependents.insert(*state_id);
                continue;
            }

            if state.parent == Some(*id) {
                children.insert(*state_id);
            }
        }

        (children, dependents)
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn clear(&mut self) {
        self.states.clear();
    }
}
