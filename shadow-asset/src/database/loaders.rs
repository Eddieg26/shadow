use super::AssetConfig;
use crate::{
    artifact::{Artifact, ArtifactMeta},
    asset::{Asset, AssetId, AssetSettings, AssetType, Settings},
    io::AssetIoError,
    loader::{
        AssetCacher, AssetError, AssetLoader, AssetProcessor, LoadContext, LoadedAsset,
        LoadedAssets, LoadedMetadata,
    },
};
use shadow_ecs::core::{internal::blob::BlobCell, DenseMap};
use std::{collections::HashSet, path::Path};

pub struct ErasedLoader {
    load: fn(
        &Self,
        AssetId,
        &AssetLoaders,
        &AssetConfig,
        &mut LoadedAssets,
        bool,
    ) -> Result<LoadedAsset, AssetError>,
    import: fn(
        &Self,
        &Path,
        &AssetLoaders,
        &AssetConfig,
        &mut LoadedAssets,
    ) -> Result<ImportedAsset, AssetError>,
    process: Option<fn(&mut ImportedAsset, &LoadedAssets) -> Result<(), AssetError>>,
    cache: fn(&ImportedAsset, &AssetConfig) -> Result<Vec<u8>, AssetIoError>,
    load_metadata: fn(&Path, &AssetConfig) -> Result<LoadedMetadata, AssetIoError>,
}

impl ErasedLoader {
    pub fn new<L: AssetLoader>() -> Self {
        Self {
            load: |_self, id, loaders: &AssetLoaders, config, assets, load_deps| {
                let artifact = config
                    .load_artifact(id)
                    .map_err(|e: AssetIoError| AssetError::load(id, e))?;

                let asset =
                    L::Cacher::load(artifact.asset()).map_err(|e| AssetError::load(id, e))?;

                if load_deps {
                    loaders.load_dependencies(artifact.dependencies(), config, assets, true);
                }

                Ok(LoadedAsset::new(asset, artifact.meta))
            },
            import: |_self, path, loaders, config, assets| {
                let path = config.asset(path);
                let mut reader = config.reader(&path);
                let settings = match config.load_metadata::<L::Settings>(&path) {
                    Ok(meta) => meta,
                    Err(_) => AssetSettings::default(),
                };

                let settings_data = config
                    .save_metadata(&path, &settings)
                    .map_err(|e| AssetError::import(&path, e))?;

                let prev_meta = config.load_artifact_meta(settings.id()).ok();

                let (asset, dependencies) = {
                    let mut ctx = LoadContext::new(&settings);
                    let asset = match L::load(&mut ctx, reader.as_mut()) {
                        Ok(asset) => asset,
                        Err(err) => return Err(AssetError::import(&path, err)),
                    };
                    (asset, ctx.finish())
                };

                let checksum = config.checksum(reader.bytes(), settings_data.as_bytes());

                let (id, settings) = settings.take();
                let meta = ArtifactMeta::new::<L::Asset>(id, checksum, dependencies);
                let mut asset = ImportedAsset::new(asset, settings, meta).with_prev_meta(prev_meta);

                if let Some(processor) = &_self.process {
                    loaders.load_dependencies(asset.dependencies(), config, assets, false);
                    processor(&mut asset, assets)?;
                }

                _self
                    .cache(&asset, config)
                    .map_err(|e| AssetError::import(path, e))?;

                Ok(asset)
            },
            process: None,
            cache: |imported, config| match L::Cacher::cache(imported.asset()) {
                Ok(bytes) => {
                    let id = imported.meta().id();
                    let mut writer = config.writer(config.artifact(id));
                    let artifact = Artifact::bytes(&bytes, imported.meta());

                    writer.write(&artifact)?;
                    writer.flush()
                }
                Err(e) => Err(AssetIoError::other(e)),
            },
            load_metadata: |path, config| {
                let metadata = config
                    .load_metadata::<L::Settings>(path)
                    .map_err(|e| AssetIoError::from(e))?;

                let data = toml::to_string(&metadata).map_err(|e| AssetIoError::from(e))?;

                Ok(LoadedMetadata::new(metadata.id(), data))
            },
        }
    }

    pub fn set_processor<P: AssetProcessor>(&mut self) {
        self.process = Some(|_, _| todo!());
    }

    pub fn load(
        &self,
        id: AssetId,
        loaders: &AssetLoaders,
        config: &AssetConfig,
        assets: &mut LoadedAssets,
        load_dependencies: bool,
    ) -> Result<LoadedAsset, AssetError> {
        (self.load)(self, id, loaders, config, assets, load_dependencies)
    }

    pub fn import(
        &self,
        path: impl AsRef<Path>,
        loaders: &AssetLoaders,
        config: &AssetConfig,
        assets: &mut LoadedAssets,
    ) -> Result<ImportedAsset, AssetError> {
        (self.import)(self, path.as_ref(), loaders, config, assets)
    }

    pub fn process(
        &self,
        asset: &mut ImportedAsset,
        assets: &LoadedAssets,
    ) -> Option<Result<(), AssetError>> {
        self.process.map(|process| process(asset, assets))
    }

    pub fn cache(
        &self,
        asset: &ImportedAsset,
        config: &AssetConfig,
    ) -> Result<Vec<u8>, AssetIoError> {
        (self.cache)(asset, config)
    }

    pub fn load_metadata(
        &self,
        path: &Path,
        config: &AssetConfig,
    ) -> Result<LoadedMetadata, AssetIoError> {
        (self.load_metadata)(path, config)
    }
}

pub struct AssetLoaders {
    loaders: DenseMap<AssetType, ErasedLoader>,
    ext_map: DenseMap<&'static str, AssetType>,
}

impl AssetLoaders {
    pub fn new() -> Self {
        Self {
            loaders: DenseMap::new(),
            ext_map: DenseMap::new(),
        }
    }

    pub fn get<A: Asset>(&self) -> Option<&ErasedLoader> {
        let ty = AssetType::of::<A>();
        self.loaders.get(&ty)
    }

    pub fn get_ty(&self, ty: AssetType) -> Option<&ErasedLoader> {
        self.loaders.get(&ty)
    }

    pub fn get_by_ext(&self, ext: &str) -> Option<&ErasedLoader> {
        self.ext_map.get(&ext).and_then(|ty| self.loaders.get(ty))
    }

    pub fn ext_type(&self, ext: &str) -> Option<AssetType> {
        self.ext_map.get(&ext).copied()
    }

    pub fn add_loader<L: AssetLoader>(&mut self) {
        let asset_type = AssetType::of::<L::Asset>();
        if !self.loaders.contains(&asset_type) {
            let loader = ErasedLoader::new::<L>();
            let extensions = L::extensions();

            self.loaders.insert(asset_type, loader);
            for &ext in extensions {
                self.ext_map.insert(ext, asset_type);
            }
        }
    }

    pub fn set_processor<P: AssetProcessor>(&mut self) {
        let ty = AssetType::of::<<P::Loader as AssetLoader>::Asset>();
        if let Some(loader) = self.loaders.get_mut(&ty) {
            loader.set_processor::<P>();
        } else {
            self.add_loader::<P::Loader>();
            let loader = self.loaders.get_mut(&ty).unwrap();
            loader.set_processor::<P>();
        }
    }

    pub fn load_dependencies<'a>(
        &self,
        dependencies: impl IntoIterator<Item = &'a AssetId>,
        io: &AssetConfig,
        assets: &mut LoadedAssets,
        recursive: bool,
    ) {
        for dependency in dependencies.into_iter() {
            if !assets.contains(dependency) {
                let meta = match io.load_artifact_meta(*dependency) {
                    Ok(meta) => meta,
                    Err(_) => continue,
                };

                let loaded = match self.get_ty(meta.ty()) {
                    Some(loader) => loader.load(*dependency, self, io, assets, recursive),
                    None => continue,
                };

                match loaded {
                    Ok(loaded) => assets.add_erased(*dependency, loaded),
                    Err(_) => continue,
                };
            }
        }
    }
}

pub struct ImportedAsset {
    asset: BlobCell,
    settings: BlobCell,
    meta: ArtifactMeta,
    prev_meta: Option<ArtifactMeta>,
}

impl ImportedAsset {
    pub fn new<A: Asset, S: Settings>(asset: A, settings: S, meta: ArtifactMeta) -> Self {
        Self {
            asset: BlobCell::new(asset),
            settings: BlobCell::new(settings),
            meta,
            prev_meta: None,
        }
    }

    pub fn with_prev_meta(mut self, prev_meta: Option<ArtifactMeta>) -> Self {
        self.prev_meta = prev_meta;
        self
    }

    pub fn id(&self) -> AssetId {
        self.meta.id()
    }

    pub fn asset<A: Asset>(&self) -> &A {
        self.asset.value()
    }

    pub fn asset_mut<A: Asset>(&mut self) -> &mut A {
        self.asset.value_mut()
    }

    pub fn settings<S: Settings>(&self) -> &S {
        self.settings.value()
    }

    pub fn settings_mut<S: Settings>(&mut self) -> &mut S {
        self.settings.value_mut()
    }

    pub fn meta(&self) -> &ArtifactMeta {
        &self.meta
    }

    pub fn prev_meta(&self) -> Option<&ArtifactMeta> {
        self.prev_meta.as_ref()
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        self.meta.dependencies()
    }
}

impl Into<LoadedAsset> for ImportedAsset {
    fn into(self) -> LoadedAsset {
        LoadedAsset {
            asset: self.asset,
            meta: self.meta,
        }
    }
}
