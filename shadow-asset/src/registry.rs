use crate::{
    asset::{Asset, AssetId, AssetMetadata, AssetPath, AssetType},
    bytes::AsBytes,
    events::import::ImportAsset,
    loader::AssetLoader,
};
use shadow_ecs::ecs::event::Events;
use std::{collections::HashMap, path::Path, sync::Arc};

pub struct AssetMeta {
    ty: AssetType,
    import: fn(&Events, &AssetPath),
    load_id: fn(&Path) -> Option<AssetId>,
}

impl AssetMeta {
    pub fn new<L: AssetLoader>() -> Self {
        Self {
            ty: AssetType::of::<L::Asset>(),
            import: |events, path| {
                events.add(ImportAsset::<L::Asset>::new(path));
            },
            load_id: |path| {
                let bytes = std::fs::read(path).ok()?;
                let metadata = AssetMetadata::<L::Settings>::from_bytes(&bytes)?;
                Some(metadata.id().clone())
            },
        }
    }

    pub fn ty(&self) -> &AssetType {
        &self.ty
    }

    pub fn import(&self, events: &Events, path: &AssetPath) {
        (self.import)(events, path);
    }

    pub fn load_id(&self, path: &Path) -> Option<AssetId> {
        (self.load_id)(path)
    }
}

#[derive(Default, Clone)]
pub struct AssetRegistry {
    registry: HashMap<AssetType, Arc<AssetMeta>>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self {
            registry: HashMap::default(),
        }
    }

    pub fn register<L: AssetLoader>(&mut self) {
        let meta = AssetMeta::new::<L>();
        let ty = meta.ty.clone();
        self.registry.insert(ty, Arc::new(meta));
    }

    pub fn meta<A: Asset>(&self) -> Arc<AssetMeta> {
        self.registry.get(&AssetType::of::<A>()).cloned().expect("Asset not registered.")
    }

    pub fn meta_dyn(&self, ty: &AssetType) -> Arc<AssetMeta> {
        self.registry.get(&ty).cloned().expect("Asset not registered")
    }
}
