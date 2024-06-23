use super::events::ImportReason;
use crate::asset::AssetId;
use shadow_ecs::ecs::storage::dense::DenseMap;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

pub enum AssetAction {
    Load { id: AssetId },
    Import { reason: ImportReason },
    ImportFolder,
}

#[derive(Clone)]
pub struct AssetQueue {
    assets: Arc<RwLock<DenseMap<PathBuf, VecDeque<AssetAction>>>>,
}

impl AssetQueue {
    pub fn new() -> Self {
        AssetQueue {
            assets: Arc::default(),
        }
    }

    pub fn push(
        &self,
        path: impl AsRef<Path>,
        action: AssetAction,
    ) -> Option<VecDeque<AssetAction>> {
        let path = path.as_ref().to_path_buf();
        let mut assets = self.assets.write().ok()?;
        if let Some(actions) = assets.get_mut(&path) {
            actions.push_back(action);
            None
        } else {
            assets.insert(path, VecDeque::from(vec![action]))
        }
    }

    pub fn pop(&self, path: impl AsRef<Path>) -> Option<AssetAction> {
        let path = path.as_ref().to_path_buf();
        let mut assets = self.assets.write().ok()?;
        assets
            .get_mut(&path)
            .and_then(|actions| actions.pop_front())
    }

    pub fn is_empty(&self) -> bool {
        self.assets.read().unwrap().is_empty()
    }
}
