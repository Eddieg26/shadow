use super::{
    config::AssetConfig,
    library::{AssetLibraryRefMut, DependencyInfo, DependencyMap, SourceInfo},
};
use crate::{
    artifact::ArtifactMeta,
    asset::{AssetId, AssetMetadata, AssetType, Settings, Type},
    bytes::ToBytes,
    loader::{
        AssetCacher, AssetLoader, AssetProcessor, AssetStorage, LoadContext, LoadError,
        ProcessError, SaveError,
    },
};
use shadow_ecs::ecs::{core::Resource, storage::dense::DenseMap};
use std::path::{Path, PathBuf};

pub struct AssetLoaderMeta {
    import: fn(
        &PathBuf,
        &AssetConfig,
        &AssetLoaderRegistry,
        &mut AssetLibraryRefMut,
        &mut AssetStorage,
        &mut DependencyMap,
    ) -> Result<ArtifactMeta, LoadError>,
    process: fn(&AssetId, &mut AssetStorage) -> Result<(), ProcessError>,
    save: fn(&AssetId, &AssetStorage) -> Result<Vec<u8>, SaveError>,
    load: fn(&AssetId, &AssetConfig, &mut AssetStorage) -> Result<(), LoadError>,
    asset_ty: AssetType,
    settings_ty: Type,
}

impl AssetLoaderMeta {
    pub fn new<L: AssetLoader>() -> Self {
        AssetLoaderMeta {
            import: |path, config, registry, library, assets, map| {
                let metadata = match std::fs::read_to_string(config.metadata(&path)) {
                    Ok(content) => toml::from_str(&content).unwrap_or_default(),
                    Err(_) => AssetMetadata::<L::Settings>::default(),
                };

                let bytes = std::fs::read(path).map_err(LoadError::Io)?;
                let source = AssetLoaderMeta::save_metadata(config, path, &bytes, &metadata)?;

                let (asset, dependencies) = {
                    let mut ctx = LoadContext::new(&path, &bytes, &metadata);
                    (L::load(&mut ctx)?, ctx.finish())
                };

                let mut dependency_info =
                    match std::fs::read(config.dependency_map().join(metadata.id().to_string())) {
                        Ok(bytes) => DependencyInfo::from_bytes(&bytes).unwrap_or_default(),
                        Err(_) => DependencyInfo::default(),
                    };

                AssetLoaderMeta::load_dependencies(
                    &metadata.id(),
                    dependencies.iter(),
                    config,
                    registry,
                    library,
                    assets,
                    map,
                );

                dependency_info.set_dependencies(dependencies);

                let (id, settings) = metadata.take();
                assets.insert(id, asset);
                assets.insert_settings(id, settings);
                library.insert_source(path.clone(), source);
                map.insert(id, dependency_info);

                let artifact = ArtifactMeta::new(id, AssetType::of::<L::Asset>(), path.clone());
                Ok(artifact)
            },
            process: |_, _| Ok(()),
            save: |id, assets| {
                let asset = assets
                    .asset::<L::Asset>(id)
                    .ok_or(SaveError::AssetNotFound { id: *id })?;

                Ok(L::Cacher::cache(asset))
            },
            load: |id, config, assets| {
                let asset = std::fs::read(config.artifact(id)).map_err(LoadError::Io)?;
                let asset = L::Cacher::load(&asset)?;
                assets.insert(*id, asset);
                Ok(())
            },
            asset_ty: AssetType::of::<L::Asset>(),
            settings_ty: Type::of::<L::Settings>(),
        }
    }

    pub fn set_processor<P: AssetProcessor>(&mut self) {
        self.process = |id, assets| {
            let mut asset = assets
                .remove::<<P::Loader as AssetLoader>::Asset>(id)
                .ok_or(ProcessError::AssetNotFound { id: *id })?;

            let settings = assets
                .remove_settings::<<P::Loader as AssetLoader>::Settings>(id)
                .ok_or(ProcessError::SettingsNotFound { id: *id })?;

            let result = P::process(&mut asset, &settings, assets);

            assets.insert(*id, asset);
            assets.insert_settings(*id, settings);

            result
        }
    }

    pub fn import(
        &self,
        path: &PathBuf,
        config: &AssetConfig,
        registry: &AssetLoaderRegistry,
        library: &mut AssetLibraryRefMut,
        assets: &mut AssetStorage,
        map: &mut DependencyMap,
    ) -> Result<ArtifactMeta, LoadError> {
        (self.import)(path, config, registry, library, assets, map)
    }

    pub fn process(&self, id: &AssetId, assets: &mut AssetStorage) -> Result<(), ProcessError> {
        (self.process)(id, assets)
    }

    pub fn save(&self, id: &AssetId, assets: &AssetStorage) -> Result<Vec<u8>, SaveError> {
        (self.save)(id, assets)
    }

    pub fn load(
        &self,
        id: &AssetId,
        config: &AssetConfig,
        assets: &mut AssetStorage,
    ) -> Result<(), LoadError> {
        (self.load)(id, config, assets)
    }

    fn save_metadata<S: Settings>(
        config: &AssetConfig,
        path: &Path,
        asset: &[u8],
        metadata: &AssetMetadata<S>,
    ) -> Result<SourceInfo, LoadError> {
        let meta_bytes =
            toml::to_string(metadata.settings()).map_err(|e| LoadError::InvalidMetadata {
                message: e.to_string(),
            })?;

        std::fs::write(config.metadata(path), &meta_bytes).map_err(LoadError::Io)?;
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

    fn load_dependencies<'a>(
        id: &AssetId,
        dependencies: impl Iterator<Item = &'a AssetId>,
        config: &AssetConfig,
        registry: &AssetLoaderRegistry,
        library: &AssetLibraryRefMut,
        assets: &mut AssetStorage,
        map: &mut DependencyMap,
    ) {
        for dependency in dependencies {
            let ty = match library.artifact(dependency) {
                Some(artifact) => artifact.ty(),
                None => continue,
            };

            if assets.contains_asset(dependency, &ty) {
                continue;
            }

            let meta = match registry.meta(&ty) {
                Some(meta) => meta,
                None => continue,
            };

            if let Some(info) = map.get_mut(dependency) {
                info.add_dependent(*id)
            } else {
                let mut info = DependencyInfo::new();
                info.add_dependent(*id);
                map.insert(*dependency, info);
            }

            let _ = (meta.load)(dependency, config, assets);
        }
    }

    pub fn asset_ty(&self) -> AssetType {
        self.asset_ty
    }

    pub fn settings_ty(&self) -> Type {
        self.settings_ty
    }
}

pub struct AssetLoaderRegistry {
    loaders: DenseMap<AssetType, AssetLoaderMeta>,
    ext_map: DenseMap<&'static str, AssetType>,
}

impl AssetLoaderRegistry {
    pub fn new() -> Self {
        AssetLoaderRegistry {
            loaders: DenseMap::new(),
            ext_map: DenseMap::new(),
        }
    }

    pub fn register<L: AssetLoader>(&mut self) {
        let meta = AssetLoaderMeta::new::<L>();
        self.loaders.insert(AssetType::of::<L::Asset>(), meta);
        for ext in L::extensions() {
            self.ext_map.insert(ext, AssetType::of::<L::Asset>());
        }
    }

    pub fn meta(&self, asset: &AssetType) -> Option<&AssetLoaderMeta> {
        self.loaders.get(asset)
    }

    pub fn meta_by_ext(&self, ext: &str) -> Option<&AssetLoaderMeta> {
        self.ext_map.get(&ext).and_then(|asset| self.meta(asset))
    }

    pub fn meta_mut(&mut self, asset: AssetType) -> Option<&mut AssetLoaderMeta> {
        self.loaders.get_mut(&asset)
    }

    pub fn meta_by_ext_mut(&mut self, ext: &str) -> Option<&mut AssetLoaderMeta> {
        self.ext_map
            .get(&ext)
            .copied()
            .and_then(|asset| self.meta_mut(asset))
    }

    pub fn ext_ty(&self, ext: &str) -> Option<AssetType> {
        self.ext_map.get(&ext).copied()
    }
}

impl Resource for AssetLoaderRegistry {}
