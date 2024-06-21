use serde::{Deserialize, Serialize};
use shadow_ecs::ecs::event::{Event, EventStorage, Events};

use super::{
    events::{ImportAsset, ImportFolder, ImportReason},
    library::{AssetLibrary, SourceInfo},
    AssetDatabase,
};
use crate::{
    asset::{Asset, Settings},
    block::MetadataBlock,
    bytes::ToBytes,
    loader::{AssetLoader, AssetPipeline},
    registry::AssetPipelineRegistry,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub struct Folder;

impl Asset for Folder {}

impl ToBytes for Folder {
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            Some(Folder)
        } else {
            None
        }
    }
}

impl AssetLoader for Folder {
    type Asset = Self;
    type Settings = FolderSettings;

    fn load(
        ctx: &mut crate::loader::LoadContext<Self::Settings>,
    ) -> Result<Self::Asset, crate::errors::AssetError> {
        if let Some(path) = ctx.path() {
            let dirs = std::fs::read_dir(path)
                .map_err(|_| crate::errors::AssetError::AssetNotFound(ctx.metadata().id()))?;
            for entry in dirs {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                let path = entry.path();
                ctx.metadata().settings_mut().insert(path);
            }
        }

        Ok(Folder)
    }

    fn extensions() -> &'static [&'static str] {
        &[]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FolderSettings {
    children: HashSet<PathBuf>,
}

impl FolderSettings {
    pub fn new() -> Self {
        FolderSettings {
            children: HashSet::new(),
        }
    }

    pub fn insert(&mut self, path: PathBuf) -> bool {
        self.children.insert(path)
    }

    pub fn remove(&mut self, path: &Path) -> bool {
        self.children.remove(path)
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.children.contains(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.children.iter()
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&PathBuf) -> bool,
    {
        self.children.retain(|path| f(path));
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl ToBytes for FolderSettings {
    fn to_bytes(&self) -> Vec<u8> {
        self.children.iter().cloned().collect::<Vec<_>>().to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let children = Vec::<PathBuf>::from_bytes(bytes)?;
        let mut settings = FolderSettings::new();
        for child in children {
            settings.insert(child);
        }
        Some(settings)
    }
}

impl Settings for FolderSettings {}

fn get_modified(path: impl AsRef<Path>) -> u64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn path_ext(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

fn scan_file(
    path: impl AsRef<Path>,
    library: &AssetLibrary,
    registry: &AssetPipelineRegistry,
) -> Option<ImportReason> {
    let path = path.as_ref();
    let ext = path_ext(path)?;

    if ext != "meta" {
        return None;
    }

    let loader = registry.meta_by_ext(ext)?;

    let metadata = match loader.load_meta(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return Some(ImportReason::Added {
                path: path.to_path_buf(),
                source: SourceInfo::default(),
            })
        }
    };

    let source = match library.source(path) {
        Some(source) => source,
        None => {
            return Some(ImportReason::Added {
                path: path.to_path_buf(),
                source: SourceInfo::new(metadata.id()),
            })
        }
    };

    if metadata.id() != source.id() {
        let asset = std::fs::read(path).ok()?;
        let checksum = SourceInfo::calculate_checksum(&asset, metadata.data());
        let modified = get_modified(path);
        let settings_modified = get_modified(path.with_extension("meta"));

        return Some(ImportReason::Modified {
            path: path.to_path_buf(),
            source: SourceInfo::from(metadata.id(), checksum, modified, settings_modified),
            previous: source,
            asset,
            metadata,
        });
    }

    let modified = get_modified(path);
    let settings_modified = get_modified(path.with_extension("meta"));
    if modified != source.modified() || settings_modified != source.settings_modified() {
        let asset = std::fs::read(path).ok()?;
        let checksum = SourceInfo::calculate_checksum(&asset, metadata.data());

        return Some(ImportReason::Modified {
            path: path.to_path_buf(),
            source: SourceInfo::from(metadata.id(), checksum, modified, settings_modified),
            previous: source,
            asset,
            metadata,
        });
    }

    let asset = std::fs::read(path).ok()?;
    let checksum = SourceInfo::calculate_checksum(&asset, metadata.data());
    if checksum != source.checksum() {
        return Some(ImportReason::Modified {
            path: path.to_path_buf(),
            source: SourceInfo::from(metadata.id(), checksum, modified, settings_modified),
            previous: source,
            asset,
            metadata,
        });
    }

    None
}

fn dir_changes(
    path: &Path,
    database: &AssetDatabase,
    children: HashSet<PathBuf>,
) -> Vec<ImportReason> {
    let mut reasons = vec![];

    let mut metadata = match database.load_metadata::<FolderSettings>(path) {
        Some(metadata) => metadata,
        None => {
            reasons.push(ImportReason::Added {
                path: path.to_path_buf(),
                source: SourceInfo::default(),
            });
            return reasons;
        }
    };

    let source = match database.source(path) {
        Some(source) => source,
        None => {
            let source = SourceInfo::new(metadata.id());
            reasons.push(ImportReason::Added {
                path: path.to_path_buf(),
                source,
            });
            return reasons;
        }
    };

    let mut meta_path = path.to_path_buf();
    meta_path.extend([".meta"].iter());
    let modified = get_modified(path);
    let settings_modified = get_modified(&&meta_path);
    if modified != source.modified() || settings_modified != source.settings_modified() {
        for child in metadata.settings().iter() {
            if !children.contains(child) {
                reasons.push(ImportReason::Removed {
                    path: child.clone(),
                    id: metadata.id(),
                });
            }
        }

        let metadata = MetadataBlock::from(metadata);

        let checksum = SourceInfo::calculate_checksum(&[], metadata.data());
        reasons.push(ImportReason::Modified {
            path: path.to_path_buf(),
            source: SourceInfo::from(metadata.id(), checksum, modified, settings_modified),
            previous: source,
            asset: vec![],
            metadata,
        });
    }

    reasons
}

fn scan_dir(
    path: impl AsRef<Path>,
    database: &AssetDatabase,
    registry: &AssetPipelineRegistry,
) -> Option<Vec<ImportReason>> {
    let path = path.as_ref();
    let mut reasons = vec![];
    let mut children = HashSet::new();

    let entry = std::fs::read_dir(path).ok()?;

    for entry in entry {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();
        children.insert(path.clone());

        if path.is_dir() {
            let mut dir_changes = scan_dir(&path, database, registry)?;
            reasons.append(&mut dir_changes);
        } else {
            let change = scan_file(&path, &database.library, registry)?;
            reasons.push(change);
        }
    }

    reasons.append(&mut dir_changes(path, database, children));

    Some(reasons)
}

pub fn import_folders(
    paths: &[<ImportFolder as Event>::Output],
    events: &Events,
    database: &AssetDatabase,
    registry: &AssetPipelineRegistry,
) {
    let mut reasons = vec![];
    for path in paths {
        reasons.append(&mut scan_dir(path, database, registry).unwrap_or_default());
    }

    let mut storage = EventStorage::new();
    for reason in reasons {
        let path = reason.path();
        if path.is_dir() {
            storage.add(ImportAsset::<Folder>::new(path).with_reason(reason));
        } else if let Some(loader) = path_ext(&path).and_then(|ext| registry.meta_by_ext(ext)) {
            storage.push(loader.import_event(database, reason))
        }
    }

    events.append(storage);
}

pub fn import_asset<P: AssetPipeline>(
    imports: &[<ImportAsset<P::Asset> as Event>::Output],
    database: &AssetDatabase,
) {
    for import in imports {
        let path = import.path();

        match import.reason() {
            ImportReason::Manual { path } => todo!(),
            ImportReason::Added { path, source } => todo!(),
            ImportReason::Modified {
                path,
                source,
                previous,
                asset,
                metadata,
            } => todo!(),
            _ => {}
        }
    }
}
