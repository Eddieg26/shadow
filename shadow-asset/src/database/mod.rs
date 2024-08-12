use crate::{
    artifact::{Artifact, ArtifactHeader, ArtifactMeta},
    asset::{Asset, AssetId, AssetSettings, Settings},
    bytes::IntoBytes,
    io::{
        local::LocalFileSystem, AssetFileSystem, AssetIoError, AssetReader, AssetWriter, PathExt,
    },
    loader::{AssetLoader, AssetProcessor},
};
use events::{AssetEvent, AssetEvents};
use library::AssetLibrary;
use loaders::AssetLoaders;
use registry::AssetRegistry;
use shadow_ecs::{core::Resource, system::RunMode};
use state::AssetStates;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub mod events;
pub mod library;
pub mod loaders;
pub mod registry;
pub mod state;

#[derive(Clone)]
pub struct AssetDatabase {
    config: Arc<AssetConfig>,
    library: Arc<RwLock<AssetLibrary>>,
    states: Arc<RwLock<AssetStates>>,
    events: Arc<Mutex<AssetEvents>>,
}

impl AssetDatabase {
    pub fn new(config: AssetConfig) -> Self {
        Self {
            library: Arc::new(RwLock::new(AssetLibrary::new())),
            states: Arc::new(RwLock::new(AssetStates::new())),
            events: Arc::new(Mutex::new(AssetEvents::new())),
            config: Arc::new(config),
        }
    }

    pub fn config(&self) -> &AssetConfig {
        &self.config
    }

    pub fn loaders(&self) -> &AssetLoaders {
        self.config.loaders()
    }

    pub fn registry(&self) -> &AssetRegistry {
        self.config.registry()
    }

    pub fn library(&self) -> RwLockReadGuard<AssetLibrary> {
        self.library.read().unwrap()
    }

    pub fn states(&self) -> RwLockReadGuard<AssetStates> {
        self.states.read().unwrap()
    }

    pub fn add_event<E: AssetEvent>(&self, event: E) {
        self.events.lock().unwrap().push(event);
    }

    pub(crate) fn library_mut(&self) -> RwLockWriteGuard<AssetLibrary> {
        self.library.write().unwrap()
    }

    pub(crate) fn states_mut(&self) -> RwLockWriteGuard<AssetStates> {
        self.states.write().unwrap()
    }

    pub(crate) fn events(&self) -> MutexGuard<AssetEvents> {
        self.events.lock().unwrap()
    }

    pub(crate) fn pop_event(&self) -> Option<Box<dyn AssetEvent>> {
        self.events().pop()
    }
}

impl Resource for AssetDatabase {}

pub struct AssetConfig {
    assets: PathBuf,
    cache: PathBuf,
    temp: PathBuf,
    import_batch_size: usize,
    registry: AssetRegistry,
    loaders: AssetLoaders,
    filesystem: Box<dyn AssetFileSystem>,
    mode: RunMode,
}

impl AssetConfig {
    pub fn new<Fs: AssetFileSystem>(filesystem: Fs) -> Self {
        let assets = PathBuf::from("assets");
        let cache = PathBuf::from(".cache");
        let temp = PathBuf::from(".temp");

        Self {
            assets,
            cache,
            temp,
            import_batch_size: 250,
            registry: AssetRegistry::new(),
            loaders: AssetLoaders::new(),
            filesystem: Box::new(filesystem),
            mode: RunMode::Parallel,
        }
    }

    pub fn root(&self) -> &Path {
        self.filesystem.root()
    }

    pub fn assets(&self) -> &Path {
        &self.assets
    }

    pub fn cache(&self) -> &Path {
        &self.cache
    }

    pub fn temp(&self) -> &Path {
        &self.temp
    }

    pub fn mode(&self) -> RunMode {
        self.mode
    }

    pub fn filesystem(&self) -> &dyn AssetFileSystem {
        self.filesystem.as_ref()
    }

    pub fn import_batch_size(&self) -> usize {
        self.import_batch_size
    }

    pub fn registry(&self) -> &AssetRegistry {
        &self.registry
    }

    pub fn loaders(&self) -> &AssetLoaders {
        &self.loaders
    }

    pub fn set_file_system<Fs: AssetFileSystem>(&mut self, filesystem: Fs) {
        self.filesystem = Box::new(filesystem);
    }

    pub fn set_import_batch_size(&mut self, size: usize) {
        match size {
            0 => self.import_batch_size = 1,
            _ => self.import_batch_size = size,
        }
    }

    pub fn set_run_mode(&mut self, mode: RunMode) {
        self.mode = mode;
    }

    pub fn register<A: Asset>(&mut self) {
        self.registry.register::<A>();
    }

    pub fn add_loader<L: AssetLoader>(&mut self) {
        self.loaders.add_loader::<L>();
    }

    pub fn set_processor<P: AssetProcessor>(&mut self) {
        self.loaders.set_processor::<P>();
    }
}

impl AssetConfig {
    pub fn init(&self) -> Result<(), AssetIoError> {
        self.writer(self.assets()).create_dir()?;

        self.writer(self.temp()).create_dir()?;

        self.writer(self.artifacts()).create_dir()
    }

    pub fn asset(&self, path: impl AsRef<Path>) -> PathBuf {
        self.assets().join(path)
    }

    pub fn artifacts(&self) -> PathBuf {
        self.cache().join("artifacts")
    }

    pub fn artifact(&self, id: AssetId) -> PathBuf {
        self.artifacts().join(id.to_string())
    }

    pub fn reader(&self, path: impl AsRef<Path>) -> Box<dyn AssetReader> {
        self.filesystem
            .reader(&path.as_ref().with_prefix(self.root()))
    }

    pub fn writer(&self, path: impl AsRef<Path>) -> Box<dyn AssetWriter> {
        self.filesystem
            .writer(&path.as_ref().with_prefix(self.root()))
    }

    pub fn checksum(&self, asset: &[u8], settings: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(asset);
        hasher.update(settings);
        hasher.finalize()
    }

    pub fn remove_file(&self, path: impl AsRef<Path>) -> Result<(), AssetIoError> {
        let mut writer = self.writer(path);
        writer.remove_file()
    }

    pub fn remove_dir(&self, path: impl AsRef<Path>) -> Result<(), AssetIoError> {
        let mut writer = self.writer(path);
        writer.remove_dir()
    }

    pub fn load_metadata<S: Settings>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<AssetSettings<S>, AssetIoError> {
        let mut reader = self.reader(path.append_ext("meta"));
        reader.read_to_end()?;

        let meta = String::from_utf8(reader.flush()?).map_err(AssetIoError::other)?;
        toml::from_str(&meta).map_err(AssetIoError::from)
    }

    pub fn save_metadata<S: Settings>(
        &self,
        path: impl AsRef<Path>,
        settings: &AssetSettings<S>,
    ) -> Result<String, AssetIoError> {
        let mut writer = self.writer(path.append_ext("meta"));
        let meta = toml::to_string(settings).map_err(AssetIoError::from)?;

        writer.write(meta.as_bytes())?;
        writer.flush()?;
        Ok(meta)
    }

    pub fn load_artifact_meta(&self, id: AssetId) -> Result<ArtifactMeta, AssetIoError> {
        let path = self.artifact(id);
        if !self.filesystem().exists(&path) {
            return Err(AssetIoError::NotFound(path));
        }

        let mut reader = self.reader(path);
        reader.read(ArtifactHeader::SIZE)?;

        let header = ArtifactHeader::from_bytes(reader.bytes())
            .ok_or(AssetIoError::from(std::io::ErrorKind::InvalidData))?;

        reader.read(header.meta())?;

        let meta_bytes =
            &reader.bytes()[ArtifactHeader::SIZE..ArtifactHeader::SIZE + header.meta()];

        ArtifactMeta::from_bytes(meta_bytes)
            .ok_or(AssetIoError::from(std::io::ErrorKind::InvalidData))
    }

    pub fn load_artifact(&self, id: AssetId) -> Result<Artifact, AssetIoError> {
        let path = self.artifact(id);
        if !self.filesystem().exists(&path) {
            return Err(AssetIoError::NotFound(path));
        }

        let mut reader = self.reader(path);
        reader.read_to_end()?;

        Artifact::from_bytes(&reader.flush()?)
            .ok_or(AssetIoError::from(std::io::ErrorKind::InvalidData))
    }
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            assets: PathBuf::from("assets"),
            cache: PathBuf::from(".cache"),
            temp: PathBuf::from(".temp"),
            import_batch_size: 250,
            registry: AssetRegistry::new(),
            loaders: AssetLoaders::new(),
            filesystem: Box::new(LocalFileSystem::new("Project")),
            mode: RunMode::Parallel,
        }
    }
}

impl Resource for AssetConfig {}
