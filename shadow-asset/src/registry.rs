use crate::{
    asset::{AssetId, AssetMetadata, AssetPath, AssetType},
    block::MetadataBlock,
    database::events::{ImportAsset, ImportReason, LoadAsset, ProcessAsset},
    loader::{AssetLoader, AssetPipeline},
};
use shadow_ecs::ecs::{
    core::Resource,
    event::{EventStorage, Events},
};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, RwLock},
};

#[derive(Clone, Copy)]
pub struct AssetPipelineMeta {
    ty: AssetType,
    import_event: fn(&mut EventStorage, ImportReason),
    load: fn(&Events, &AssetPath),
    load_meta: fn(&Path) -> std::io::Result<MetadataBlock>,
    process: fn(&Events, AssetId),
}

impl AssetPipelineMeta {
    pub fn new<L: AssetLoader>() -> Self {
        AssetPipelineMeta {
            ty: AssetType::of::<L::Asset>(),
            import_event: |events, reason| events.add(ImportAsset::<L::Asset>::with_reason(reason)),
            load: |events, path| events.add(LoadAsset::<L::Asset>::new(path)),
            load_meta: |path| {
                let path = path.with_extension("meta");
                let bytes = std::fs::read_to_string(&path)?;
                let metadata = match toml::from_str::<AssetMetadata<L::Settings>>(&bytes) {
                    Ok(data) => Ok(data),
                    Err(e) => {
                        println!("Failed to parse metadata: {:?}", e);
                        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    }
                }?;
                Ok(MetadataBlock::new(metadata.id(), bytes.as_bytes().to_vec()))
            },
            process: |events, id| events.add(ProcessAsset::<L::Asset>::new(id)),
        }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn import_event(&self, events: &mut EventStorage, reason: ImportReason) {
        (self.import_event)(events, reason)
    }

    pub fn load(&self, events: &Events, path: &AssetPath) {
        (self.load)(events, path);
    }

    pub fn load_meta(&self, path: &Path) -> std::io::Result<MetadataBlock> {
        (self.load_meta)(path)
    }

    pub fn process(&self, events: &Events, id: AssetId) {
        (self.process)(events, id);
    }
}

#[derive(Clone)]
pub struct AssetRegistry {
    metas: Arc<RwLock<HashMap<AssetType, AssetPipelineMeta>>>,
    ext_map: Arc<RwLock<HashMap<&'static str, AssetType>>>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        AssetRegistry {
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

impl Resource for AssetRegistry {}
