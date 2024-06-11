use crate::{
    asset::{AssetPath, AssetType},
    database::events::{ImportAsset, LoadAsset},
    loader::AssetLoader,
};
use shadow_ecs::ecs::{core::Resource, event::Events};
use std::collections::HashMap;

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

pub struct AssetLoaderRegistry {
    metas: HashMap<AssetType, AssetLoaderMeta>,
    ext_map: HashMap<&'static str, AssetType>,
}

impl AssetLoaderRegistry {
    pub fn new() -> Self {
        AssetLoaderRegistry {
            metas: HashMap::new(),
            ext_map: HashMap::new(),
        }
    }

    pub fn register<L: AssetLoader>(&mut self) {
        let meta = AssetLoaderMeta::new::<L>();
        let exts = L::extensions();

        for &ext in exts {
            self.ext_map.insert(ext, meta.ty());
        }

        self.metas.insert(meta.ty(), meta);
    }

    pub fn meta(&self, ty: AssetType) -> Option<&AssetLoaderMeta> {
        self.metas.get(&ty)
    }

    pub fn meta_by_ext(&self, ext: &str) -> Option<&AssetLoaderMeta> {
        self.ext_map.get(ext).and_then(|ty| self.meta(*ty))
    }
}

impl Resource for AssetLoaderRegistry {}
