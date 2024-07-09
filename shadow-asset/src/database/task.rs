use super::{
    library::{AssetLibrary, AssetLibraryError, SourceInfo},
    registry::{AssetRegistry, ImportResult},
    AssetDatabase,
};
use crate::{
    artifact::ArtifactMeta,
    asset::{AssetId, AssetMetadata},
    importer::{Folder, FolderSettings, ImportFailed},
};
use shadow_ecs::ecs::{
    event::{Event, EventStorage},
    storage::dense::DenseSet,
    world::World,
};
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetTaskExecutorState {
    Idle,
    Running,
}

pub struct StartAssetTask {
    task: Box<dyn AssetTaskExecutor>,
}

impl StartAssetTask {
    pub fn new(task: impl AssetTaskExecutor) -> Self {
        Self {
            task: Box::new(task),
        }
    }
}

impl Event for StartAssetTask {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        database.tasks().push_back_dyn(self.task);
        Some(())
    }
}

pub struct AssetTaskComplete;

impl Event for AssetTaskComplete {
    type Output = ();

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(())
    }
}

pub trait AssetTaskExecutor: Send + 'static {
    fn execute(
        &self,
        database: &AssetDatabase,
        registry: &AssetRegistry,
        events: &mut EventStorage,
    );
}

pub struct AssetTaskExecutorQueue {
    state: AssetTaskExecutorState,
    tasks: VecDeque<Box<dyn AssetTaskExecutor>>,
}

impl AssetTaskExecutorQueue {
    pub fn new() -> Self {
        Self {
            state: AssetTaskExecutorState::Idle,
            tasks: VecDeque::new(),
        }
    }

    pub fn state(&self) -> AssetTaskExecutorState {
        self.state
    }

    pub fn set_state(&mut self, state: AssetTaskExecutorState) {
        self.state = state;
    }

    pub fn push_back(&mut self, task: impl AssetTaskExecutor) {
        self.tasks.push_back(Box::new(task));
    }

    pub fn push_back_dyn(&mut self, task: Box<dyn AssetTaskExecutor>) {
        self.tasks.push_back(task);
    }

    pub fn push_front(&mut self, task: impl AssetTaskExecutor) {
        self.tasks.push_front(Box::new(task));
    }

    pub fn push_front_dyn(&mut self, task: Box<dyn AssetTaskExecutor>) {
        self.tasks.push_front(task);
    }

    pub fn pop(&mut self) -> Option<Box<dyn AssetTaskExecutor>> {
        self.tasks.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }
}

impl Default for AssetTaskExecutorQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub type AssetQueueShared = Arc<Mutex<AssetTaskExecutorQueue>>;
pub type AssetQueueRef<'a> = MutexGuard<'a, AssetTaskExecutorQueue>;

fn path_ext(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

pub struct ImportAsset {
    path: PathBuf,
}

impl ImportAsset {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn import(
        path: &PathBuf,
        database: &AssetDatabase,
        registry: &AssetRegistry,
    ) -> Result<HashSet<AssetId>, ImportFailed> {
        let ext = if path.is_dir() {
            Folder::EXT
        } else if let Some(ext) = path_ext(path) {
            ext
        } else {
            return Err(ImportFailed::import(
                path.clone(),
                AssetId::ZERO,
                "No extension found",
            ));
        };

        let meta = match registry.meta_by_ext(ext) {
            Some(meta) => meta,
            None => {
                let error = ImportFailed::import(path.clone(), AssetId::ZERO, "No importer found");
                return Err(error);
            }
        };

        let result = meta.import(path, database.config())?;
        let ImportResult { source, artifact } = result;

        let dependents = artifact.dependents().clone();

        let mut library = database.library_mut();

        Self::update_dependents(&artifact, &mut library);

        library.insert_source(path.clone(), source);
        library.insert_artifact(artifact.id(), artifact);

        Ok(dependents)
    }

    fn update_dependents(artifact: &ArtifactMeta, library: &mut AssetLibrary) {
        for dependencies in artifact.dependencies() {
            let dep_artifact = match library.artifact_mut(dependencies) {
                Some(artifact) => artifact,
                None => continue,
            };

            dep_artifact.add_dependent(artifact.id());
        }

        let removed = library.artifact(&artifact.id()).map(|prev| {
            let dependencies = prev.dependencies().difference(artifact.dependencies());
            dependencies.copied().collect::<Vec<_>>()
        });

        if let Some(removed) = removed {
            for id in removed {
                let artifact = match library.artifact_mut(&id) {
                    Some(artifact) => artifact,
                    None => continue,
                };

                artifact.remove_dependent(&artifact.id());
            }
        }
    }

    fn dependents(
        library: &AssetLibrary,
        dependents: impl Iterator<Item = AssetId>,
    ) -> Vec<PathBuf> {
        dependents
            .filter_map(|dep| {
                let artifact = library.artifact(&dep)?;
                Some(artifact.filepath().clone())
            })
            .collect::<Vec<_>>()
    }
}

impl AssetTaskExecutor for ImportAsset {
    fn execute(
        &self,
        database: &AssetDatabase,
        registry: &AssetRegistry,
        events: &mut EventStorage,
    ) {
        let path = database.config().asset(&self.path);
        let mut deps = match Self::import(&path, database, registry) {
            Ok(deps) => deps,
            Err(error) => {
                events.add(error);
                return;
            }
        };

        let _reload = Self::dependents(&database.library(), deps.drain());

        let mut tasks = database.tasks();
        tasks.push_front(SaveLibrary);
    }
}

impl Event for ImportAsset {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetTask::new(self));

        None
    }
}

pub struct ImportAssets {
    paths: Vec<PathBuf>,
}

impl ImportAssets {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }
}

impl AssetTaskExecutor for ImportAssets {
    fn execute(
        &self,
        database: &AssetDatabase,
        registry: &AssetRegistry,
        events: &mut EventStorage,
    ) {
        let mut dependents = DenseSet::new();
        for path in &self.paths {
            let path = database.config().asset(path);
            match ImportAsset::import(&path, database, registry) {
                Ok(deps) => dependents.extend(deps),
                Err(error) => events.add(error),
            }
        }

        let imports = ImportAsset::dependents(&database.library(), dependents.drain());

        let mut tasks = database.tasks();
        tasks.push_back(SaveLibrary);
        if !imports.is_empty() {
            tasks.push_front(ImportAssets::new(imports));
        }
    }
}

impl Event for ImportAssets {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetTask::new(self));

        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportScan {
    Added { path: PathBuf },
    Modified { path: PathBuf },
    Removed { path: PathBuf },
    Error { path: PathBuf, message: String },
}

impl ImportScan {
    pub fn path(&self) -> &PathBuf {
        match self {
            ImportScan::Added { path } => path,
            ImportScan::Modified { path } => path,
            ImportScan::Removed { path } => path,
            ImportScan::Error { path, .. } => path,
        }
    }

    pub fn value(&self) -> u8 {
        match self {
            ImportScan::Added { .. } => 0,
            ImportScan::Modified { .. } => 1,
            ImportScan::Removed { .. } => 2,
            ImportScan::Error { .. } => 3,
        }
    }
}

impl Ord for ImportScan {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value().cmp(&other.value())
    }
}

impl PartialOrd for ImportScan {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct ImportFolder {
    path: PathBuf,
}

impl ImportFolder {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl AssetTaskExecutor for ImportFolder {
    fn execute(
        &self,
        database: &AssetDatabase,
        registry: &AssetRegistry,
        events: &mut EventStorage,
    ) {
        let path = database.config().asset(&self.path);
        let mut scans = Self::scan_folder(&path, database);

        scans.sort();

        let mut dependents = DenseSet::new();
        let mut removed = Vec::new();
        for scan in scans {
            match scan {
                ImportScan::Added { path } | ImportScan::Modified { path } => {
                    match ImportAsset::import(&path, database, registry) {
                        Ok(deps) => dependents.extend(deps),
                        Err(error) => events.add(error),
                    }
                }
                ImportScan::Removed { path } => {
                    removed.push(path);
                }
                ImportScan::Error { path, message } => {
                    let id = database.library().source(&path).map(|s| s.id());
                    let error = ImportFailed::import(path, id.unwrap_or(AssetId::ZERO), message);
                    events.add(error);
                }
            }
        }

        let reload = ImportAsset::dependents(&database.library(), dependents.drain());

        let mut tasks = database.tasks();
        tasks.push_back(SaveLibrary);
        tasks.push_front(RemoveAssets::new(removed));
    }
}

impl ImportFolder {
    fn scan_file(path: &PathBuf, database: &AssetDatabase) -> Option<ImportScan> {
        let library = database.library();
        let metadata = match std::fs::read(database.config().metadata(path)) {
            Ok(content) => content,
            Err(_) => return Some(ImportScan::Added { path: path.clone() }),
        };

        let source = match library.source(path) {
            Some(source) => source,
            None => return Some(ImportScan::Added { path: path.clone() }),
        };

        let asset_modified = SourceInfo::modified(&path);
        let settings_modified = SourceInfo::modified(&database.config().metadata(path));

        if asset_modified > source.asset_modified()
            || settings_modified > source.settings_modified()
        {
            return Some(ImportScan::Modified { path: path.clone() });
        }

        let asset = match std::fs::read(path) {
            Ok(asset) => asset,
            Err(e) => {
                return Some(ImportScan::Error {
                    path: path.clone(),
                    message: e.to_string(),
                })
            }
        };

        let checksum = SourceInfo::calculate_checksum(&asset, &metadata);

        if checksum != source.checksum() {
            return Some(ImportScan::Modified { path: path.clone() });
        }

        None
    }

    fn scan_folder(path: &PathBuf, database: &AssetDatabase) -> Vec<ImportScan> {
        let mut scans = Vec::new();
        let mut metadata = match database.config().load_metadata::<FolderSettings>(path) {
            Ok(metadata) => metadata,
            Err(_) => AssetMetadata::<FolderSettings>::default(),
        };

        let mut children = HashSet::new();

        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) => {
                scans.push(ImportScan::Error {
                    path: path.clone(),
                    message: e.to_string(),
                });
                return scans;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let path = entry.path();
            match path_ext(&path) {
                Some("meta") | None => continue,
                _ => children.insert(path.clone()),
            };

            if path.is_dir() {
                scans.extend(Self::scan_folder(&path, database));
            } else if path.is_file() {
                scans.extend(Self::scan_file(&path, database));
            }
        }

        let removed = metadata.settings_mut().set_children(children);
        scans.extend(
            removed
                .iter()
                .map(|path| ImportScan::Removed { path: path.clone() }),
        );

        match database.config.save_metadata(path, &metadata) {
            Ok(_) => scans.push(ImportScan::Added { path: path.clone() }),
            Err(e) => scans.push(ImportScan::Error {
                path: path.clone(),
                message: e.to_string(),
            }),
        }

        scans
    }
}

impl Event for ImportFolder {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetTask::new(self));

        None
    }
}

pub struct RemoveAsset {
    path: PathBuf,
}

impl RemoveAsset {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn remove(path: &PathBuf, database: &AssetDatabase) -> Option<ArtifactMeta> {
        let mut library = database.library_mut();
        let source = library.remove_source(path)?;

        let _ = std::fs::remove_file(database.config().artifact(&source.id()));

        let artifact = library.remove_artifact(&source.id())?;

        for id in artifact.dependencies() {
            let artifact = match library.artifact_mut(id) {
                Some(artifact) => artifact,
                None => continue,
            };

            artifact.remove_dependent(&source.id());
        }

        Some(artifact)
    }
}

impl AssetTaskExecutor for RemoveAsset {
    fn execute(&self, database: &AssetDatabase, _: &AssetRegistry, _: &mut EventStorage) {
        let path = database.config().asset(&self.path);
        if let Some(artifact) = RemoveAsset::remove(&path, database) {
            //TODO: Unload asset
        }
    }
}

impl Event for RemoveAsset {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetTask::new(self));

        None
    }
}

pub struct RemoveAssets {
    paths: Vec<PathBuf>,
}

impl RemoveAssets {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }
}

impl AssetTaskExecutor for RemoveAssets {
    fn execute(&self, database: &AssetDatabase, _: &AssetRegistry, _: &mut EventStorage) {
        for path in &self.paths {
            let path = database.config().asset(path);
            if let Some(artifact) = RemoveAsset::remove(&path, database) {
                //TODO: Unload asset
            }
        }
    }
}

impl Event for RemoveAssets {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetTask::new(self));

        None
    }
}

pub struct SaveLibrary;

impl AssetTaskExecutor for SaveLibrary {
    fn execute(&self, database: &AssetDatabase, _: &AssetRegistry, events: &mut EventStorage) {
        let library = database.library();
        let config = database.config();

        if let Err(error) = library.save(config, false) {
            events.add(AssetLibraryError::new(error));
        }
    }
}

impl Event for SaveLibrary {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetTask::new(self));

        None
    }
}

pub struct LoadLibrary;

impl AssetTaskExecutor for LoadLibrary {
    fn execute(&self, database: &AssetDatabase, _: &AssetRegistry, events: &mut EventStorage) {
        let mut library = database.library_mut();
        let config = database.config();

        match AssetLibrary::load(config) {
            Ok(loaded) => library.replace(loaded),
            Err(error) => events.add(AssetLibraryError::new(error)),
        }
    }
}

impl Event for LoadLibrary {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetTask::new(self));

        None
    }
}
