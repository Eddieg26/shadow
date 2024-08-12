use std::{collections::HashSet, path::Path};

use super::{
    events::{AssetLoaded, AssetUnloaded},
    state::AssetState,
    AssetConfig,
};
use crate::{
    artifact::{Artifact, ArtifactMeta},
    asset::{Asset, AssetId, AssetSettings, AssetType, Assets, Settings},
    io::AssetIoError,
    loader::{
        AssetError, AssetLoader, AssetProcessor, AssetSerializer, LoadContext, LoadErrorKind,
        LoadedAsset, LoadedAssets, LoadedMetadata,
    },
};
use shadow_ecs::{
    core::{internal::blob::BlobCell, DenseMap},
    world::{event::ErasedEvent, World},
};

pub struct AssetMetadata {
    loaded: fn(LoadedAsset) -> ErasedEvent,
    unloaded: fn(AssetId, AssetState, &World) -> Option<ErasedEvent>,
    import: fn(
        &Self,
        &Path,
        &AssetRegistry,
        &AssetConfig,
        &mut LoadedAssets,
    ) -> Result<ImportedAsset, AssetError>,
    load: fn(
        &Self,
        AssetId,
        &AssetRegistry,
        &AssetConfig,
        &mut LoadedAssets,
        bool,
    ) -> Result<LoadedAsset, AssetError>,
    process: Option<fn(&mut ImportedAsset, &LoadedAssets) -> Result<(), AssetError>>,
    serialize: fn(&Path, &ImportedAsset, &AssetConfig) -> Result<Vec<u8>, AssetError>,
    load_metadata: Option<fn(&Path, &AssetConfig) -> Result<LoadedMetadata, AssetError>>,
}

impl AssetMetadata {
    pub fn new<A: Asset>() -> Self {
        Self {
            loaded: |loaded: LoadedAsset| {
                let id = loaded.meta.id();
                let dependencies = loaded.meta.dependencies;
                let asset = loaded.asset.take::<A>();
                ErasedEvent::new(AssetLoaded::new(id, asset, dependencies))
            },
            unloaded: |id, state, world| {
                let assets = world.resource_mut::<Assets<A>>();
                let asset = assets.remove(&id)?;

                Some(AssetUnloaded::new(id, asset, state).into())
            },
            import: |_self, path, _, _, _| Err(AssetError::import(path, LoadErrorKind::NoLoader)),
            load: |_self, id, _, _, _, _| Err(AssetError::load(id, LoadErrorKind::NoLoader)),
            process: None,
            serialize: |path, _imported, _config| {
                Err(AssetError::import(path, LoadErrorKind::NoSerializer))
            },
            load_metadata: None,
        }
    }

    pub fn set_loader<L: AssetLoader>(&mut self) {
        self.import = |_self, path, registry, config, assets| {
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
                registry.load_dependencies(asset.dependencies(), config, assets, false);
                processor(&mut asset, assets)?;
            }

            _self
                .serialize(&path, &asset, config)
                .map_err(|e| AssetError::import(path, e))?;

            Ok(asset)
        };

        self.set_serializer::<L::Serializer>();
    }

    pub fn set_serializer<C: AssetSerializer>(&mut self) {
        self.serialize = |path, imported, config| match C::serialize(imported.asset()) {
            Ok(bytes) => {
                let id = imported.meta().id();
                let mut writer = config.writer(config.artifact(id));
                let artifact = Artifact::bytes(&bytes, imported.meta());

                writer
                    .write(&artifact)
                    .map_err(|e| AssetError::import(path, e))?;
                writer.flush().map_err(|e| AssetError::import(path, e))
            }
            Err(e) => Err(AssetError::import(path, e)),
        };

        self.load = |_self, id, registry, config, assets, load_deps| {
            let artifact = config
                .load_artifact(id)
                .map_err(|e: AssetIoError| AssetError::load(id, e))?;

            let asset = C::deserialize(artifact.asset()).map_err(|e| AssetError::load(id, e))?;

            if load_deps {
                registry.load_dependencies(artifact.dependencies(), config, assets, true);
            }

            Ok(LoadedAsset::new(asset, artifact.meta))
        }
    }

    pub fn set_processor<P: AssetProcessor>(&mut self) {
        self.process = Some(|_, _| todo!());
    }

    pub fn loaded(&self, loaded: LoadedAsset) -> ErasedEvent {
        (self.loaded)(loaded)
    }

    pub fn unloaded(&self, id: AssetId, state: AssetState, world: &World) -> Option<ErasedEvent> {
        (self.unloaded)(id, state, world)
    }

    pub fn import(
        &self,
        path: &Path,
        registry: &AssetRegistry,
        config: &AssetConfig,
        assets: &mut LoadedAssets,
    ) -> Result<ImportedAsset, AssetError> {
        (self.import)(self, path, registry, config, assets)
    }

    pub fn load(
        &self,
        id: AssetId,
        registry: &AssetRegistry,
        config: &AssetConfig,
        assets: &mut LoadedAssets,
        load_dependencies: bool,
    ) -> Result<LoadedAsset, AssetError> {
        (self.load)(self, id, registry, config, assets, load_dependencies)
    }

    pub fn process(
        &self,
        asset: &mut ImportedAsset,
        assets: &LoadedAssets,
    ) -> Option<Result<(), AssetError>> {
        self.process.map(|process| process(asset, assets))
    }

    pub fn serialize(
        &self,
        path: &Path,
        asset: &ImportedAsset,
        config: &AssetConfig,
    ) -> Result<Vec<u8>, AssetError> {
        (self.serialize)(path, asset, config)
    }

    pub fn load_metadata(
        &self,
        path: &Path,
        config: &AssetConfig,
    ) -> Option<Result<LoadedMetadata, AssetError>> {
        self.load_metadata.map(|load| load(path, config))
    }
}

pub struct AssetRegistry {
    metadata: DenseMap<AssetType, AssetMetadata>,
    ext_map: DenseMap<&'static str, AssetType>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self {
            metadata: DenseMap::new(),
            ext_map: DenseMap::new(),
        }
    }

    pub fn has<A: Asset>(&self) -> bool {
        self.metadata.contains(&AssetType::of::<A>())
    }

    pub fn register<A: Asset>(&mut self) {
        let asset_type = AssetType::of::<A>();
        self.metadata.insert(asset_type, AssetMetadata::new::<A>());
    }

    pub fn set_loader<L: AssetLoader>(&mut self) {
        let asset_type = AssetType::of::<L::Asset>();
        let metadata = match self.metadata.get_mut(&asset_type) {
            Some(metadata) => metadata,
            None => {
                self.register::<L::Asset>();
                self.metadata.get_mut(&asset_type).unwrap()
            }
        };

        for ext in L::extensions() {
            self.ext_map.insert(ext, asset_type);
        }

        metadata.set_loader::<L>();
    }

    pub fn set_processor<P: AssetProcessor>(&mut self) {
        let asset_type = AssetType::of::<<P::Loader as AssetLoader>::Asset>();
        let metadata = match self.metadata.get_mut(&asset_type) {
            Some(metadata) => metadata,
            None => {
                self.set_loader::<P::Loader>();
                self.metadata.get_mut(&asset_type).unwrap()
            }
        };

        metadata.set_processor::<P>();
    }

    pub fn set_serializer<S: AssetSerializer>(&mut self) {
        let asset_type = AssetType::of::<S::Asset>();
        let metadata = match self.metadata.get_mut(&asset_type) {
            Some(metadata) => metadata,
            None => {
                self.register::<S::Asset>();
                self.metadata.get_mut(&asset_type).unwrap()
            }
        };

        metadata.set_serializer::<S>();
    }

    pub fn get_metadata(&self, asset_type: AssetType) -> Option<&AssetMetadata> {
        self.metadata.get(&asset_type)
    }

    pub fn get_metadata_mut(&mut self, asset_type: AssetType) -> Option<&mut AssetMetadata> {
        self.metadata.get_mut(&asset_type)
    }

    pub fn get_metadata_by_ext(&self, ext: &str) -> Option<&AssetMetadata> {
        self.ext_map
            .get(&ext)
            .and_then(|&asset_type| self.metadata.get(&asset_type))
    }

    pub fn ext_type(&self, ext: &str) -> Option<AssetType> {
        self.ext_map.get(&ext).copied()
    }

    fn load_dependencies<'a>(
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

                let loaded = match self.get_metadata(meta.ty()) {
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
