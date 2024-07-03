use crate::{
    asset::{Asset, AssetId, AssetType, Settings},
    bytes::ToBytes,
};

use super::{
    events::{ImportAsset, RemoveAsset},
    library::{Artifacts, Sources},
    pipeline::{AssetLoader, AssetPipeline, AssetRegistry, BasicProcessor, LoadContext},
    tracker::ImportStatus,
    AssetDatabase,
};
use shadow_ecs::ecs::{
    event::{Event, EventStorage, Events},
    storage::dense::DenseSet,
    task::TaskManager,
};
use std::{collections::HashSet, path::PathBuf};
use utils::EntryScan;

mod utils {
    use std::{
        collections::HashSet,
        path::{Path, PathBuf},
    };

    use crate::{
        asset::AssetMetadata,
        database::{
            library::{SourceInfo, Sources},
            pipeline::AssetRegistry,
        },
    };

    use super::FolderSettings;

    #[derive(Debug, PartialEq, Eq)]
    pub enum EntryScan {
        Added { path: PathBuf },
        Removed { path: PathBuf },
        Modified { path: PathBuf },
    }

    impl Into<u8> for EntryScan {
        fn into(self) -> u8 {
            match self {
                EntryScan::Added { .. } => 2,
                EntryScan::Removed { .. } => 1,
                EntryScan::Modified { .. } => 0,
            }
        }
    }

    impl Into<PathBuf> for EntryScan {
        fn into(self) -> PathBuf {
            match self {
                EntryScan::Added { path } => path,
                EntryScan::Removed { path } => path,
                EntryScan::Modified { path } => path,
            }
        }
    }

    impl From<&EntryScan> for u8 {
        fn from(scan: &EntryScan) -> u8 {
            match scan {
                EntryScan::Added { .. } => 2,
                EntryScan::Removed { .. } => 1,
                EntryScan::Modified { .. } => 0,
            }
        }
    }

    impl Ord for EntryScan {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            u8::from(self).cmp(&other.into())
        }
    }

    impl PartialOrd for EntryScan {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    pub fn path_ext(path: &Path) -> Option<&str> {
        path.extension().and_then(|ext| ext.to_str())
    }

    pub fn scan_file(
        path: &Path,
        sources: &Sources,
        registry: &AssetRegistry,
    ) -> Option<EntryScan> {
        let meta_path = PathBuf::from(format!("{}.meta", path.display()));
        let pipeline = path_ext(path).and_then(|ext| registry.pipeline_by_ext(ext))?;

        let metadata = match pipeline.load_metablock(path) {
            Ok(metadata) => metadata,
            Err(_) => {
                return Some(EntryScan::Added {
                    path: path.to_path_buf(),
                })
            }
        };

        let source = {
            match sources.get(&path.to_path_buf()) {
                Some(source) => source,
                None => {
                    return Some(EntryScan::Added {
                        path: path.to_path_buf(),
                    });
                }
            }
        };

        if metadata.id() != source.id() {
            return Some(EntryScan::Modified {
                path: path.to_path_buf(),
            });
        }

        let asset_modified = SourceInfo::modified(path);
        let settings_modified = SourceInfo::modified(&meta_path);

        if source.asset_modified() != asset_modified
            || source.settings_modified() != settings_modified
        {
            return Some(EntryScan::Modified {
                path: path.to_path_buf(),
            });
        }

        let asset = match std::fs::read(&path) {
            Ok(asset) => asset,
            Err(_) => return None,
        };

        let checksum = SourceInfo::calculate_checksum(&asset, metadata.data());
        if source.checksum() != checksum {
            return Some(EntryScan::Modified {
                path: path.to_path_buf(),
            });
        }

        None
    }

    pub fn scan_folder(path: &Path, sources: &Sources, registry: &AssetRegistry) -> Vec<EntryScan> {
        let mut scans = vec![];
        let mut paths = vec![path.to_path_buf()];

        while let Some(path) = paths.pop() {
            let dir = match std::fs::read_dir(&path) {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let meta_path = format!("{}.meta", path.display());
            let mut children = HashSet::new();
            let mut metadata = match std::fs::read_to_string(&meta_path) {
                Ok(ref content) => {
                    toml::from_str::<AssetMetadata<FolderSettings>>(content).unwrap_or_default()
                }
                Err(_) => AssetMetadata::<FolderSettings>::default(),
            };

            for entry in dir {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                let path = entry.path();
                match path_ext(&path) {
                    Some("meta") => continue,
                    _ => children.insert(path.clone()),
                };

                if path.is_dir() {
                    paths.push(path);
                } else {
                    scans.extend(scan_file(&path, sources, registry));
                }
            }

            for child in metadata.settings().children.iter() {
                if !children.contains(child) {
                    scans.push(EntryScan::Removed {
                        path: child.clone(),
                    });
                }
            }

            metadata.settings_mut().set_children(children);

            let metadata = match toml::to_string(&metadata) {
                Ok(content) => content,
                Err(_) => continue,
            };

            let _ = std::fs::write(meta_path, metadata);
        }

        scans
    }
}

pub struct Folder;

impl AssetPipeline for Folder {
    type Loader = Self;
    type Processor = BasicProcessor<
        <Self::Loader as AssetLoader>::Asset,
        <Self::Loader as AssetLoader>::Settings,
    >;
    type Saver = Self;
}

impl AssetLoader for Folder {
    type Asset = Folder;
    type Settings = FolderSettings;

    fn load(_: &mut LoadContext<Self::Settings>) -> Result<Folder, super::pipeline::AssetError> {
        Ok(Folder)
    }

    fn extensions() -> &'static [&'static str] {
        &[]
    }
}

impl ToBytes for Folder {
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn from_bytes(_: &[u8]) -> Option<Self> {
        Some(Folder)
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FolderSettings {
    children: HashSet<PathBuf>,
}

impl FolderSettings {
    pub fn children(&self) -> &HashSet<PathBuf> {
        &self.children
    }

    pub fn set_children(&mut self, children: HashSet<PathBuf>) {
        self.children = children;
    }
}

impl ToBytes for FolderSettings {
    fn to_bytes(&self) -> Vec<u8> {
        self.children.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(FolderSettings {
            children: HashSet::from_bytes(bytes)?,
        })
    }
}

impl Settings for FolderSettings {}

impl Asset for Folder {}

pub fn import_folders(
    paths: &[PathBuf],
    database: &AssetDatabase,
    registry: &AssetRegistry,
    events: &Events,
) {
    let mut scans = vec![];
    let sources = database.library().sources();

    for path in paths {
        scans.extend(utils::scan_folder(path, &sources, registry));
    }

    scans.sort();

    let mut event_storage = EventStorage::new();
    for scan in scans {
        match scan {
            EntryScan::Added { path } => event_storage.add(ImportAsset::new(path)),
            EntryScan::Removed { path } => event_storage.add(ImportAsset::new(path)),
            EntryScan::Modified { path } => event_storage.add(RemoveAsset::new(path)),
        }
    }

    events.append(event_storage);
}

pub fn import_asset(
    path: &PathBuf,
    database: &AssetDatabase,
    registry: &AssetRegistry,
    artifacts: &mut Artifacts,
    sources: &mut Sources,
    events: &mut EventStorage,
    dependents: &mut DenseSet<AssetId>,
) {
    let pipeline = match utils::path_ext(&path) {
        Some(ext) => registry.pipeline_by_ext(ext),
        None => registry.pipeline(&AssetType::of::<Folder>()),
    };

    let pipeline = match pipeline {
        Some(pipeline) => pipeline,
        None => {
            database.set_import_status(path.clone().into(), ImportStatus::Failed);
            return;
        }
    };

    match pipeline.import(path, database, sources, artifacts) {
        Ok(info) => {
            dependents.extend(info.dependents().iter().cloned());
            dependents.remove(&info.id());
            artifacts.insert(info.id(), info);
        }
        Err(error) => {
            pipeline.import_failed(path.clone(), error, events);
            return;
        }
    }
}

pub fn import_assets(
    imports: &[<ImportAsset as Event>::Output],
    database: &AssetDatabase,
    registry: &AssetRegistry,
    events: &Events,
    tasks: &TaskManager,
) {
    let events = events.clone();
    let database = database.clone();
    let registry = registry.clone();
    let imports = imports.iter().cloned().collect::<DenseSet<_>>();

    tasks.spawn(move || {
        {
            let mut event_storage = EventStorage::new();
            let mut artifacts = database.library().artifacts();
            let mut sources = database.library().sources();
            let mut dependents = DenseSet::new();
            for path in imports.values() {
                import_asset(
                    path,
                    &database,
                    &registry,
                    &mut artifacts,
                    &mut sources,
                    &mut event_storage,
                    &mut dependents,
                );
            }

            while !dependents.is_empty() {
                for id in dependents.drain().collect::<Vec<_>>() {
                    let (pipeline, path) = match artifacts.get(&id).and_then(|a| {
                        registry
                            .pipeline(&a.ty())
                            .map(|p| (p, a.filepath().clone()))
                    }) {
                        Some(pipeline) => pipeline,
                        None => continue,
                    };

                    match database.import_status(&path) {
                        ImportStatus::None => (),
                        _ => continue,
                    }

                    match pipeline.import(&path, &database, &mut sources, &mut artifacts) {
                        Ok(info) => {
                            dependents.extend(info.dependents().iter().cloned());
                            dependents.remove(&info.id());
                            artifacts.insert(info.id(), info);
                        }
                        Err(error) => {
                            pipeline.import_failed(path, error, &mut event_storage);
                            continue;
                        }
                    }
                }
            }

            events.append(event_storage);
        }

        let _ = database
            .library()
            .save(&database.config().library().to_path_buf());

        database.clear_import_statuses();
    });
}
