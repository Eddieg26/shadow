use super::{
    load::{LoadAssets, UnloadAsset},
    AssetEvent, StartAssetEvent,
};
use crate::{
    asset::{Asset, AssetId, AssetKind, AssetSettings, Settings},
    database::{library::DependentLibrary, AssetDatabase},
    importer::{AssetError, AssetErrorKind, LoadErrorKind, LoadedAssets},
    io::PathExt,
    AssetPath,
};
use ecs::{
    core::DenseSet,
    world::{
        event::{Event, Events},
        World,
    },
};
use std::{
    collections::HashSet,
    error::Error,
    path::{Path, PathBuf},
};

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Folder;
impl Asset for Folder {}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct FolderMeta {
    children: HashSet<PathBuf>,
}

impl FolderMeta {
    pub fn new() -> Self {
        Self {
            children: HashSet::new(),
        }
    }

    pub fn children(&self) -> &HashSet<PathBuf> {
        &self.children
    }

    pub fn set_children(&mut self, children: impl IntoIterator<Item = PathBuf>) {
        self.children = children.into_iter().collect();
    }
}

impl Settings for FolderMeta {}

pub enum ImportScan {
    Added(PathBuf),
    Removed(PathBuf),
    Modified(PathBuf),
    Error(AssetError),
}

impl ImportScan {
    pub fn added(path: impl AsRef<Path>) -> Self {
        Self::Added(path.as_ref().to_path_buf())
    }

    pub fn removed(path: impl AsRef<Path>) -> Self {
        Self::Removed(path.as_ref().to_path_buf())
    }

    pub fn modified(path: impl AsRef<Path>) -> Self {
        Self::Modified(path.as_ref().to_path_buf())
    }

    pub fn error(path: impl AsRef<Path>, error: impl Error + Send + Sync + 'static) -> Self {
        Self::Error(AssetError::import(path, error))
    }

    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            ImportScan::Added(path) => Some(path),
            ImportScan::Removed(path) => Some(path),
            ImportScan::Modified(path) => Some(path),
            ImportScan::Error(error) => match error.kind() {
                AssetErrorKind::Load(_) => None,
                AssetErrorKind::Import(path) => Some(path),
            },
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
}

impl Event for ImportFolder {
    type Output = PathBuf;

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetEvent::new(self));

        None
    }
}

impl ImportFolder {
    fn scan_file(path: &Path, database: &AssetDatabase) -> Option<ImportScan> {
        let ext = match path.ext() {
            Some("meta") | None => return None,
            Some(ext) => ext,
        };
        let config = database.config();
        let registry = database.registry();
        let library = database.library();
        let loader = match registry.get_metadata_by_ext(ext) {
            Some(loader) => loader,
            None => return Some(ImportScan::error(path, LoadErrorKind::NoImporter)),
        };

        let metadata = match loader.load_metadata(path, config) {
            Some(Ok(metadata)) => metadata,
            _ => return Some(ImportScan::added(path)),
        };

        if library
            .path(&metadata.id)
            .map(|source| source != path)
            .unwrap_or_default()
        {
            return Some(ImportScan::added(path));
        }

        let artifact_meta = match config.load_artifact_meta(metadata.id) {
            Ok(artifact_meta) => artifact_meta,
            Err(_) => return Some(ImportScan::added(path)),
        };

        let asset = {
            let mut reader = config.reader(path);
            match reader.read_to_end().and_then(|_| reader.flush()) {
                Ok(bytes) => bytes,
                Err(e) => return Some(ImportScan::error(path, e)),
            }
        };

        match config.checksum(&asset, metadata.data()) != artifact_meta.checksum() {
            true => Some(ImportScan::modified(path)),
            false => None,
        }
    }

    fn scan_folder(path: &Path, database: &AssetDatabase) -> Vec<ImportScan> {
        let config = database.config();
        let children = match config.reader(path).read_dir() {
            Ok(children) => children,
            Err(e) => return vec![ImportScan::error(path, e)],
        };

        let mut metadata = match config.load_metadata::<FolderMeta>(path) {
            Ok(metadata) => metadata,
            Err(_) => AssetSettings::<FolderMeta>::default(),
        };

        let mut scans = Vec::new();
        for child in &children {
            match config.filesystem().is_dir(&child) {
                true => scans.extend(Self::scan_folder(child, database)),
                false => scans.extend(Self::scan_file(child, database)),
            }
        }

        for child in metadata.children() {
            if !children.contains(child) {
                scans.push(ImportScan::removed(child));
            }
        }

        metadata.set_children(children);

        if let Err(e) = config.save_metadata(path, &metadata) {
            scans.push(ImportScan::error(path, e));
        }

        scans
    }
}

impl AssetEvent for ImportFolder {
    fn execute(&mut self, database: &AssetDatabase, events: &Events) {
        let config = database.config();
        let path = self.path.with_prefix(config.root().join(config.assets()));
        let scans = Self::scan_folder(&path, database);

        let mut errors = vec![];
        let mut imports = vec![];
        let mut removed = vec![];

        for scan in scans {
            match scan {
                ImportScan::Added(path) | ImportScan::Modified(path) => imports.push(path),
                ImportScan::Removed(path) => removed.push(DeleteAsset::new(path)),
                ImportScan::Error(error) => errors.push(error),
            }
        }

        if !imports.is_empty() {
            database.events().push_front(ImportAssets::new(imports));
        }
        if !removed.is_empty() {
            database.events().push_front(DeleteAssets::new(removed));
        }
        events.extend(errors);
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

    pub fn observer(paths: &[PathBuf], events: &Events) {
        events.add(ImportAssets::new(paths.to_vec()));
    }
}

impl Event for ImportAsset {
    type Output = PathBuf;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self.path)
    }
}

pub struct ImportAssets {
    paths: Vec<PathBuf>,
}

impl ImportAssets {
    pub fn new(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            paths: paths.into_iter().collect(),
        }
    }
}

impl Event for ImportAssets {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetEvent::new(self));
        None
    }
}

impl AssetEvent for ImportAssets {
    fn execute(&mut self, database: &AssetDatabase, events: &Events) {
        let config = database.config();
        let registry = database.registry();
        let batch_size = database.config().import_batch_size();

        let mut paths = std::mem::take(&mut self.paths);
        let mut errors = vec![];
        let mut imports = vec![];
        let mut removed = vec![];
        let mut dependents = DependentLibrary::load(config).unwrap_or_default();

        while !paths.is_empty() {
            let paths = paths.drain(..batch_size.min(paths.len()));
            let mut assets = LoadedAssets::new();

            for path in paths {
                let path = path
                    .without_prefix(config.root().join(config.assets()))
                    .to_path_buf();

                let loader = match path.ext().and_then(|ext| registry.get_metadata_by_ext(ext)) {
                    Some(loader) => loader,
                    None => {
                        errors.push(AssetError::import(path, LoadErrorKind::NoImporter));
                        continue;
                    }
                };

                let imported = match loader.import(&path, config, &mut assets) {
                    Ok(imported) => imported,
                    Err(error) => {
                        errors.push(error);
                        continue;
                    }
                };

                for id in imported.dependencies() {
                    dependents.add_dependent(*id, imported.id());
                }

                if let Some(prev_meta) = imported.prev_meta() {
                    for id in prev_meta.dependencies().difference(imported.dependencies()) {
                        dependents.remove_dependent(id, &imported.id());
                    }

                    for child in prev_meta.children().difference(imported.meta().children()) {
                        removed.push(DeleteAsset::new(child));
                    }
                }

                let mut library = database.library_mut();
                library.add_asset(imported.id(), path.clone(), AssetKind::Main);

                for child in imported.meta().children() {
                    library.add_asset(*child, path.clone(), AssetKind::Sub);
                }

                imports.push(AssetImported::new(imported.id(), path));
                assets.add_erased(imported.id(), imported.into());
            }
        }

        if let Err(e) = dependents.save(config) {
            errors.push(AssetError::import(DependentLibrary::path(config), e));
        }

        let library = database.library();
        let mut reimports = DenseSet::new();
        let mut reloads = DenseSet::new();
        for import in &imports {
            if let Some(dependents) = dependents.get(&import.id()) {
                let dependents = dependents.iter().filter_map(|id| library.path(id).cloned());
                reimports.extend(dependents);
            }

            if database.states().is_loaded(&import.id()) {
                reloads.insert(import.id());
            }
        }

        if !reloads.is_empty() {
            database.events().push_front(LoadAssets::soft(reloads));
        }
        if !reimports.is_empty() {
            database.events().push_front(ImportAssets::new(reimports));
        }

        events.extend(errors);
        events.extend(imports);
    }
}

#[derive(Debug, Clone)]
pub struct DeleteAsset {
    path: AssetPath,
}

impl DeleteAsset {
    pub fn new(path: impl Into<AssetPath>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn observer(assets: &[Self], events: &Events) {
        events.add(DeleteAssets::new(assets.to_vec()));
    }
}

impl<I: Into<AssetPath>> From<I> for DeleteAsset {
    fn from(path: I) -> Self {
        Self::new(path)
    }
}

impl Event for DeleteAsset {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct DeleteAssets {
    assets: Vec<DeleteAsset>,
}

impl DeleteAssets {
    pub fn new(assets: impl IntoIterator<Item = impl Into<DeleteAsset>>) -> Self {
        Self {
            assets: assets.into_iter().map(Into::into).collect(),
        }
    }
}

impl Event for DeleteAssets {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetEvent::new(self));
        None
    }
}

impl AssetEvent for DeleteAssets {
    fn execute(&mut self, database: &AssetDatabase, events: &Events) {
        let config = database.config();
        let mut dependents = DependentLibrary::load(config).unwrap_or_default();
        let mut reimports = DenseSet::new();
        let mut unloads = Vec::new();
        let mut removed = Vec::new();

        for asset in &self.assets {
            let (id, path) = match &asset.path {
                AssetPath::Id(id) => match database.library().path(id) {
                    Some(path) => (*id, path.clone()),
                    None => continue,
                },
                AssetPath::Path(path) => {
                    let id = match database.library().id(&path).copied() {
                        Some(id) => id,
                        None => continue,
                    };

                    let path = path
                        .without_prefix(config.root().join(config.assets()))
                        .to_path_buf();

                    (id, path)
                }
            };

            database.library_mut().remove_asset(&id);

            let artifact_meta = match config.load_artifact_meta(id) {
                Ok(artifact_meta) => artifact_meta,
                Err(_) => continue,
            };

            let _ = config.remove_file(config.artifact(id));

            for dep in artifact_meta.dependencies() {
                dependents.remove_dependent(dep, &id);
            }

            for child in artifact_meta.children() {
                removed.push(DeleteAsset::new(child));
            }

            let mut dependents = dependents.remove_asset(&id);
            let dependents = dependents
                .drain()
                .filter_map(|id| database.library().path(&id).cloned());
            reimports.extend(dependents);

            reimports.remove(&path);
            unloads.push(UnloadAsset::new(id));
        }

        if let Err(e) = dependents.save(config) {
            events.add(AssetError::import(DependentLibrary::path(config), e));
        }

        if !removed.is_empty() {
            database.events().push_front(DeleteAssets::new(removed));
        }

        if !reimports.is_empty() {
            database.events().push_front(ImportAssets::new(reimports));
        }
        events.extend(unloads);
    }
}

pub struct AssetImported {
    id: AssetId,
    path: PathBuf,
}

impl AssetImported {
    pub fn new(id: AssetId, path: impl AsRef<Path>) -> Self {
        Self {
            id,
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Event for AssetImported {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}
