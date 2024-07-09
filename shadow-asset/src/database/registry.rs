use super::{config::AssetConfig, library::SourceInfo};
use crate::{
    artifact::ArtifactMeta,
    asset::{AssetMetadata, AssetPath, AssetType, Settings, Type},
    importer::{
        AssetImporter, AssetLoader, AssetProcessor, ImportContext, ImportFailed, LoadFailed,
    },
};
use shadow_ecs::ecs::{core::Resource, event::EventStorage, storage::dense::DenseMap};
use std::path::PathBuf;

pub struct ImportResult {
    pub source: SourceInfo,
    pub artifact: ArtifactMeta,
}

impl ImportResult {
    pub fn new(source: SourceInfo, artifact: ArtifactMeta) -> Self {
        ImportResult { source, artifact }
    }
}

#[derive(Clone, Copy)]
pub struct AssetMeta {
    import: fn(&PathBuf, &AssetConfig) -> Result<ImportResult, ImportFailed>,
    load_failed: fn(AssetPath, String, &mut EventStorage),
    asset_ty: AssetType,
    settings_ty: Type,
}

impl AssetMeta {
    pub fn new<L: AssetImporter>() -> Self {
        AssetMeta {
            import: |path, config| {
                let metadata = match config.load_metadata::<L::Settings>(path) {
                    Ok(metadata) => metadata,
                    Err(_) => AssetMetadata::<L::Settings>::default(),
                };

                let bytes = match path.is_dir() {
                    true => vec![],
                    false => std::fs::read(path)
                        .map_err(|e| ImportFailed::import(path.clone(), metadata.id(), e))?,
                };
                let source = AssetMeta::save_metadata(config, path, &bytes, &metadata)?;

                let (mut asset, dependencies) = {
                    let mut ctx = ImportContext::new(&path, &bytes, &metadata);
                    let asset = L::import(&mut ctx)
                        .map_err(|e| ImportFailed::import(path.clone(), metadata.id(), e))?;
                    (asset, ctx.finish())
                };

                let (id, settings) = metadata.take();

                L::Processor::process(&mut asset, &settings)
                    .map_err(|e| ImportFailed::process(path.clone(), id, e))?;

                if !path.is_dir() {
                    std::fs::write(config.artifact(&id), L::Loader::save(&asset))
                        .map_err(|e| ImportFailed::save(path.clone(), id, e))?;
                }

                let artifact =
                    ArtifactMeta::new(id, AssetType::of::<L::Asset>(), path.clone(), dependencies);

                Ok(ImportResult::new(source, artifact))
            },
            load_failed: |path, message, events| {
                let event = LoadFailed::<L::Asset>::new(path, message);
                events.add(event)
            },
            asset_ty: AssetType::of::<L::Asset>(),
            settings_ty: Type::of::<L::Settings>(),
        }
    }

    pub fn import(
        &self,
        path: &PathBuf,
        config: &AssetConfig,
    ) -> Result<ImportResult, ImportFailed> {
        (self.import)(path, config)
    }

    pub fn load_failed(&self, path: AssetPath, message: impl ToString, events: &mut EventStorage) {
        (self.load_failed)(path, message.to_string(), events)
    }

    fn save_metadata<S: Settings>(
        config: &AssetConfig,
        path: &PathBuf,
        asset: &[u8],
        metadata: &AssetMetadata<S>,
    ) -> Result<SourceInfo, ImportFailed> {
        let meta_bytes = config
            .save_metadata(path, metadata)
            .map_err(|e| ImportFailed::import(path.clone(), metadata.id(), e))?;
        let checksum = SourceInfo::calculate_checksum(asset, meta_bytes.as_bytes());
        let asset_modified = SourceInfo::modified(&path);
        let settings_modified = SourceInfo::modified(&config.metadata(&path));
        Ok(SourceInfo::raw(
            metadata.id(),
            checksum,
            asset_modified,
            settings_modified,
        ))
    }

    pub fn asset_ty(&self) -> AssetType {
        self.asset_ty
    }

    pub fn settings_ty(&self) -> Type {
        self.settings_ty
    }
}

#[derive(Clone)]
pub struct AssetRegistry {
    loaders: DenseMap<AssetType, AssetMeta>,
    ext_map: DenseMap<&'static str, AssetType>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        AssetRegistry {
            loaders: DenseMap::new(),
            ext_map: DenseMap::new(),
        }
    }

    pub fn register<L: AssetImporter>(&mut self) {
        let meta = AssetMeta::new::<L>();
        self.loaders.insert(AssetType::of::<L::Asset>(), meta);
        for ext in L::extensions() {
            self.ext_map.insert(ext, AssetType::of::<L::Asset>());
        }
    }

    pub fn meta(&self, asset: &AssetType) -> Option<&AssetMeta> {
        self.loaders.get(asset)
    }

    pub fn meta_by_ext(&self, ext: &str) -> Option<&AssetMeta> {
        self.ext_map.get(&ext).and_then(|asset| self.meta(asset))
    }

    pub fn meta_mut(&mut self, asset: AssetType) -> Option<&mut AssetMeta> {
        self.loaders.get_mut(&asset)
    }

    pub fn meta_by_ext_mut(&mut self, ext: &str) -> Option<&mut AssetMeta> {
        self.ext_map
            .get(&ext)
            .copied()
            .and_then(|asset| self.meta_mut(asset))
    }

    pub fn ext_ty(&self, ext: &str) -> Option<AssetType> {
        self.ext_map.get(&ext).copied()
    }
}

impl Resource for AssetRegistry {}
