use super::{AssetEvent, ImportAssets, ImportFolder, LoadAsset, RemoveAssets, UnloadAsset};
use crate::{
    asset::{Asset, AssetKind, AssetSettings, Settings},
    database::{library::DependentLibrary, AssetDatabase},
    io::PathExt,
    loader::{AssetError, AssetErrorKind, LoadErrorKind, LoadedAssets},
};
use shadow_ecs::{core::DenseSet, world::event::Events};
use std::{
    collections::HashSet,
    error::Error,
    path::{Path, PathBuf},
};

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

    pub fn priority(&self) -> u32 {
        match self {
            ImportScan::Error(_) => 0,
            ImportScan::Removed(_) => 1,
            ImportScan::Modified(_) => 2,
            ImportScan::Added(_) => 3,
        }
    }
}

impl ImportFolder {
    fn scan_file(path: &Path, database: &AssetDatabase) -> Option<ImportScan> {
        let io = database.io();
        let loaders = database.loaders();
        let library = database.library();
        let loader = path.ext().and_then(|ext| loaders.get_by_ext(ext))?;

        let metadata = match loader.load_metadata(path, io) {
            Ok(metadata) => metadata,
            Err(_) => return Some(ImportScan::added(path)),
        };

        if library
            .path(&metadata.id)
            .map(|source| source != path)
            .unwrap_or_default()
        {
            return Some(ImportScan::added(path));
        }

        let artifact_meta = match io.load_artifact_meta(metadata.id) {
            Ok(artifact_meta) => artifact_meta,
            Err(_) => return Some(ImportScan::added(path)),
        };

        let asset = {
            let mut reader = io.reader(path);
            match reader.read_to_end().and_then(|_| reader.flush()) {
                Ok(bytes) => bytes,
                Err(e) => return Some(ImportScan::error(path, e)),
            }
        };

        match io.checksum(&asset, metadata.data()) != artifact_meta.checksum() {
            true => Some(ImportScan::modified(path)),
            false => None,
        }
    }

    fn scan_folder(path: &Path, database: &AssetDatabase) -> Vec<ImportScan> {
        let io = database.io();
        let children = match io.reader(path).read_dir() {
            Ok(chidren) => chidren,
            Err(e) => return vec![ImportScan::error(path, e)],
        };

        let mut metadata = match io.load_metadata::<FolderMeta>(path) {
            Ok(metadata) => metadata,
            Err(_) => AssetSettings::<FolderMeta>::default(),
        };

        let mut scans = Vec::new();
        for child in &children {
            match io.filesystem().is_dir(&child) {
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

        if let Err(e) = io.save_metadata(path, &metadata) {
            scans.push(ImportScan::error(path, e));
        }

        scans
    }
}

impl AssetEvent for ImportFolder {
    fn execute(&self, database: &AssetDatabase, events: &Events) {
        let scans = Self::scan_folder(&self.path, database);

        let mut errors = vec![];
        let mut imports = vec![];
        let mut removed = vec![];

        for scan in scans {
            match scan {
                ImportScan::Added(path) | ImportScan::Modified(path) => imports.push(path),
                ImportScan::Removed(path) => removed.push(path),
                ImportScan::Error(error) => errors.push(error),
            }
        }

        database.events().push_front(ImportAssets::new(imports));
        database.events().push_front(RemoveAssets::new(removed));
        events.extend(errors);
    }
}

impl AssetEvent for ImportAssets {
    fn execute(&self, database: &AssetDatabase, events: &Events) {
        let mut assets = LoadedAssets::new();
        let mut errors = vec![];
        let mut imports = HashSet::new();
        let mut dependents = {
            let reader = database
                .io()
                .reader(database.io().temp().join("dependents.lib"));
            DependentLibrary::load(reader).unwrap_or(DependentLibrary::new())
        };
        let io = database.io();
        let loaders = database.loaders();
        let mut library = database.library_mut();

        for path in &self.paths {
            let loader = match path.ext().and_then(|ext| loaders.get_by_ext(ext)) {
                Some(loader) => loader,
                None => {
                    errors.push(AssetError::import(path, LoadErrorKind::NoLoader));
                    continue;
                }
            };

            let imported = match loader.import(path, &loaders, io, &mut assets) {
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
            }

            library.add_asset(imported.id(), path.to_path_buf(), AssetKind::Main);
            imports.insert(imported.id());
            assets.add_erased(imported.id(), imported.into());
        }

        let writer = io.writer(io.temp().join("dependents.lib"));
        dependents.save(writer).unwrap();

        let mut reimports = DenseSet::new();
        for id in &imports {
            if let Some(dependents) = dependents.get(id) {
                reimports.extend(dependents.iter().filter_map(|id| library.path(id)));
            }
        }

        let import = ImportAssets::new(reimports.drain().cloned().collect());
        database.events().push_front(import);
        events.extend(errors);
        events.extend(imports.drain().map(LoadAsset::soft).collect());
    }
}

impl AssetEvent for RemoveAssets {
    fn execute(&self, database: &AssetDatabase, events: &Events) {
        let mut dependents = {
            let reader = database
                .io()
                .reader(database.io().temp().join("dependents.lib"));
            DependentLibrary::load(reader).unwrap_or(DependentLibrary::new())
        };

        let mut reimports = DenseSet::new();
        let mut unloads = Vec::new();

        for path in &self.paths {
            let id = match database.library_mut().remove_path(path) {
                Some(id) => id,
                None => continue,
            };

            let artifact_meta = match database.io().load_artifact_meta(id) {
                Ok(artifact_meta) => artifact_meta,
                Err(_) => continue,
            };

            for dep in artifact_meta.dependencies() {
                dependents.remove_dependent(dep, &id);
            }

            let mut dependents = dependents.remove_asset(&id);
            reimports.extend(
                dependents
                    .drain()
                    .filter_map(|id| database.library().path(&id).cloned()),
            );

            let _ = database.io.remove_file(database.io().artifact(id));

            unloads.push(UnloadAsset::new(id));
        }

        let writer = database
            .io()
            .writer(database.io().temp().join("dependents.lib"));
        dependents.save(writer).unwrap();

        let import = ImportAssets::new(reimports.drain().collect());
        database.events().push_front(import);
        events.extend(unloads);
    }
}
