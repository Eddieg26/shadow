use crate::{
    artifact::{Artifact, MetadataBlock},
    asset::{Asset, AssetId, AssetMetadata, AssetPath, AssetType, Settings},
    bytes::ToBytes,
    database::library::ArtifactInfo,
};
use shadow_ecs::ecs::{core::Resource, event::EventStorage, storage::dense::DenseMap};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{
    events::ImportFailed,
    library::{Artifacts, Sources},
    AssetDatabase,
};

pub enum LoadContextType<'a> {
    Processed { bytes: &'a [u8] },
    UnProcessed { path: &'a Path, bytes: &'a [u8] },
}

pub struct LoadContext<'a, S: Settings> {
    metadata: &'a mut AssetMetadata<S>,
    ty: LoadContextType<'a>,
    dependencies: HashSet<AssetId>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(metadata: &'a mut AssetMetadata<S>, ty: LoadContextType<'a>) -> Self {
        LoadContext {
            metadata,
            ty,
            dependencies: HashSet::new(),
        }
    }

    pub fn unprocessed(
        path: &'a Path,
        bytes: &'a [u8],
        metadata: &'a mut AssetMetadata<S>,
    ) -> Self {
        LoadContext {
            metadata,
            ty: LoadContextType::UnProcessed { path, bytes },
            dependencies: HashSet::new(),
        }
    }

    pub fn processed(bytes: &'a [u8], metadata: &'a mut AssetMetadata<S>) -> Self {
        LoadContext {
            metadata,
            ty: LoadContextType::Processed { bytes },
            dependencies: HashSet::new(),
        }
    }

    pub fn ty(&self) -> &LoadContextType<'a> {
        &self.ty
    }

    pub fn metadata(&self) -> &AssetMetadata<S> {
        self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut AssetMetadata<S> {
        self.metadata
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }
}

pub enum AssetError {
    AssetNotFound(AssetPath),
    SettingsNotFound(AssetId),
    ArtifactNotFound,
    Io(std::io::Error),
    Deserialize(String),
    Serialize(String),
    Process(String),
    PostProcess(String),
}

pub trait AssetLoader {
    type Asset: Asset;
    type Settings: Settings;

    fn load(ctx: &mut LoadContext<Self::Settings>) -> Result<Self::Asset, AssetError>;
    fn extensions() -> &'static [&'static str];
}

pub trait AssetProcessor {
    type Asset: Asset;
    type Settings: Settings;

    fn process(asset: &mut Self::Asset, settings: &mut Self::Settings) -> Result<(), AssetError>;
}

pub trait AssetSaver<A: Asset, S: Settings> {
    fn save(asset: &A, settings: &S) -> Result<Artifact, AssetError>;
}

impl<S: Settings + ToBytes, A: Asset + ToBytes> AssetSaver<A, S> for A {
    fn save(asset: &A, settings: &S) -> Result<Artifact, AssetError> {
        let asset = asset.to_bytes();
        let settings = settings.to_bytes();

        Ok(Artifact::new(&asset, &settings))
    }
}

pub struct BasicProcessor<A: Asset, S: Settings>(std::marker::PhantomData<(A, S)>);

impl<A: Asset, S: Settings> AssetProcessor for BasicProcessor<A, S> {
    type Asset = A;
    type Settings = S;

    fn process(_: &mut Self::Asset, _: &mut Self::Settings) -> Result<(), AssetError> {
        Ok(())
    }
}

pub trait AssetPipeline: 'static {
    type Loader: AssetLoader;
    type Processor: AssetProcessor<
        Asset = <Self::Loader as AssetLoader>::Asset,
        Settings = <Self::Loader as AssetLoader>::Settings,
    >;
    type Saver: AssetSaver<
        <Self::Loader as AssetLoader>::Asset,
        <Self::Loader as AssetLoader>::Settings,
    >;
}

pub struct ImportResult {
    pub id: AssetId,
    pub path: PathBuf,
    pub dependencies: HashSet<AssetId>,
}

impl ImportResult {
    pub fn new(id: AssetId, path: PathBuf, dependencies: HashSet<AssetId>) -> Self {
        ImportResult {
            id,
            path,
            dependencies,
        }
    }
}

mod import_utils {
    use super::AssetError;
    use crate::{
        asset::{AssetId, AssetMetadata, Settings},
        bytes::ToBytes,
        database::{
            library::{ArtifactInfo, Artifacts, SourceInfo},
            AssetDatabase,
        },
    };
    use std::{
        collections::HashSet,
        path::{Path, PathBuf},
    };

    pub fn load_metadata<S: Settings>(database: &AssetDatabase, path: &Path) -> AssetMetadata<S> {
        let path = database.config().metadata(path);
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return Default::default(),
        };
        toml::from_str(&content).unwrap_or_default()
    }

    pub fn save_metadata<S: Settings>(
        database: &AssetDatabase,
        path: &Path,
        asset: &[u8],
        metadata: &AssetMetadata<S>,
    ) -> Result<SourceInfo, AssetError> {
        let meta = database.config().metadata(path);
        let content =
            toml::to_string(metadata).map_err(|e| AssetError::Deserialize(e.to_string()))?;
        let content = std::fs::write(meta, &content)
            .map(|_| content)
            .map_err(AssetError::Io)?;

        let checksum = SourceInfo::calculate_checksum(&asset, content.as_bytes());
        let settings_modified = SourceInfo::modified(&database.config().metadata(path));
        let asset_modified = SourceInfo::modified(path);
        let source = SourceInfo::new(metadata.id())
            .with_asset_modified(asset_modified)
            .with_settings_modified(settings_modified)
            .with_checksum(checksum);

        Ok(source)
    }

    pub fn load_artifact_info(database: &AssetDatabase, id: &AssetId) -> Option<ArtifactInfo> {
        let path = PathBuf::from(format!("{}.meta", database.config().artifact(id).display()));
        let bytes = std::fs::read(path).ok()?;
        ArtifactInfo::from_bytes(&bytes)
    }

    pub fn save_artifact_info(
        database: &AssetDatabase,
        artifact: ArtifactInfo,
    ) -> Result<ArtifactInfo, AssetError> {
        let path = PathBuf::from(format!(
            "{}.meta",
            database.config().artifact(&artifact.id()).display()
        ));
        std::fs::write(path, artifact.to_bytes()).map_err(AssetError::Io)?;
        Ok(artifact)
    }

    pub fn remove_old_dependents(
        database: &AssetDatabase,
        artifacts: &mut Artifacts,
        artifact: &ArtifactInfo,
        dependencies: &HashSet<AssetId>,
    ) {
        let removed = artifact.dependencies().difference(&dependencies);
        for removed in removed {
            if let Some(artifact) = artifacts.get_mut(removed) {
                artifact.remove_dependent(&artifact.id());
            } else if let Some(mut artifact) = load_artifact_info(database, removed) {
                artifact.remove_dependent(&artifact.id());
                artifacts.insert(*removed, artifact);
            }
        }
    }
}

pub struct AssetPipelineMeta {
    ty: AssetType,
    import:
        fn(&Path, &AssetDatabase, &mut Sources, &mut Artifacts) -> Result<ArtifactInfo, AssetError>,
    load_metablock: fn(&Path) -> std::io::Result<MetadataBlock>,
    import_failed: fn(PathBuf, AssetError, &mut EventStorage),
}

impl AssetPipelineMeta {
    pub fn new<P: AssetPipeline>() -> Self {
        Self {
            ty: AssetType::of::<<P::Loader as AssetLoader>::Asset>(),
            import: |path, database, sources, artifacts| {
                let mut metadata = import_utils::load_metadata::<
                    <P::Loader as AssetLoader>::Settings,
                >(database, path);

                let bytes = std::fs::read(path).map_err(AssetError::Io)?;
                let mut ctx = LoadContext::unprocessed(path, &bytes, &mut metadata);

                let mut asset = P::Loader::load(&mut ctx)?;
                let dependencies = ctx.dependencies().clone();

                let source = import_utils::save_metadata(database, path, &bytes, &metadata)?;
                sources.insert(path.to_path_buf(), source);

                let (id, mut settings) = metadata.take();

                let artifact = match artifacts.remove(&id) {
                    Some(old) => old
                        .with_id(id)
                        .with_ty(AssetType::of::<<P::Loader as AssetLoader>::Asset>())
                        .with_path(path),
                    None => import_utils::load_artifact_info(database, &id).unwrap_or(
                        ArtifactInfo::new::<<P::Loader as AssetLoader>::Asset>(id, path),
                    ),
                };

                import_utils::remove_old_dependents(database, artifacts, &artifact, &dependencies);

                let info = import_utils::save_artifact_info(
                    database,
                    artifact.with_dependencies(dependencies),
                )?;

                P::Processor::process(&mut asset, &mut settings)?;
                let artifact = P::Saver::save(&asset, &settings)?;
                let path = database.config().artifact(&id);
                std::fs::write(path, artifact.to_bytes()).map_err(AssetError::Io)?;

                Ok(info)
            },
            load_metablock: |path| {
                let bytes = std::fs::read_to_string(path)?;
                let metadata =
                    toml::from_str::<AssetMetadata<<P::Loader as AssetLoader>::Settings>>(&bytes)
                        .map_err(|_| std::io::ErrorKind::InvalidData)?;

                Ok(MetadataBlock::new(metadata.id(), bytes.into_bytes()))
            },
            import_failed: |path, error, events| {
                events.add(ImportFailed::<<P::Loader as AssetLoader>::Asset>::new(
                    path, error,
                ));
            },
        }
    }

    pub fn ty(&self) -> &AssetType {
        &self.ty
    }

    pub fn import(
        &self,
        path: &Path,
        database: &AssetDatabase,
        sources: &mut Sources,
        artifacts: &mut Artifacts,
    ) -> Result<ArtifactInfo, AssetError> {
        (self.import)(path, database, sources, artifacts)
    }

    pub fn load_metablock(&self, path: &Path) -> std::io::Result<MetadataBlock> {
        (self.load_metablock)(path)
    }

    pub fn import_failed(&self, path: PathBuf, error: AssetError, events: &mut EventStorage) {
        (self.import_failed)(path, error, events);
    }
}

#[derive(Default, Clone)]
pub struct AssetRegistry {
    pipelines: DenseMap<AssetType, Arc<AssetPipelineMeta>>,
    ext_map: DenseMap<&'static str, AssetType>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        AssetRegistry {
            pipelines: DenseMap::new(),
            ext_map: DenseMap::new(),
        }
    }

    pub fn register<P: AssetPipeline>(&mut self) {
        let ty = AssetType::of::<<P::Loader as AssetLoader>::Asset>();
        self.pipelines
            .insert(ty, Arc::new(AssetPipelineMeta::new::<P>()));

        for ext in P::Loader::extensions() {
            self.ext_map.insert(ext, ty);
        }
    }

    pub fn pipeline(&self, ty: &AssetType) -> Option<&Arc<AssetPipelineMeta>> {
        self.pipelines.get(ty)
    }

    pub fn pipeline_by_ext(&self, ext: &str) -> Option<&Arc<AssetPipelineMeta>> {
        self.ext_map.get(&ext).and_then(|ty| self.pipeline(ty))
    }

    pub fn ext_ty<'a>(&'a self, ext: &'a str) -> Option<&AssetType> {
        self.ext_map.get(&ext)
    }
}

impl Resource for AssetRegistry {}
