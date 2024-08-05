use crate::{
    artifact::{Artifact, ArtifactMeta},
    asset::{Asset, AssetId, AssetSettings, AssetType, Settings},
    io::{AssetIo, AssetIoError, AssetWriter},
    loader::{
        AssetCacher, AssetError, AssetLoader, AssetProcessor, LoadContext, LoadedAsset,
        LoadedAssets, LoadedMetadata,
    },
};
use either::Either;
use shadow_ecs::core::{internal::blob::BlobCell, DenseMap};
use std::{collections::HashSet, path::Path};

pub struct ErasedLoader {
    load: fn(
        &Self,
        AssetId,
        &AssetLoaders,
        &AssetIo,
        &mut LoadedAssets,
        bool,
    ) -> Result<LoadedAsset, AssetError>,
    import: fn(
        &Self,
        &Path,
        &AssetLoaders,
        &AssetIo,
        &mut LoadedAssets,
    ) -> Result<ImportedAsset, AssetError>,
    process: Option<
        fn(Either<&mut ImportedAsset, &mut LoadedAsset>, &LoadedAssets) -> Result<(), AssetError>,
    >,
    cache: fn(&ImportedAsset, &mut dyn AssetWriter) -> Result<Vec<u8>, AssetIoError>,
    load_metadata: fn(&Path, &AssetIo) -> Result<LoadedMetadata, AssetIoError>,
}

impl ErasedLoader {
    pub fn new<L: AssetLoader>() -> Self {
        Self {
            load: |_self, id, loaders: &AssetLoaders, io, assets, load_deps| {
                let artifact = io
                    .load_artifact(id)
                    .map_err(|e: AssetIoError| AssetError::load(id, e))?;

                let asset =
                    L::Cacher::load(artifact.asset()).map_err(|e| AssetError::load(id, e))?;

                if load_deps {
                    loaders.load_dependencies(artifact.dependencies(), io, assets, true);
                }

                Ok(LoadedAsset::new(asset, artifact.meta))
            },
            import: |_self, path, loaders, io, assets| {
                let mut reader = io.reader(path);
                let settings = match io.load_metadata::<L::Settings>(path) {
                    Ok(meta) => meta,
                    Err(_) => AssetSettings::default(),
                };

                let settings_data = io
                    .save_metadata(path, &settings)
                    .map_err(|e| AssetError::import(path, e))?;

                let prev_meta = io.load_artifact_meta(settings.id()).ok();

                let (asset, dependencies) = {
                    let mut ctx = LoadContext::new(&settings);
                    let asset = match L::load(&mut ctx, reader.as_mut()) {
                        Ok(asset) => asset,
                        Err(err) => return Err(AssetError::import(path, err)),
                    };
                    (asset, ctx.finish())
                };

                let checksum = io.checksum(reader.bytes(), settings_data.as_bytes());

                let (id, settings) = settings.take();
                let meta = ArtifactMeta::new::<L::Asset>(id, checksum, dependencies);
                let loaded_bytes = reader.flush().map_err(|e| AssetError::import(path, e))?;
                let mut asset = ImportedAsset::new(asset, settings, meta, loaded_bytes)
                    .with_prev_meta(prev_meta);

                if let Some(processor) = &_self.process {
                    loaders.load_dependencies(asset.dependencies(), io, assets, false);
                    processor(Either::Left(&mut asset), assets)?;
                }

                let mut writer = io.writer(io.artifact(asset.meta.id()));
                _self
                    .cache(&asset, writer.as_mut())
                    .map_err(|e| AssetError::import(path, e))?;

                Ok(asset)
            },
            process: None,
            cache: |imported, writer| match L::Cacher::cache(imported.asset(), imported.settings())
            {
                Ok(bytes) => {
                    let settings = toml::to_string(imported.settings::<L::Settings>())
                        .map_err(|e| AssetIoError::from(e))?;
                    let artifact = Artifact::bytes(&bytes, settings.as_bytes(), imported.meta());

                    writer.write(&artifact)?;
                    writer.flush()
                }
                Err(e) => Err(AssetIoError::other(e)),
            },
            load_metadata: |path, io| {
                let metadata = io
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
        io: &AssetIo,
        assets: &mut LoadedAssets,
        load_dependencies: bool,
    ) -> Result<LoadedAsset, AssetError> {
        (self.load)(self, id, loaders, io, assets, load_dependencies)
    }

    pub fn import(
        &self,
        path: impl AsRef<Path>,
        loaders: &AssetLoaders,
        io: &AssetIo,
        assets: &mut LoadedAssets,
    ) -> Result<ImportedAsset, AssetError> {
        (self.import)(self, path.as_ref(), loaders, io, assets)
    }

    pub fn process(
        &self,
        asset: Either<&mut ImportedAsset, &mut LoadedAsset>,
        assets: &LoadedAssets,
    ) -> Option<Result<(), AssetError>> {
        self.process.map(|process| process(asset, assets))
    }

    pub fn cache(
        &self,
        asset: &ImportedAsset,
        writer: &mut dyn AssetWriter,
    ) -> Result<Vec<u8>, AssetIoError> {
        (self.cache)(asset, writer)
    }

    pub fn load_metadata(&self, path: &Path, io: &AssetIo) -> Result<LoadedMetadata, AssetIoError> {
        (self.load_metadata)(path, io)
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
        let loader = ErasedLoader::new::<L>();
        let asset_type = AssetType::of::<L::Asset>();
        let extensions = L::extensions();

        self.loaders.insert(asset_type, loader);
        for &ext in extensions {
            self.ext_map.insert(ext, asset_type);
        }
    }

    pub fn set_processor<P: AssetProcessor>(&mut self) {
        let ty = AssetType::of::<<P::Loader as AssetLoader>::Asset>();
        if let Some(loader) = self.loaders.get_mut(&ty) {
            loader.set_processor::<P>();
        } else {
            let mut loader = ErasedLoader::new::<P::Loader>();
            loader.set_processor::<P>();
            self.loaders.insert(ty, loader);
        }
    }

    pub fn load_dependencies<'a>(
        &self,
        dependencies: impl IntoIterator<Item = &'a AssetId>,
        io: &AssetIo,
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
    loaded_bytes: Vec<u8>,
}

impl ImportedAsset {
    pub fn new<A: Asset, S: Settings>(
        asset: A,
        settings: S,
        meta: ArtifactMeta,
        loaded_bytes: Vec<u8>,
    ) -> Self {
        Self {
            asset: BlobCell::new(asset),
            settings: BlobCell::new(settings),
            meta,
            prev_meta: None,
            loaded_bytes,
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

    pub fn loaded_bytes(&self) -> &[u8] {
        &self.loaded_bytes
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
