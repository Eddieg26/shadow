use super::{events::AssetUnloaded, state::AssetState};
use crate::asset::{Asset, AssetId, AssetType, Assets};
use shadow_ecs::{
    core::DenseMap,
    world::{event::ErasedEvent, World},
};

pub struct AssetMetadata {
    unloaded: fn(AssetId, AssetState, &World) -> Option<ErasedEvent>,
}

impl AssetMetadata {
    pub fn new<A: Asset>() -> Self {
        Self {
            unloaded: |id, state, world| {
                let assets = world.resource_mut::<Assets<A>>();
                let asset = assets.remove(&id)?;

                Some(AssetUnloaded::new(id, asset, state).into())
            },
        }
    }

    pub fn unloaded(&self, id: AssetId, state: AssetState, world: &World) -> Option<ErasedEvent> {
        (self.unloaded)(id, state, world)
    }
}

pub struct AssetRegistry {
    metadata: DenseMap<AssetType, AssetMetadata>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self {
            metadata: DenseMap::new(),
        }
    }

    pub fn register<A: Asset>(&mut self) {
        let asset_type = AssetType::of::<A>();
        self.metadata.insert(asset_type, AssetMetadata::new::<A>());
    }

    pub fn get_metadata(&self, asset_type: AssetType) -> Option<&AssetMetadata> {
        self.metadata.get(&asset_type)
    }

    pub fn get_metadata_mut(&mut self, asset_type: AssetType) -> Option<&mut AssetMetadata> {
        self.metadata.get_mut(&asset_type)
    }
}
