use crate::{
    asset::{AssetId, AssetType},
    loader::AssetPipeline,
};
use shadow_ecs::ecs::core::Resource;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub trait BaseStorage: 'static {
    fn clear(&mut self);
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub struct AssetStorage<P: AssetPipeline> {
    items: HashMap<AssetId, (P::Asset, P::Settings)>,
}

impl<P: AssetPipeline> BaseStorage for AssetStorage<P> {
    fn clear(&mut self) {
        self.items.clear();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl<P: AssetPipeline> AssetStorage<P> {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        id: AssetId,
        data: (P::Asset, P::Settings),
    ) -> Option<(P::Asset, P::Settings)> {
        self.items.insert(id, data)
    }

    pub fn get(&self, id: &AssetId) -> Option<&(P::Asset, P::Settings)> {
        self.items.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut (P::Asset, P::Settings)> {
        self.items.get_mut(id)
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.items.contains_key(id)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<(P::Asset, P::Settings)> {
        self.items.remove(id)
    }
}

pub struct AssetStorages {
    storages: HashMap<AssetType, Arc<Mutex<Box<dyn BaseStorage>>>>,
}

impl AssetStorages {
    pub fn new() -> Self {
        Self {
            storages: HashMap::new(),
        }
    }

    pub fn register<P: AssetPipeline>(&mut self) {
        let ty = AssetType::of::<P::Asset>();
        let storage: Box<dyn BaseStorage> = Box::new(AssetStorage::<P>::new());
        self.storages.insert(ty, Arc::new(Mutex::new(storage)));
    }

    pub fn get<P: AssetPipeline>(&self) -> Option<Arc<Mutex<Box<dyn BaseStorage>>>> {
        let ty = AssetType::of::<P::Asset>();
        self.storages.get(&ty).cloned()
    }

    pub fn insert<P: AssetPipeline>(
        &self,
        id: AssetId,
        asset: P::Asset,
        settings: P::Settings,
    ) -> Option<(P::Asset, P::Settings)> {
        self.execute_mut::<P, (P::Asset, P::Settings)>(|storage| {
            storage.insert(id, (asset, settings))
        })
    }

    pub fn remove<P: AssetPipeline>(&self, id: AssetId) -> Option<(P::Asset, P::Settings)> {
        self.execute_mut::<P, (P::Asset, P::Settings)>(|storage| storage.remove(&id))
    }

    pub fn execute<P: AssetPipeline, U>(
        &self,
        callback: impl FnOnce(&AssetStorage<P>) -> Option<U>,
    ) -> Option<U> {
        let storage = self.get::<P>()?;
        let storage = storage.lock().ok()?;
        let storage = storage.as_any().downcast_ref::<AssetStorage<P>>()?;

        callback(&storage)
    }

    pub fn execute_mut<P: AssetPipeline, U>(
        &self,
        callback: impl FnOnce(&mut AssetStorage<P>) -> Option<U>,
    ) -> Option<U> {
        let storage = self.get::<P>()?;
        let mut storage = storage.lock().ok()?;
        let mut storage = storage.as_any_mut().downcast_mut::<AssetStorage<P>>()?;

        callback(&mut storage)
    }

    pub fn clear(&mut self) {
        for storage in self.storages.values() {
            let mut storage = match storage.lock() {
                Ok(storage) => storage,
                Err(_) => continue,
            };

            storage.clear();
        }
    }
}

impl Resource for AssetStorages {}
