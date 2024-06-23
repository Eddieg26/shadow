use serde::{Deserialize, Serialize};
use shadow_ecs::ecs::event::{Event, EventStorage, Events};

use crate::{
    asset::{Asset, AssetId, AssetMetadata, AssetSettings, AssetType, Assets, Settings},
    block::{AssetBlock, MetadataBlock},
    bytes::ToBytes,
    database::{
        events::{
            AssetFailed, AssetRemoved, AssetSaved, ImportAsset, ImportDependency, ImportDependent,
            ImportFolder, ImportReason, PostProcessAsset, ProcessAsset,
        },
        library::{AssetLibrary, AssetStatus, BlockInfo, ImportStatus, SourceInfo},
        storage::AssetStorages,
        AssetDatabase,
    },
    errors::AssetError,
    loader::{
        AssetLoader, AssetPipeline, AssetPostProcessor, AssetProcessor, AssetSaver, LoadContext,
    },
    registry::AssetRegistry,
};
use std::{
    borrow::Cow,
    collections::HashSet,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub enum EntryScan {
    Added {
        path: PathBuf,
        source: SourceInfo,
    },
    Modified {
        path: PathBuf,
        source: SourceInfo,
        previous: SourceInfo,
        asset: Vec<u8>,
        metadata: MetadataBlock,
    },
    Removed {
        path: PathBuf,
    },
}

impl EntryScan {
    pub fn path(&self) -> &Path {
        match self {
            EntryScan::Added { path, .. } => path,
            EntryScan::Modified { path, .. } => path,
            EntryScan::Removed { path } => path,
        }
    }

    pub fn into_reason(self) -> Option<ImportReason> {
        match self {
            EntryScan::Added { path, source } => Some(ImportReason::Added { path, source }),
            EntryScan::Modified {
                path,
                source,
                previous,
                asset,
                metadata,
            } => Some(ImportReason::Modified {
                path,
                source,
                previous,
                asset,
                metadata,
            }),
            EntryScan::Removed { .. } => None,
        }
    }
}

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

    fn load(ctx: &mut LoadContext<Self::Settings>) -> Result<Self::Asset, String> {
        if let Some(path) = ctx.path() {
            let dirs = std::fs::read_dir(path).map_err(|e| e.to_string())?;
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
        &["folder::directory"]
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

fn meta_path(path: &Path) -> PathBuf {
    let mut path = path.to_path_buf();
    path.extend([".meta"].iter());
    path
}

fn scan_file(
    path: impl AsRef<Path>,
    library: &AssetLibrary,
    registry: &AssetRegistry,
) -> Option<EntryScan> {
    let path = path.as_ref();
    let ext = path_ext(path)?;

    if ext != "meta" {
        return None;
    }

    let loader = registry.meta_by_ext(ext)?;

    let metadata = match loader.load_meta(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return Some(EntryScan::Added {
                path: path.to_path_buf(),
                source: SourceInfo::default(),
            })
        }
    };

    let source = match library.source(path) {
        Some(source) => source,
        None => {
            return Some(EntryScan::Added {
                path: path.to_path_buf(),
                source: SourceInfo::new(metadata.id()),
            })
        }
    };

    if metadata.id() != source.id() {
        let asset = std::fs::read(path).ok()?;
        let checksum = SourceInfo::calculate_checksum(&asset, metadata.data());
        let modified = get_modified(path);
        let settings_modified = get_modified(meta_path(path));

        return Some(EntryScan::Modified {
            path: path.to_path_buf(),
            source: SourceInfo::from(metadata.id(), checksum, modified, settings_modified),
            previous: source,
            asset,
            metadata,
        });
    }

    let modified = get_modified(path);
    let settings_modified = get_modified(meta_path(path));
    if modified != source.modified() || settings_modified != source.settings_modified() {
        let asset = std::fs::read(path).ok()?;
        let checksum = SourceInfo::calculate_checksum(&asset, metadata.data());

        return Some(EntryScan::Modified {
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
        return Some(EntryScan::Modified {
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
) -> Vec<EntryScan> {
    let mut scans = vec![];

    let metadata = match database.load_metadata::<FolderSettings>(path) {
        Some(metadata) => metadata,
        None => {
            scans.push(EntryScan::Added {
                path: path.to_path_buf(),
                source: SourceInfo::default(),
            });
            return scans;
        }
    };

    let source = match database.source(path) {
        Some(source) => source,
        None => {
            scans.push(EntryScan::Added {
                path: path.to_path_buf(),
                source: SourceInfo::new(metadata.id()),
            });
            return scans;
        }
    };

    let mut meta_path = path.to_path_buf();
    meta_path.extend([".meta"].iter());
    let modified = get_modified(path);
    let settings_modified = get_modified(&&meta_path);
    if modified != source.modified() || settings_modified != source.settings_modified() {
        for child in metadata.settings().iter() {
            if !children.contains(child) {
                scans.push(EntryScan::Removed {
                    path: child.clone(),
                });
            }
        }

        let metadata = MetadataBlock::from(metadata);

        let checksum = SourceInfo::calculate_checksum(&[], metadata.data());
        scans.push(EntryScan::Modified {
            path: path.to_path_buf(),
            source: SourceInfo::from(metadata.id(), checksum, modified, settings_modified),
            previous: source,
            asset: vec![],
            metadata,
        });
    }

    scans
}

fn scan_dir(
    path: impl AsRef<Path>,
    database: &AssetDatabase,
    registry: &AssetRegistry,
) -> Option<Vec<EntryScan>> {
    let path = path.as_ref();
    let mut scans = vec![];
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
            scans.append(&mut dir_changes);
        } else {
            let change = scan_file(&path, &database.library, registry)?;
            scans.push(change);
        }
    }

    scans.append(&mut dir_changes(path, database, children));

    Some(scans)
}

pub fn import_folders(
    paths: &[<ImportFolder as Event>::Output],
    events: &Events,
    database: &AssetDatabase,
    registry: &AssetRegistry,
) {
    let mut scans = vec![];
    for path in paths {
        scans.append(&mut scan_dir(path, database, registry).unwrap_or_default());
    }

    let mut storage = EventStorage::new();
    for scan in scans {
        let loader = match if scan.path().is_dir() {
            registry.meta_by_ext("folder::directory")
        } else {
            path_ext(scan.path()).and_then(|ext| registry.meta_by_ext(ext))
        } {
            Some(loader) => loader,
            None => continue,
        };

        match scan {
            EntryScan::Added { path, source } => {
                loader.import_event(&mut storage, ImportReason::added(path, source))
            }
            EntryScan::Modified {
                path,
                source,
                previous,
                asset,
                metadata,
            } => {
                let reason = ImportReason::modified(path, source, previous, asset, metadata);
                loader.import_event(&mut storage, reason)
            }
            EntryScan::Removed { path } => events.add(AssetRemoved::new(path)),
        }
    }

    events.append(storage);
}

struct ImportResult<P: AssetPipeline> {
    asset: P::Asset,
    metadata: AssetMetadata<P::Settings>,
    source: SourceInfo,
}

impl<P: AssetPipeline> ImportResult<P> {
    fn new(asset: P::Asset, metadata: AssetMetadata<P::Settings>, source: SourceInfo) -> Self {
        ImportResult {
            asset,
            metadata,
            source,
        }
    }
}

type ImportInfo<P> = <ImportAsset<<P as AssetPipeline>::Asset> as Event>::Output;
pub fn import_asset<P: AssetPipeline>(
    imports: &[ImportInfo<P>],
    database: &AssetDatabase,
    storages: &AssetStorages,
    events: &Events,
) {
    let inner = |import: &ImportInfo<P>| -> Result<ImportResult<P>, String> {
        let path = import.path();
        let reason = import.reason();
        let (bytes, mut metadata) = match reason {
            ImportReason::Manual { path } => {
                let metadata = match database.load_metadata::<P::Settings>(path) {
                    Some(metadata) => metadata,
                    None => database
                        .source(path)
                        .and_then(|source| {
                            let metadata = AssetMetadata::new(source.id(), P::Settings::default());
                            Some(metadata)
                        })
                        .unwrap_or_else(|| AssetMetadata::default()),
                };

                let asset = match path.is_dir() {
                    true => Vec::new(),
                    false => std::fs::read(path).map_err(|e| e.to_string())?,
                };

                (Cow::Owned(asset), metadata)
            }
            ImportReason::Added { path, source } => {
                let metadata = AssetMetadata::new(source.id(), P::Settings::default());
                let asset = match path.is_dir() {
                    true => Vec::new(),
                    false => std::fs::read(path).map_err(|e| e.to_string())?,
                };

                (Cow::Owned(asset), metadata)
            }
            ImportReason::Modified {
                source,
                asset,
                metadata,
                ..
            } => {
                let metadata = match metadata.into_metadata::<P::Settings>() {
                    Some(metadata) => metadata,
                    None => AssetMetadata::new(source.id(), P::Settings::default()),
                };
                (Cow::Borrowed(asset), metadata)
            }
            ImportReason::DependencyModified { asset, .. } => {
                let metadata = match database.load_metadata::<P::Settings>(path) {
                    Some(metadata) => metadata,
                    None => AssetMetadata::new(*asset, P::Settings::default()),
                };

                let asset = match path.is_dir() {
                    true => Vec::new(),
                    false => std::fs::read(path).map_err(|e| e.to_string())?,
                };

                (Cow::Owned(asset), metadata)
            }
        };

        let mut ctx = LoadContext::unprocessed(path, &bytes, &mut metadata);
        let asset = P::Loader::load(&mut ctx)?;
        let dependencies = ctx.dependencies().to_vec();
        let dependents = database.library.dependents(&metadata.id());

        for dependent in dependents {
            events.add(ImportDependent::new(metadata.id(), dependent));
        }

        for dependency in dependencies {
            events.add(ImportDependency::new(metadata.id(), dependency));
            database.library.add_dependency(metadata.id(), dependency);
        }

        let data = toml::to_string(&metadata.settings()).map_err(|e| e.to_string())?;
        let checksum = SourceInfo::calculate_checksum(&bytes, data.as_bytes());
        let modified = get_modified(path);
        let settings_modified = SourceInfo::system_time_to_secs(SystemTime::now());
        let source = SourceInfo::from(metadata.id(), checksum, modified, settings_modified);

        let _ = std::fs::write(meta_path(path), &data).map_err(|e| e.to_string());

        Ok(ImportResult::new(asset, metadata, source))
    };

    for import in imports {
        let result = match inner(import) {
            Ok(result) => result,
            Err(e) => {
                let error = AssetError::Importing {
                    path: import.path().to_path_buf(),
                    message: e,
                };
                events.add(AssetFailed::<P::Asset>::new(error));
                continue;
            }
        };

        let ImportResult {
            asset,
            metadata,
            source,
        } = result;

        let block = BlockInfo::new(import.path(), AssetType::of::<P::Asset>());
        database.library.set_block(metadata.id(), block);
        database.library.set_source(import.path(), source);

        let (id, settings) = metadata.take();
        storages.insert::<P>(id, asset, settings);

        if dependencies_done(database, &id) {
            events.add(ProcessAsset::<P::Asset>::new(id));
        }
    }
}

pub fn process_assets<P: AssetPipeline>(
    assets: &[<ProcessAsset<P::Asset> as Event>::Output],
    storages: &AssetStorages,
    events: &Events,
    arg: &<P::Processor as AssetProcessor>::Arg,
) {
    storages.execute_mut::<P, _>(|storage| {
        for id in assets {
            let (asset, settings) = match storage.get_mut(id) {
                Some(data) => data,
                None => continue,
            };

            match P::Processor::process(asset, settings, arg) {
                Ok(_) => events.add(PostProcessAsset::<P::Asset>::new(*id)),
                Err(e) => {
                    let error = AssetError::Processing {
                        id: *id,
                        message: e.to_string(),
                    };
                    events.add(AssetFailed::<P::Asset>::new(error))
                }
            }
        }

        Some(())
    });
}

pub fn postprocess_assets<P: AssetPipeline>(
    assets: &[<ProcessAsset<P::Asset> as Event>::Output],
    database: &AssetDatabase,
    storages: &AssetStorages,
    events: &Events,
    arg: &<P::PostProcessor as AssetPostProcessor>::Arg,
) {
    storages.execute_mut::<P, _>(|storage| {
        for id in assets {
            let (asset, settings) = match storage.get_mut(id) {
                Some(data) => data,
                None => continue,
            };

            match P::PostProcessor::post_process(asset, settings, arg) {
                Ok(_) => {
                    let data = P::Saver::save(&asset);
                    let dependencies = database.library.dependencies(id);
                    let dependencies = dependencies.iter().copied().collect::<Vec<_>>();

                    let block = AssetBlock::new(data, settings, &dependencies);

                    let path = database.config().blocks().join(id.to_string());
                    match std::fs::write(&path, block.data()) {
                        Ok(_) => events.add(AssetSaved::<P::Asset>::new(*id, path)),
                        Err(e) => {
                            let error = AssetError::Saving {
                                id: *id,
                                path: path.clone(),
                                message: e.to_string(),
                            };
                            events.add(AssetFailed::<P::Asset>::new(error))
                        }
                    }
                }
                Err(e) => {
                    let error = AssetError::PostProcessing {
                        id: *id,
                        message: e.to_string(),
                    };
                    let event = AssetFailed::<P::Asset>::new(error);
                    events.add(event);
                }
            }
        }

        Some(())
    });
}

pub fn asset_saved<P: AssetPipeline>(
    assets: &[<AssetSaved<P::Asset> as Event>::Output],
    database: &AssetDatabase,
    registry: &AssetRegistry,
    storages: &AssetStorages,
    events: &Events,
) {
    for id in assets {
        let dependents = database.library.dependents(id);

        let process_list = dependents
            .iter()
            .filter_map(|dependent| {
                let block = match database.library.block(dependent) {
                    Some(block) => block,
                    None => return None,
                };
                match database.status::<ImportStatus>(block.filepath()) {
                    ImportStatus::Importing => {
                        if dependencies_done(database, dependent) {
                            let block = match database.library.block(dependent) {
                                Some(block) => block,
                                None => return None,
                            };
                            let loader = match registry.meta(block.ty()) {
                                Some(loader) => loader,
                                None => return None,
                            };

                            Some((dependent, loader))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>();

        for (dependent, loader) in &process_list {
            loader.process(events, **dependent);
        }

        if process_list.is_empty() {
            let (asset, settings) = match storages.remove::<P>(*id) {
                Some(data) => data,
                None => continue,
            };
        }
    }
}

fn dependencies_done(database: &AssetDatabase, asset: &AssetId) -> bool {
    let dependencies = database.library.dependencies(asset);
    dependencies.iter().all(|dependency| {
        let block = match database.library.block(dependency) {
            Some(block) => block,
            None => return false,
        };
        match database.status::<ImportStatus>(block.filepath()) {
            ImportStatus::Done | ImportStatus::Failed => true,
            _ => false,
        }
    })
}
