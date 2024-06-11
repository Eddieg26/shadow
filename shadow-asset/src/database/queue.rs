use crate::asset::{AssetPath, AssetType};
use std::sync::{Arc, RwLock};

pub struct AssetQueueInner {
    loads: Vec<(AssetPath, AssetType)>,
    imports: Vec<AssetPath>,
}

impl AssetQueueInner {
    pub fn new() -> Self {
        AssetQueueInner {
            loads: Vec::new(),
            imports: Vec::new(),
        }
    }

    pub fn add_load(&mut self, path: AssetPath, asset_type: AssetType) {
        self.loads.push((path, asset_type));
    }

    pub fn add_import(&mut self, path: AssetPath) {
        self.imports.push(path);
    }

    pub fn loads(&self) -> &[(AssetPath, AssetType)] {
        &self.loads
    }

    pub fn imports(&self) -> &[AssetPath] {
        &self.imports
    }
}

#[derive(Clone)]
pub struct AssetQueue(Arc<RwLock<AssetQueueInner>>);

impl AssetQueue {
    pub fn new() -> Self {
        AssetQueue(Arc::new(RwLock::new(AssetQueueInner::new())))
    }

    pub fn add_load(&self, path: AssetPath, asset_type: AssetType) {
        self.0.write().unwrap().add_load(path, asset_type);
    }

    pub fn add_import(&self, path: AssetPath) {
        self.0.write().unwrap().add_import(path);
    }

    pub fn drain_load_queue(&self) -> Vec<(AssetPath, AssetType)> {
        std::mem::take(&mut self.0.write().unwrap().loads)
    }

    pub fn drain_import_queue(&self) -> Vec<AssetPath> {
        std::mem::take(&mut self.0.write().unwrap().imports)
    }
}
