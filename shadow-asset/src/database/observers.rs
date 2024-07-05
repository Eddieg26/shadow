use super::{
    config::AssetConfig,
    events::{ImportAsset, RemoveAsset},
    library::{AssetLibraryRef, AssetLibraryRefMut, DependencyInfo, DependencyMap, SourceInfo},
    registry::AssetLoaderRegistry,
    AssetDatabase,
};
use crate::{
    artifact::ArtifactMeta,
    asset::{Asset, AssetId, AssetMetadata, AssetType, Settings},
    bytes::ToBytes,
    loader::{AssetStorage, ImportError, LoadError},
};
use shadow_ecs::ecs::{
    event::{EventStorage, Events},
    storage::dense::DenseSet,
};
use std::{collections::HashSet, path::PathBuf};

fn path_ext(path: &PathBuf) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportScan {
    Added { path: PathBuf },
    Modified { path: PathBuf },
    Removed { path: PathBuf },
}

impl ImportScan {
    pub fn path(&self) -> &PathBuf {
        match self {
            ImportScan::Added { path } => path,
            ImportScan::Modified { path } => path,
            ImportScan::Removed { path } => path,
        }
    }

    pub fn value(&self) -> u8 {
        match self {
            ImportScan::Added { .. } => 0,
            ImportScan::Modified { .. } => 1,
            ImportScan::Removed { .. } => 2,
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

pub struct Folder;

impl Asset for Folder {}

impl ToBytes for Folder {
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn from_bytes(_: &[u8]) -> Option<Self> {
        Some(Folder)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FolderSettings {
    children: HashSet<PathBuf>,
}

impl FolderSettings {
    pub fn new() -> FolderSettings {
        FolderSettings {
            children: HashSet::new(),
        }
    }

    pub fn children(&self) -> std::collections::hash_set::Iter<PathBuf> {
        self.children.iter()
    }

    pub fn set_children(&mut self, children: HashSet<PathBuf>) -> Vec<PathBuf> {
        let removed = self.children.difference(&children).cloned().collect();
        self.children = children;

        removed
    }

    pub fn has_child(&self, path: &PathBuf) -> bool {
        self.children.contains(path)
    }
}

impl Settings for FolderSettings {}

fn scan_file(
    path: &PathBuf,
    database: &AssetDatabase,
    library: &AssetLibraryRef,
) -> Option<ImportScan> {
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

    if asset_modified > source.asset_modified() || settings_modified > source.settings_modified() {
        return Some(ImportScan::Modified { path: path.clone() });
    }

    let asset = match std::fs::read(path) {
        Ok(asset) => asset,
        Err(_) => return None,
    };

    let checksum = SourceInfo::calculate_checksum(&asset, &metadata);

    if checksum != source.checksum() {
        return Some(ImportScan::Modified { path: path.clone() });
    }

    None
}

fn scan_folder(
    path: &PathBuf,
    database: &AssetDatabase,
    library: &AssetLibraryRef,
    registry: &AssetLoaderRegistry,
) -> Vec<ImportScan> {
    let mut scans = Vec::new();

    let mut metadata = match std::fs::read_to_string(database.config().metadata(path)) {
        Ok(content) => {
            toml::from_str::<AssetMetadata<FolderSettings>>(&content).unwrap_or_default()
        }
        Err(_) => AssetMetadata::<FolderSettings>::default(),
    };

    let mut children = HashSet::new();

    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return scans,
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();
        children.insert(path.clone());

        if path.is_dir() {
            scans.extend(scan_folder(&path, database, library, registry));
        } else if path.is_file() {
            scans.extend(scan_file(&path, database, library));
        }
    }

    let removed = metadata.settings_mut().set_children(children);
    scans.extend(
        removed
            .iter()
            .map(|path| ImportScan::Removed { path: path.clone() }),
    );

    match toml::to_string(&metadata)
        .map(|data| std::fs::write(database.config().metadata(path), data))
    {
        Ok(_) => {
            scans.push(ImportScan::Added { path: path.clone() });
            scans
        }
        Err(_) => scans,
    }
}

pub fn import_folders(
    paths: &[PathBuf],
    database: &AssetDatabase,
    registry: &AssetLoaderRegistry,
    events: &Events,
) {
    let mut scans = Vec::new();
    let library = database.library();

    for path in paths {
        scans.extend(scan_folder(path, database, &library, &registry));
    }

    scans.sort();

    let mut _events = EventStorage::new();
    for scan in scans {
        match scan {
            ImportScan::Added { path } => _events.add(ImportAsset::new(path)),
            ImportScan::Modified { path } => _events.add(ImportAsset::new(path)),
            ImportScan::Removed { path } => _events.add(RemoveAsset::new(path)),
        }
    }

    events.append(_events);
}

fn import_asset(
    path: &PathBuf,
    ty: &AssetType,
    config: &AssetConfig,
    registry: &AssetLoaderRegistry,
    library: &mut AssetLibraryRefMut,
    assets: &mut AssetStorage,
    map: &mut DependencyMap,
) -> Result<ArtifactMeta, ImportError> {
    let meta = registry.meta(ty).ok_or(LoadError::LoaderNotFound)?;

    let artifact = meta.import(path, config, registry, library, assets, map)?;

    meta.process(&artifact.id(), assets)
        .map_err(ImportError::new)?;

    meta.save(&artifact.id(), &assets)
        .map_err(ImportError::new)?;

    Ok(artifact)
}

fn import_dependents(
    mut dependents: DenseSet<AssetId>,
    config: &AssetConfig,
    registry: &AssetLoaderRegistry,
    library: &mut AssetLibraryRefMut,
    assets: &mut AssetStorage,
    map: &mut DependencyMap,
    imported: &mut DenseSet<PathBuf>,
) {
    let mut new = DenseSet::new();
    for dependent in dependents.drain().collect::<Vec<_>>() {
        let (ty, path) = match library.artifact(&dependent) {
            Some(artifact) => (artifact.ty(), artifact.filepath().clone()),
            None => continue,
        };

        let artifact = match import_asset(&path, &ty, &config, &registry, library, assets, map) {
            Ok(artifact) => artifact,
            Err(_) => continue,
        };

        if let Some(info) = map.get(&artifact.id()) {
            new.extend(info.dependents().cloned());
            remove_dependencies(info, library, registry, imported, map, assets);
        }

        imported.insert(artifact.filepath().clone());

        library.insert_artifact(artifact.id(), artifact);
    }

    if !new.is_empty() {
        import_dependents(new, config, registry, library, assets, map, imported);
    }
}

fn remove_dependencies(
    info: &DependencyInfo,
    library: &AssetLibraryRefMut,
    registry: &AssetLoaderRegistry,
    imported: &DenseSet<PathBuf>,
    map: &DependencyMap,
    assets: &mut AssetStorage,
) {
    for dependency in info.dependencies() {
        let artifact = match library.artifact(dependency) {
            Some(artifact) => artifact,
            None => continue,
        };

        let meta = match registry.meta(&artifact.ty()) {
            Some(meta) => meta,
            None => continue,
        };

        let mut dependents = match map.get(dependency) {
            Some(info) => info.dependents(),
            None => continue,
        };

        if dependents.all(|dep| {
            let artifact = match library.artifact(dep) {
                Some(artifact) => artifact,
                None => return true,
            };

            imported.contains(artifact.filepath())
        }) {
            assets.remove_asset_by_ty(dependency, &artifact.ty());
            assets.remove_settings_by_ty(dependency, &meta.settings_ty());
        }
    }
}

fn import_assets(paths: &[PathBuf], database: &AssetDatabase, registry: &AssetLoaderRegistry) {
    let config = database.config();
    let mut library = database.library_mut();
    let mut assets = AssetStorage::new();
    let mut map = DependencyMap::new();
    let mut dependents = DenseSet::new();
    let mut imported = DenseSet::new();

    for path in paths {
        let ty = match path_ext(path).and_then(|ext| registry.ext_ty(ext)) {
            Some(ty) => ty,
            None => continue,
        };

        let artifact = match import_asset(
            path,
            &ty,
            &config,
            &registry,
            &mut library,
            &mut assets,
            &mut map,
        ) {
            Ok(artifact) => artifact,
            Err(_) => continue,
        };

        if let Some(info) = map.get(&artifact.id()) {
            dependents.extend(info.dependents().cloned());

            remove_dependencies(info, &library, registry, &imported, &map, &mut assets);
        }

        imported.insert(artifact.filepath().clone());

        library.insert_artifact(artifact.id(), artifact);
    }

    import_dependents(
        dependents,
        &config,
        &registry,
        &mut library,
        &mut assets,
        &mut map,
        &mut imported,
    );

    for (id, info) in map.drain() {
        let old = match std::fs::read(config.dependency_map().join(id.to_string())) {
            Ok(bytes) => DependencyInfo::from_bytes(&bytes).unwrap_or_default(),
            Err(_) => DependencyInfo::default(),
        };

        let removed = old
            .dependencies()
            .filter(|dep| !info.dependencies().contains(dep))
            .cloned()
            .collect::<Vec<_>>();

        for id in removed {
            
        }
    }
}
