use crate::{
    asset::{AssetPath, AssetType},
    database::{
        events::{ImportAsset, ImportReason, LoadAsset},
        AssetDatabase,
    },
    loader::{AssetLoader, AssetPipeline},
};
use shadow_ecs::ecs::{core::Resource, event::Events};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Clone, Copy)]
pub struct AssetPipelineMeta {
    ty: AssetType,
    import: fn(&AssetDatabase, &Events, ImportReason),
    load: fn(&Events, &AssetPath),
}

impl AssetPipelineMeta {
    pub fn new<L: AssetLoader>() -> Self {
        AssetPipelineMeta {
            ty: AssetType::of::<L::Asset>(),
            import: |db, events, reason: ImportReason| {
                match reason.path() {
                    AssetPath::Id(id) => match db.block(&id) {
                        Some(info) => {
                            let path = info.filepath().to_path_buf();
                            events.add(ImportAsset::<L::Asset>::new(path).with_reason(reason))
                        }
                        None => {}
                    },
                    AssetPath::Path(path) => {
                        events.add(ImportAsset::<L::Asset>::new(path).with_reason(reason))
                    }
                };
            },
            load: |events, path| events.add(LoadAsset::<L::Asset>::new(path)),
        }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn import(&self, db: &AssetDatabase, events: &Events, reason: ImportReason) {
        (self.import)(db, events, reason);
    }

    pub fn load(&self, events: &Events, path: &AssetPath) {
        (self.load)(events, path);
    }
}

#[derive(Clone)]
pub struct AssetPipelineRegistry {
    metas: Arc<RwLock<HashMap<AssetType, AssetPipelineMeta>>>,
    ext_map: Arc<RwLock<HashMap<&'static str, AssetType>>>,
}

impl AssetPipelineRegistry {
    pub fn new() -> Self {
        AssetPipelineRegistry {
            metas: Arc::default(),
            ext_map: Arc::default(),
        }
    }

    pub fn register<P: AssetPipeline>(&mut self) {
        let meta = AssetPipelineMeta::new::<P::Loader>();
        let exts = P::Loader::extensions();

        let mut metas = self.metas.write().unwrap();
        let mut ext_map = self.ext_map.write().unwrap();

        for ext in exts {
            ext_map.insert(ext, meta.ty());
        }

        metas.insert(meta.ty(), meta);
    }

    pub fn meta(&self, ty: AssetType) -> Option<AssetPipelineMeta> {
        self.metas.read().unwrap().get(&ty).copied()
    }

    pub fn meta_by_ext(&self, ext: &str) -> Option<AssetPipelineMeta> {
        self.ext_map
            .read()
            .unwrap()
            .get(ext)
            .and_then(|ty| self.meta(*ty))
    }
}

impl Resource for AssetPipelineRegistry {}
