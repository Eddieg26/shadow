use super::{
    events::{AssetLoaded, AssetUnloaded},
    state::AssetState,
};
use crate::{
    asset::{Asset, AssetId, AssetType, Assets},
    loader::LoadedAsset,
};
use shadow_ecs::{
    core::DenseMap,
    world::{event::ErasedEvent, World},
};

pub struct AssetMetadata {
    loaded: fn(LoadedAsset) -> ErasedEvent,
    unloaded: fn(AssetId, AssetState, &World) -> Option<ErasedEvent>,
}

impl AssetMetadata {
    pub fn new<A: Asset>() -> Self {
        Self {
            loaded: |loaded: LoadedAsset| {
                let id = loaded.meta.id();
                let dependencies = loaded.meta.dependencies;
                let asset = loaded.asset.take::<A>();
                ErasedEvent::new(AssetLoaded::new(id, asset, dependencies))
            },
            unloaded: |id, state, world| {
                let assets = world.resource_mut::<Assets<A>>();
                let asset = assets.remove(&id)?;

                Some(AssetUnloaded::new(id, asset, state).into())
            },
        }
    }

    pub fn loaded(&self, loaded: LoadedAsset) -> ErasedEvent {
        (self.loaded)(loaded)
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

    pub fn has<A: Asset>(&self) -> bool {
        self.metadata.contains(&AssetType::of::<A>())
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
