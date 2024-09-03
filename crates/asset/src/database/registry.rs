use super::{
    events::{AssetLoaded, AssetUnloaded},
    state::AssetState,
    AssetConfig,
};
use crate::{
    artifact::{Artifact, ArtifactMeta},
    asset::{Asset, AssetId, AssetSettings, AssetType, Assets, Settings},
    importer::{
        AssetError, AssetImporter, ImportContext, LoadErrorKind, LoadedAsset, LoadedAssets,
        LoadedMetadata, ProcessContext,
    },
    io::{
        embedded::{EmbeddedAssets, EmbeddedReader},
        AssetIoError, PathExt,
    },
    AssetKind,
};
use ecs::{
    core::{internal::blob::BlobCell, DenseMap},
    world::{event::ErasedEvent, World},
};
use std::{collections::HashSet, path::Path};

pub type AssetImportFn =
    fn(&Path, &AssetConfig, &mut LoadedAssets) -> Result<ImportedAsset, AssetError>;

pub type AssetLoadFn =
    fn(AssetId, &AssetConfig, &mut LoadedAssets, bool) -> Result<LoadedAsset, AssetError>;

pub type AssetSaveFn = fn(&Path, &ImportedAsset, &AssetConfig) -> Result<Vec<u8>, AssetError>;

pub type AssetEmbedFn = fn(&'static str, &'static [u8], &World) -> Result<(), AssetError>;

pub struct AssetMetadata {
    load: AssetLoadFn,
    save: AssetSaveFn,
    importers: DenseMap<&'static str, AssetImportFn>,
    embeders: DenseMap<&'static str, AssetEmbedFn>,
    loaded: fn(LoadedAsset) -> ErasedEvent,
    unloaded: fn(AssetId, AssetState, &World) -> Option<ErasedEvent>,
    load_metadata: Option<fn(&Path, &AssetConfig) -> Result<LoadedMetadata, AssetError>>,
}

impl AssetMetadata {
    pub fn new<A: Asset>() -> Self {
        Self {
            importers: DenseMap::new(),
            embeders: DenseMap::new(),
            load: |id, config, assets, load_deps| {
                let artifact = config
                    .load_artifact(id)
                    .map_err(|e: AssetIoError| AssetError::load(id, e))?;

                let asset = bincode::deserialize::<A>(artifact.asset())
                    .map_err(|e| AssetError::load(id, e))?;

                if load_deps {
                    let registry = config.registry();
                    let deps = artifact.dependencies();
                    Self::load_dependencies(registry, deps, config, assets, true);
                }

                Ok(LoadedAsset::new(asset, artifact.meta))
            },
            save: |path, asset, config| {
                let bytes = bincode::serialize::<A>(asset.asset())
                    .map_err(|e| AssetError::import(path, e))?;

                config
                    .save_artifact(&Artifact::new(&bytes, asset.meta().clone()))
                    .map_err(|e| AssetError::import(path, e))
            },
            loaded: |loaded: LoadedAsset| {
                let id = loaded.meta.id();
                let dependencies = loaded.meta.dependencies;
                let parent = loaded.meta.parent;
                let asset = loaded.asset.take::<A>();
                ErasedEvent::new(AssetLoaded::new(id, asset, dependencies, parent))
            },
            unloaded: |id, state, world| {
                let assets = world.resource_mut::<Assets<A>>();
                let asset = assets.remove(&id);

                Some(AssetUnloaded::new(id, asset, state).into())
            },
            load_metadata: None,
        }
    }

    pub fn add_importer<I: AssetImporter>(&mut self) {
        let import: AssetImportFn = |path, config, assets| {
            let registry = config.registry();
            let path = config.asset(path);
            let mut reader = config.reader(&path);

            let mut settings = match config.load_metadata::<I::Settings>(&path) {
                Ok(meta) => meta,
                Err(_) => AssetSettings::default(),
            };

            let (mut asset, dependencies, sub_assets) = {
                let mut ctx = ImportContext::new(config, &settings);
                match I::import(&mut ctx, reader.as_mut()) {
                    Err(err) => return Err(AssetError::import(&path, err)),
                    Ok(asset) => {
                        let (dependencies, sub_assets) = ctx.finish();
                        (asset, dependencies, sub_assets)
                    }
                }
            };

            let sub_assets = {
                Self::load_dependencies(registry, &dependencies, config, assets, false);
                let mut ctx = ProcessContext::new(&mut settings, assets, sub_assets);
                I::process(&mut ctx, &mut asset).map_err(|e| AssetError::import(&path, e))?;
                ctx.finish()
            };

            let mut meta = {
                let bytes = config
                    .save_metadata(&path, &settings)
                    .map_err(|e| AssetError::import(&path, e))?;

                let checksum = config.checksum(reader.bytes(), &bytes);
                ArtifactMeta::new::<I::Asset>(settings.id(), checksum, dependencies)
            };

            for sub_asset in sub_assets {
                let metadata = match registry.get_metadata(sub_asset.meta().ty()) {
                    Some(metadata) => metadata,
                    None => continue,
                };

                if let Ok(_) = metadata.save(&path, &sub_asset, config) {
                    meta.add_child(sub_asset.meta().id());
                }
            }

            let bytes = bincode::serialize(&asset).map_err(|e| AssetError::import(&path, e))?;
            let artifact = Artifact::new(&bytes, meta);
            config
                .save_artifact(&artifact)
                .map_err(|e| AssetError::import(path, e))?;

            let prev_meta = config.load_artifact_meta(settings.id()).ok();
            let asset = ImportedAsset::new(asset, settings.take().1, artifact.meta, prev_meta);

            Ok(asset)
        };

        let embed: AssetEmbedFn = |path, bytes, world| {
            let config = world.resource::<AssetConfig>();
            let mut reader = EmbeddedReader::new(path, bytes);
            let mut settings = AssetSettings::default();

            let (mut asset, sub_assets) = {
                let mut ctx = ImportContext::new(config, &settings);
                match I::import(&mut ctx, &mut reader) {
                    Err(err) => return Err(AssetError::import(&path, err)),
                    Ok(asset) => {
                        let (_, sub_assets) = ctx.finish();
                        (asset, sub_assets)
                    }
                }
            };

            let sub_assets = {
                let mut assets = LoadedAssets::new();
                let mut ctx = ProcessContext::new(&mut settings, &mut assets, sub_assets);
                I::process(&mut ctx, &mut asset).map_err(|e| AssetError::import(&path, e))?;
                ctx.finish()
            };

            world.events().add(AssetLoaded::new(
                settings.id(),
                asset,
                Default::default(),
                None,
            ));

            let embedded = world.resource_mut::<EmbeddedAssets>();
            embedded.add(settings.id(), path, AssetKind::Main);

            let registry = config.registry();
            for asset in sub_assets {
                let metadata = match registry.get_metadata(asset.meta().ty()) {
                    Some(metadata) => metadata,
                    None => continue,
                };

                embedded.add(asset.id(), path, AssetKind::Sub);
                world.events().add(metadata.loaded(asset.into()));
            }

            Ok(())
        };

        for ext in I::extensions() {
            self.importers.insert(ext, import);
            self.embeders.insert(ext, embed);
        }
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
        config: &AssetConfig,
        assets: &mut LoadedAssets,
    ) -> Result<ImportedAsset, AssetError> {
        let ext = path
            .ext()
            .ok_or_else(|| AssetError::import(path, LoadErrorKind::NoExtension))?;
        let import = self
            .importers
            .get(&ext)
            .ok_or_else(|| AssetError::import(path, LoadErrorKind::NoImporter))?;

        import(path, config, assets)
    }

    pub fn load(
        &self,
        id: AssetId,
        config: &AssetConfig,
        assets: &mut LoadedAssets,
        load_dependencies: bool,
    ) -> Result<LoadedAsset, AssetError> {
        (self.load)(id, config, assets, load_dependencies)
    }

    pub fn save(
        &self,
        path: &Path,
        asset: &ImportedAsset,
        config: &AssetConfig,
    ) -> Result<Vec<u8>, AssetError> {
        (self.save)(path, asset, config)
    }

    pub fn load_metadata(
        &self,
        path: &Path,
        config: &AssetConfig,
    ) -> Option<Result<LoadedMetadata, AssetError>> {
        self.load_metadata.map(|load| load(path, config))
    }

    pub fn embed(
        &self,
        path: &'static str,
        bytes: &'static [u8],
        world: &World,
    ) -> Result<(), AssetError> {
        let ext = path
            .ext()
            .ok_or_else(|| AssetError::import(path, LoadErrorKind::NoExtension))?;
        let embed = self
            .embeders
            .get(&ext)
            .ok_or_else(|| AssetError::import(path, LoadErrorKind::NoImporter))?;

        embed(path, bytes, world)
    }

    pub fn load_dependencies<'a>(
        registry: &AssetRegistry,
        dependencies: impl IntoIterator<Item = &'a AssetId>,
        config: &AssetConfig,
        assets: &mut LoadedAssets,
        recursive: bool,
    ) {
        for dependency in dependencies.into_iter() {
            if !assets.contains(dependency) {
                let meta = match config.load_artifact_meta(*dependency) {
                    Ok(meta) => meta,
                    Err(_) => continue,
                };

                let loaded = match registry.get_metadata(meta.ty()) {
                    Some(loader) => loader.load(*dependency, config, assets, recursive),
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

    pub fn add_importer<I: AssetImporter>(&mut self) {
        let asset_type = AssetType::of::<I::Asset>();
        let metadata = match self.metadata.get_mut(&asset_type) {
            Some(metadata) => metadata,
            None => {
                self.register::<I::Asset>();
                self.metadata.get_mut(&asset_type).unwrap()
            }
        };

        for ext in I::extensions() {
            self.ext_map.insert(ext, asset_type);
        }

        metadata.add_importer::<I>();
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
}

pub struct ImportedAsset {
    asset: BlobCell,
    settings: BlobCell,
    meta: ArtifactMeta,
    prev_meta: Option<ArtifactMeta>,
}

impl ImportedAsset {
    pub fn new<A: Asset, S: Settings>(
        asset: A,
        settings: S,
        meta: ArtifactMeta,
        prev_meta: Option<ArtifactMeta>,
    ) -> Self {
        Self {
            asset: BlobCell::new(asset),
            settings: BlobCell::new(settings),
            meta,
            prev_meta,
        }
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
