use crate::{
    asset::{AssetMetadata, AssetPath, AssetType},
    block::MetadataBlock,
    database::{
        events::{ImportAsset, ImportReason, LoadAsset},
        AssetDatabase,
    },
    loader::{AssetLoader, AssetPipeline},
};
use shadow_ecs::ecs::{core::Resource, event::Events};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, RwLock},
};

#[derive(Clone, Copy)]
pub struct AssetPipelineMeta {
    ty: AssetType,
    import: fn(&AssetDatabase, &Events, ImportReason),
    load: fn(&Events, &AssetPath),
    load_meta: fn(&Path) -> std::io::Result<MetadataBlock>,
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

    pub fn load_meta(&self, path: &Path) -> std::io::Result<MetadataBlock> {
        (self.load_meta)(path)
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
