use super::{
    importer::{AssetStore, DependentUpdates, ImportError, LoadedAsset, SavedAsset},
    AssetDatabase,
};
use crate::{
    artifact::ArtifactMeta,
    asset::{Asset, AssetId, AssetMetadata, Settings},
    importer::{AssetError, LoadError},
    AssetConfig, AssetFileSystem, AssetIoError, AssetPath, Assets, IntoBytes, PathExt,
};
use shadow_ecs::{
    event::{Event, EventStorage, Events},
    storage::dense::DenseSet,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    vec,
};

pub trait AssetEvent: Send + Sync + 'static {
    fn execute(&self, db: &AssetDatabase, events: &Events);
}

impl<A: AssetEvent> From<A> for Box<dyn AssetEvent> {
    fn from(value: A) -> Self {
        Box::new(value)
    }
}

pub trait AssetEventExt {
    fn add_asset_event(&self, event: impl AssetEvent);
}

impl AssetEventExt for Events {
    fn add_asset_event(&self, event: impl AssetEvent) {
        self.add(StartAssetEvent::new(event));
    }
}

pub struct AssetEvents {
    events: VecDeque<Box<dyn AssetEvent>>,
    running: bool,
}

impl AssetEvents {
    pub fn new() -> AssetEvents {
        Self {
            events: VecDeque::new(),
            running: false,
        }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    pub fn push_front(&mut self, event: impl Into<Box<dyn AssetEvent>>) {
        self.events.push_front(event.into())
    }

    pub fn push_back(&mut self, event: impl Into<Box<dyn AssetEvent>>) {
        self.events.push_back(event.into())
    }

    pub fn pop(&mut self) -> Option<Box<dyn AssetEvent>> {
        let event = self.events.pop_front();
        event
    }
}

pub struct StartAssetEvent {
    event: Box<dyn AssetEvent>,
    front: bool,
}

impl StartAssetEvent {
    pub fn new(event: impl Into<Box<dyn AssetEvent>>) -> Self {
        Self {
            event: event.into(),
            front: false,
        }
    }

    pub fn push_front(mut self) -> Self {
        self.front = true;
        self
    }
}

impl Event for StartAssetEvent {
    type Output = ();

    fn invoke(self, world: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        match self.front {
            true => database.events().push_front(self.event),
            false => database.events().push_back(self.event),
        }

        Some(())
    }
}

impl AssetFileSystem {
    pub fn dependents(&self) -> PathBuf {
        self.config().temp().join("dependents")
    }

    pub fn asset_dependents(&self, id: AssetId) -> HashSet<AssetId> {
        let path = self.dependents().join(id.to_string());

        match self.read(&path) {
            Ok(bytes) => HashSet::<AssetId>::from_bytes(&bytes).unwrap_or_default(),
            Err(_) => HashSet::new(),
        }
    }

    pub fn save_dependents(
        &self,
        id: AssetId,
        dependents: HashSet<AssetId>,
    ) -> Result<(), AssetIoError> {
        let path = self.dependents().join(id.to_string());
        let bytes = dependents.into_bytes();
        self.write(&path, &bytes)
    }

    pub fn remove_dependents(&self, id: &AssetId) -> Result<(), AssetIoError> {
        let path = self.dependents().join(id.to_string());
        self.remove(&path).map(|_| ())
    }
}

pub struct Folder;
impl Asset for Folder {}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct FolderSettings {
    children: HashSet<PathBuf>,
}

impl FolderSettings {
    pub fn new() -> Self {
        FolderSettings {
            children: HashSet::new(),
        }
    }

    pub fn children(&self) -> &HashSet<PathBuf> {
        &self.children
    }

    pub fn set_children(&mut self, children: impl IntoIterator<Item = PathBuf>) {
        self.children.extend(children);
    }
}

impl Settings for FolderSettings {}

pub enum EntryScan {
    Added(PathBuf),
    Removed(PathBuf),
    Modified(PathBuf),
    Error(ImportError),
}

impl EntryScan {
    pub fn path(&self) -> &Path {
        match self {
            EntryScan::Added(path) => &path,
            EntryScan::Removed(path) => &path,
            EntryScan::Modified(path) => &path,
            EntryScan::Error(error) => &error.path,
        }
    }

    pub fn priority(&self) -> usize {
        match self {
            EntryScan::Error(_) => 0,
            EntryScan::Removed(_) => 1,
            EntryScan::Modified(_) => 2,
            EntryScan::Added(_) => 3,
        }
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

    pub fn scan_file(path: &Path, fs: &AssetFileSystem, db: &AssetDatabase) -> Option<EntryScan> {
        let importers = db.importers();
        let importer = path.ext().and_then(|ext| importers.get_by_ext(ext))?;

        let metapath = AssetConfig::metadata(path);
        if !metapath.exists() {
            return Some(EntryScan::Added(path.to_path_buf()));
        }

        let metadata = match importer.load_metadata(&metapath, fs) {
            Ok(metadata) => metadata,
            Err(_) => return Some(EntryScan::Added(path.to_path_buf())),
        };

        {
            let mut library = db.library_mut();
            match library.id_path(&metadata.id) {
                Some(prev_path) => {
                    if prev_path != path {
                        library.add(metadata.id, path.to_path_buf());
                        return Some(EntryScan::Modified(path.to_path_buf()));
                    }
                }
                None => return Some(EntryScan::Added(path.to_path_buf())),
            }
        }

        let artifact = match fs.load_artifact_meta(&metadata.id) {
            Ok(artifact) => artifact,
            Err(_) => return Some(EntryScan::Added(path.to_path_buf())),
        };

        let modified = AssetFileSystem::modified_secs(path).unwrap_or_default();
        if modified != artifact.modified() {
            return Some(EntryScan::Modified(path.to_path_buf()));
        }

        let bytes = match fs.read(path) {
            Ok(bytes) => bytes,
            Err(e) => return Some(EntryScan::Error(ImportError::new(path, e))),
        };

        let checksum = AssetFileSystem::calculate_checksum(&bytes, &metadata.data);
        if checksum != artifact.checksum() {
            return Some(EntryScan::Modified(path.to_path_buf()));
        }

        None
    }

    fn import_folder(path: &Path, db: &AssetDatabase) -> Vec<EntryScan> {
        let fs = db.filesystem();
        let children = match fs.read_directory(path, false) {
            Ok(mut children) => children.drain(..).collect::<HashSet<_>>(),
            Err(e) => return vec![EntryScan::Error(ImportError::new(path, e))],
        };

        let mut scans = vec![];
        let mut metadata = match fs.load_metadata::<FolderSettings>(path) {
            Ok(metadata) => metadata,
            Err(_) => AssetMetadata::default(),
        };

        for path in &children {
            if path.is_dir() {
                scans.extend(Self::import_folder(path, db));
            } else if !matches!(path.ext(), Some("meta")) {
                scans.extend(Self::scan_file(path, fs, db))
            }
        }

        for path in metadata.children().difference(&children) {
            scans.push(EntryScan::Removed(path.clone()))
        }

        metadata.set_children(children);

        if let Err(e) = fs.save_metadata(path, &metadata) {
            scans.push(EntryScan::Error(ImportError::new(path, e)));
        }

        scans
    }
}

impl AssetEvent for ImportFolder {
    fn execute(&self, db: &AssetDatabase, events: &Events) {
        let path = db.filesystem().config().assets().join(&self.path);
        let mut scans = Self::import_folder(&path, db);
        scans.sort_by(|a, b| a.priority().cmp(&b.priority()));

        let mut errors = vec![];
        let mut removed = vec![];
        let mut imports = vec![];

        for scan in scans {
            match scan {
                EntryScan::Removed(path) => removed.push(path),
                EntryScan::Error(error) => errors.push(error),
                EntryScan::Added(p) | EntryScan::Modified(p) => imports.push(p),
            }
        }

        db.events().push_front(ImportAssets::new(imports));
        db.events().push_front(RemoveAssets::new(removed));
        events.extend(errors);
    }
}

impl Event for ImportFolder {
    type Output = ();

    fn invoke(self, world: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        world.events().add(StartAssetEvent::new(self));

        None
    }
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
}

impl Event for ImportAsset {
    type Output = PathBuf;

    fn invoke(self, world: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        let db = world.resource::<AssetDatabase>();
        let path = db.filesystem().config().assets().join(self.path);
        Some(path)
    }
}

pub struct ImportResult {
    pub imported: DenseSet<AssetId>,
    pub errors: Vec<ImportError>,
}

impl ImportResult {
    pub fn new(imported: DenseSet<AssetId>, errors: Vec<ImportError>) -> Self {
        Self { imported, errors }
    }
}

pub struct ImportAssets {
    paths: Vec<PathBuf>,
}

impl ImportAssets {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }

    fn import<P: AsRef<Path>>(
        paths: impl IntoIterator<Item = P>,
        db: &AssetDatabase,
        assets: &mut AssetStore,
    ) -> ImportResult {
        let fs = db.filesystem();
        let mut imported = DenseSet::new();
        let mut dep_updates = HashMap::new();
        let mut errors = vec![];

        for path in paths {
            let saved = match Self::import_asset(path.as_ref(), fs, db, assets) {
                Ok(saved) => saved,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };

            for id in saved.meta.dependencies() {
                let updates = dep_updates.entry(*id).or_insert(DependentUpdates::new());
                updates.add(saved.meta.id());
            }

            for id in &saved.removed_dependencies {
                let updates = dep_updates.entry(*id).or_insert(DependentUpdates::new());
                updates.remove(saved.meta.id());
            }

            db.library_mut()
                .add(saved.meta.id(), path.as_ref().to_path_buf());

            imported.insert(saved.meta.id());
            assets.insert(saved.meta.id(), LoadedAsset::from(saved));
        }

        for (id, updates) in dep_updates {
            let _ = Self::update_dependents(id, fs, &updates);
        }

        let mut dependents = HashSet::new();
        for id in &imported {
            let deps = fs.asset_dependents(*id);

            for dep in deps {
                if let Some(path) = db.library().id_path(&dep) {
                    dependents.insert(path.clone());
                }
            }
        }

        let result = Self::import(dependents, db, assets);
        imported.extend(result.imported);
        errors.extend(result.errors);

        ImportResult::new(imported, errors)
    }

    fn import_asset(
        path: &Path,
        fs: &AssetFileSystem,
        db: &AssetDatabase,
        assets: &mut AssetStore,
    ) -> Result<SavedAsset, ImportError> {
        let ext = path
            .ext()
            .ok_or(ImportError::new(path, AssetError::InvalidExtension))?;

        let importers = db.importers();
        let importer = importers
            .get_by_ext(ext)
            .ok_or(ImportError::new(path, AssetError::NoImporter))?;

        let mut imported = importer.import(fs, path)?;

        if let Some(process) = importer.process {
            Self::load_dependencies(imported.artifact.dependencies(), fs, db, assets);
            process(path, &mut imported, assets)?;
        }

        importer.save(fs, path, imported)
    }

    fn load_dependencies<'a>(
        ids: impl IntoIterator<Item = &'a AssetId>,
        fs: &AssetFileSystem,
        db: &AssetDatabase,
        assets: &mut AssetStore,
    ) {
        let importers = db.importers();

        for id in ids {
            if assets.contains(id) {
                return;
            }

            let artifact = match fs.load_artifact(id) {
                Ok(artifact) => artifact,
                Err(_) => continue,
            };

            let importer = match importers.get(artifact.meta().ty()) {
                Some(importer) => importer,
                None => continue,
            };

            match importer.load(artifact) {
                Ok(loaded) => assets.insert(*id, loaded),
                Err(_) => continue,
            };
        }
    }

    fn update_dependents(
        id: AssetId,
        fs: &AssetFileSystem,
        updates: &DependentUpdates,
    ) -> Result<(), AssetIoError> {
        let mut dependents = fs.asset_dependents(id);
        dependents.extend(updates.added());
        dependents.retain(|id| !updates.removed().contains(id));

        match dependents.is_empty() {
            true => fs.remove_dependents(&id),
            false => fs.save_dependents(id, dependents),
        }
    }
}

impl AssetEvent for ImportAssets {
    fn execute(&self, db: &AssetDatabase, events: &Events) {
        let mut assets = AssetStore::new();

        let ImportResult { imported, errors } = Self::import(&self.paths, db, &mut assets);
        let reloads = imported.into_iter().map(|id| LoadRequest::soft(id));

        events.extend(errors);
        db.events().push_back(LoadAssets::new(reloads.collect()));
    }
}

impl Event for ImportAssets {
    type Output = ();

    fn invoke(self, world: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        world.events().add(StartAssetEvent::new(self));

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

impl AssetEvent for RemoveAssets {
    fn execute(&self, db: &AssetDatabase, events: &Events) {
        let fs = db.filesystem();
        let mut updates = HashMap::new();
        let mut removed = vec![];

        for path in &self.paths {
            let id = match db.library().path_id(path) {
                Some(id) => *id,
                None => continue,
            };

            if let Ok(artifact) = fs.load_artifact_meta(&id) {
                for dependency in artifact.dependencies() {
                    let updates = updates
                        .entry(*dependency)
                        .or_insert(DependentUpdates::new());
                    updates.remove(id);
                }
            }

            db.library_mut().remove(&id);
            fs.remove_dependents(&id).ok();
            fs.remove_artifact(&id).ok();
            removed.push(AssetRemoved::new(id));
        }

        for (id, updates) in updates.drain() {
            let _ = ImportAssets::update_dependents(id, fs, &updates);
        }

        events.extend(removed);
    }
}

impl Event for RemoveAssets {
    type Output = ();

    fn invoke(self, world: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        world.events().add(StartAssetEvent::new(self));

        None
    }
}

pub struct AssetRemoved {
    id: AssetId,
}

impl AssetRemoved {
    pub fn new(id: AssetId) -> Self {
        Self { id }
    }
}

impl Event for AssetRemoved {
    type Output = AssetId;

    fn invoke(self, _: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        Some(self.id)
    }
}

#[derive(Debug, Clone)]
pub struct LoadRequest {
    pub id: AssetId,
    pub load_dependencies: bool,
}

impl LoadRequest {
    pub fn new(id: AssetId, load_dependencies: bool) -> Self {
        Self {
            id,
            load_dependencies,
        }
    }

    pub fn soft(id: AssetId) -> Self {
        Self::new(id, false)
    }

    pub fn hard(id: AssetId) -> Self {
        Self::new(id, true)
    }
}

pub struct LoadAsset {
    path: AssetPath,
}

impl LoadAsset {
    pub fn new(path: impl Into<AssetPath>) -> Self {
        Self { path: path.into() }
    }
}

impl From<AssetId> for LoadRequest {
    fn from(value: AssetId) -> Self {
        Self::new(value, true)
    }
}

impl Event for LoadAsset {
    type Output = AssetId;

    fn invoke(self, world: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        match self.path {
            AssetPath::Id(id) => Some(id),
            AssetPath::Path(path) => {
                let db = world.resource::<AssetDatabase>();
                let path = db.filesystem().config().assets().join(path);
                let id = db.library().path_id(&path).copied();
                id
            }
        }
    }
}

pub struct LoadAssets {
    requests: Vec<LoadRequest>,
}

impl LoadAssets {
    pub fn new<I: Into<LoadRequest>>(requests: Vec<I>) -> Self {
        let requests = requests.into_iter().map(Into::into).collect();
        Self { requests }
    }

    pub fn load_asset(request: &LoadRequest, db: &AssetDatabase) -> Result<LoadedAsset, LoadError> {
        let id = request.id;
        let fs = db.filesystem();

        let artifact = fs.load_artifact(&id).map_err(|e| LoadError::new(id, e))?;
        let importers = db.importers();
        let importer = importers
            .get(artifact.meta().ty())
            .ok_or(LoadError::new(id, AssetError::NoImporter))?;

        importer.load(artifact).map_err(|e| LoadError::new(id, e))
    }

    pub fn load<'a>(
        requests: impl IntoIterator<Item = &'a LoadRequest>,
        db: &AssetDatabase,
        assets: &mut AssetStore,
    ) -> Vec<LoadError> {
        let mut dependencies = Vec::new();
        let mut errors = vec![];

        for request in requests {
            if assets.contains(&request.id) {
                continue;
            }

            let asset = match Self::load_asset(request, db) {
                Ok(asset) => asset,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };

            if request.load_dependencies {
                let deps = asset.meta().dependencies().iter().filter_map(|id| {
                    (!assets.contains(id) && (!db.loaded(id))).then_some(LoadRequest::hard(*id))
                });
                dependencies.extend(deps);
            }

            assets.insert(request.id, asset);
        }

        errors.extend(Self::load(dependencies.iter(), db, assets));

        errors
    }
}

impl AssetEvent for LoadAssets {
    fn execute(&self, db: &AssetDatabase, events: &Events) {
        let mut assets = AssetStore::new();
        let errors = Self::load(&self.requests, db, &mut assets);

        let mut loaded = vec![];
        let mut loaded_events = EventStorage::new();
        for asset in assets.drain() {
            let importers = db.importers();
            let importer = importers.get(asset.meta().ty()).unwrap();

            loaded.push(asset.meta().id());
            importer.asset_loaded(asset, &mut loaded_events);
        }

        let dependents = db.states().dependents(loaded.iter());
        let reloads = dependents.iter().map(|id| LoadRequest::soft(*id));

        events.extend(errors);
        events.extend(loaded_events.into());
        events.add_asset_event(LoadAssets::new(reloads.collect()));
    }
}

pub struct AssetLoaded<A: Asset> {
    asset: A,
    meta: ArtifactMeta,
}

impl<A: Asset> AssetLoaded<A> {
    pub fn new(asset: A, meta: ArtifactMeta) -> Self {
        AssetLoaded { asset, meta }
    }
}

impl<A: Asset> Event for AssetLoaded<A> {
    type Output = AssetId;

    fn invoke(self, world: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        let id = self.meta.id();
        let assets = world.resource_mut::<Assets<A>>();
        assets.insert(id, self.asset);

        let db = world.resource::<AssetDatabase>();
        db.states_mut()
            .load(id, self.meta.ty(), self.meta.dependencies);

        Some(id)
    }
}

pub struct UnloadAsset<A: Asset> {
    id: AssetId,
    _phantom: std::marker::PhantomData<A>,
}

impl<A: Asset> UnloadAsset<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Event for UnloadAsset<A> {
    type Output = AssetUnload<A>;

    fn invoke(self, world: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        let assets = world.resource_mut::<Assets<A>>();
        let asset = assets.remove(&self.id)?;

        let db = world.resource::<AssetDatabase>();
        let state = db.states_mut().unload(&self.id)?;
        let dependencies = state.dependencies().iter().copied().collect();

        Some(AssetUnload::new(self.id, asset, dependencies))
    }
}

pub struct AssetUnload<A: Asset> {
    id: AssetId,
    asset: A,
    dependencies: Vec<AssetId>,
}

impl<A: Asset> AssetUnload<A> {
    pub fn new(id: AssetId, asset: A, dependencies: Vec<AssetId>) -> Self {
        Self {
            id,
            asset,
            dependencies,
        }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn asset(self) -> A {
        self.asset
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }
}
