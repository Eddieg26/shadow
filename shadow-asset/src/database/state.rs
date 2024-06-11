use super::{
    library::{AssetLibrary, AssetStatus},
    queue::AssetQueue,
};
use crate::asset::{Asset, AssetId, AssetPath, AssetType};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Clone)]
pub struct DatabaseState {
    asset_status: Arc<RwLock<HashMap<AssetId, AssetStatus>>>,
    library: AssetLibrary,
    queue: AssetQueue,
}

impl DatabaseState {
    pub(super) fn new() -> Self {
        DatabaseState {
            asset_status: Arc::new(RwLock::new(HashMap::new())),
            library: AssetLibrary::new(),
            queue: AssetQueue::new(),
        }
    }

    pub fn status(&self, path: impl Into<AssetPath>) -> AssetStatus {
        let path: AssetPath = path.into();
        let id = match path {
            AssetPath::Id(id) => Some(id),
            AssetPath::Path(path) => self.library.source(&path).map(|source| source.id()),
        };

        let asset_status = self.asset_status.read().unwrap();

        match id {
            Some(id) => *asset_status.get(&id).unwrap_or(&AssetStatus::None),
            None => AssetStatus::None,
        }
    }

    pub(super) fn library(&self) -> &AssetLibrary {
        &self.library
    }

    pub(crate) fn enqueue_import<A: Asset>(&self, path: impl Into<AssetPath>) {
        self.queue.add_import(path.into());
    }

    pub(crate) fn enqueue_load<A: Asset>(&self, path: impl Into<AssetPath>) {
        self.queue.add_load(path.into(), AssetType::of::<A>());
    }

    pub(crate) fn set_asset_status(&self, id: AssetId, status: AssetStatus) {
        self.asset_status.write().unwrap().insert(id, status);
    }
}
