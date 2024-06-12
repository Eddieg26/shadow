use crate::{
    asset::{AssetPath, AssetType},
    database::events::{ImportAsset, LoadAsset},
    loader::AssetLoader,
};
use shadow_ecs::ecs::{core::Resource, event::Events};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Clone, Copy)]
pub struct AssetLoaderMeta {
    ty: AssetType,
    import: fn(&Events, &AssetPath),
    load: fn(&Events, &AssetPath),
}

impl AssetLoaderMeta {
    pub fn new<L: AssetLoader>() -> Self {
        AssetLoaderMeta {
            ty: AssetType::of::<L::Asset>(),
            import: |events, path| events.add(ImportAsset::<L::Asset>::new(path)),
            load: |events, path| events.add(LoadAsset::<L::Asset>::new(path)),
        }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn import(&self, events: &Events, path: &AssetPath) {
        (self.import)(events, path);
    }

    pub fn load(&self, events: &Events, path: &AssetPath) {
        (self.load)(events, path);
    }
}

#[derive(Clone)]
pub struct AssetLoaderRegistry {
    metas: Arc<RwLock<HashMap<AssetType, AssetLoaderMeta>>>,
    ext_map: Arc<RwLock<HashMap<&'static str, AssetType>>>,
}

impl AssetLoaderRegistry {
    pub fn new() -> Self {
        AssetLoaderRegistry {
            metas: Arc::default(),
            ext_map: Arc::default(),
        }
    }

    pub fn register<L: AssetLoader>(&mut self) {
        let meta = AssetLoaderMeta::new::<L>();
        let exts = L::extensions();

        let mut metas = self.metas.write().unwrap();
        let mut ext_map = self.ext_map.write().unwrap();

        for ext in exts {
            ext_map.insert(ext, meta.ty());
        }

        metas.insert(meta.ty(), meta);
    }

    pub fn meta(&self, ty: AssetType) -> Option<AssetLoaderMeta> {
        self.metas.read().unwrap().get(&ty).copied()
    }

    pub fn meta_by_ext(&self, ext: &str) -> Option<AssetLoaderMeta> {
        self.ext_map
            .read()
            .unwrap()
            .get(ext)
            .and_then(|ty| self.meta(*ty))
    }
}

impl Resource for AssetLoaderRegistry {}
