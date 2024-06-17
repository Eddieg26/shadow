use super::{
    events::{AssetErrorExt, AssetFailed, ImportAsset, ImportFolder, ImportInfo, ImportReason},
    library::AssetStatus,
};
use crate::{
    asset::{AssetId, AssetMetadata, AssetType, Folder, FolderSettings, Settings},
    block::AssetBlock,
    bytes::ToBytes,
    database::{
        library::{BlockInfo, SourceInfo},
        AssetDatabase,
    },
    errors::AssetError,
    loader::{AssetLoader, AssetPipeline, AssetPostProcessor, AssetProcessor, LoadContext},
    registry::AssetPipelineRegistry,
};
use shadow_ecs::ecs::{
    event::{Event, Events},
    task::TaskManager,
};
use std::{borrow::Cow, collections::HashSet, path::Path};

fn path_ext(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

pub fn import_folders(
    paths: &[<ImportFolder as Event>::Output],
    events: &Events,
    database: &AssetDatabase,
    registry: &AssetPipelineRegistry,
    tasks: &TaskManager,
) {
    let mut paths = paths.to_vec();
    let events = events.clone();
    let database = database.clone();
    let registry = registry.clone();

    tasks.spawn(move || {
        while let Some(path) = paths.pop() {
            let entries = match path.read_dir() {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            let source = match database.source(&path) {
                Some(source) => match database.status(source.id()) {
                    AssetStatus::Importing => continue,
                    _ => source,
                },
                None => SourceInfo::default(),
            };

            database
                .library
                .set_status(source.id(), AssetStatus::Importing);

            let mut path_metadata = match database
                .load_metadata(&path)
                .ok()
                .and_then(|d| d.into::<FolderSettings>())
            {
                Some(metadata) => metadata.with_id(source.id()),
                None => {
                    AssetMetadata::<FolderSettings>::new(source.id(), FolderSettings::default())
                }
            };

            let mut children = HashSet::new();

            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                let path = &entry.path();
                children.insert(path.to_path_buf());

                if path.is_dir() {
                    paths.push(path.to_path_buf());
                } else if let Some(loader) =
                    path_ext(path).and_then(|ext| registry.meta_by_ext(ext))
                {
                    let metadata = match database.load_metadata(path).ok() {
                        Some(metadata) => metadata,
                        None => {
                            loader.import(&database, &events, ImportReason::added(path));
                            continue;
                        }
                    };

                    let mut source = match database.source(path) {
                        Some(source) => source,
                        None => {
                            loader.import(&database, &events, ImportReason::added(path));
                            continue;
                        }
                    };

                    if metadata.id() != source.id() {
                        source.set_id(metadata.id());
                        database.library.set_source(path.clone(), source);
                        let asset = match std::fs::read(path) {
                            Ok(asset) => asset,
                            Err(_) => continue,
                        };
                        let (_, metadata) = metadata.take();
                        let reason = ImportReason::asset_modified(path, asset, metadata);
                        loader.import(&database, &events, reason);
                        continue;
                    }

                    let modified = database.config().modified(path);

                    if modified != source.modified() {
                        let asset = match std::fs::read(path) {
                            Ok(asset) => asset,
                            Err(_) => continue,
                        };
                        let (_, metadata) = metadata.take();
                        let reason = ImportReason::asset_modified(path, asset, metadata);
                        loader.import(&database, &events, reason);
                        continue;
                    }

                    let asset = match std::fs::read(path) {
                        Ok(asset) => asset,
                        Err(_) => continue,
                    };

                    let checksum = SourceInfo::calculate_checksum(&asset, metadata.data());
                    if checksum != source.checksum() {
                        let (_, metadata) = metadata.take();
                        let reason = ImportReason::asset_modified(path, asset, metadata);
                        loader.import(&database, &events, reason);
                    }
                }
            }

            database.block(&path_metadata.id()).unwrap_or_else(|| {
                let block = BlockInfo::new(path.clone(), AssetType::of::<Folder>(), vec![]);
                database
                    .library
                    .set_block(path_metadata.id(), block.clone());
                block
            });

            path_metadata.settings().iter().for_each(|child| {
                if !children.contains(child) {
                    database.library.remove_source(&path);
                }
            });

            path_metadata.settings_mut().set_children(children);

            database
                .save_metadata::<Folder, FolderSettings>(path, &[], &path_metadata)
                .ok();
            database.library.set_status(source.id(), AssetStatus::None);
        }
    });
}

pub fn import_assets<P: AssetPipeline>(
    imports: &[<ImportAsset<P::Asset> as Event>::Output],
    events: &Events,
    database: &AssetDatabase,
    registry: &AssetPipelineRegistry,
    tasks: &TaskManager,
) {
    let imports = imports.to_vec();
    let events = events.clone();
    let database = database.clone();
    let registry = registry.clone();

    tasks.spawn(move || {
        for info in &imports {
            match import_asset::<P>(info, &database) {
                Ok(id) => {
                    database.library.set_status(id, AssetStatus::Imported);
                    database.import_dependents(id, &registry);
                }
                Err(event) => events.add(event),
            }
        }
    });
}

fn import_asset<P: AssetPipeline>(
    import: &ImportInfo,
    database: &AssetDatabase,
) -> Result<AssetId, AssetFailed<P::Asset>> {
    let path = import.path();
    let reason = import.reason();

    let mut metadata =
        load_metadata::<P::Settings>(database, reason).map_err(|e| e.into_event(path))?;

    let bytes = match reason {
        ImportReason::AssetModified { asset, .. } => Cow::Borrowed(asset),
        _ => Cow::Owned(std::fs::read(path).map_err(|e| e.into_event(path))?),
    };

    let (mut asset, dependencies) = {
        let mut ctx = LoadContext::new(path, &bytes, &mut metadata);
        let asset = P::Loader::load(&mut ctx);
        (asset, ctx.dependencies().to_vec())
    };

    P::Processor::process(&mut asset, &mut metadata);
    P::PostProcessor::post_process(&mut asset, &mut metadata);

    database
        .save_metadata::<P::Asset, P::Settings>(path, &bytes, &metadata)
        .map_err(|e| e.into_event(path))?;

    let asset = AssetBlock::new(&asset, metadata.settings());
    database
        .save_asset::<P::Asset, P::Settings>(path, &asset, &metadata, dependencies)
        .map_err(|e| e.into_event(path))?;

    Ok(metadata.id())
}

fn load_metadata<S: Settings>(
    database: &AssetDatabase,
    reason: &ImportReason,
) -> Result<AssetMetadata<S>, AssetError> {
    match reason {
        ImportReason::AssetModified { metadata, .. } => {
            AssetMetadata::from_bytes(&metadata).ok_or(AssetError::InvalidMetadata)
        }
        ImportReason::Added { .. } => Ok(AssetMetadata::<S>::new(AssetId::gen(), S::default())),
        ImportReason::DependencyModified { asset, .. } => database
            .library
            .block(asset)
            .ok_or(AssetError::AssetNotFound(*asset))
            .and_then(|info| {
                database
                    .load_metadata(info.filepath())
                    .map_err(|e| e.into())
            })
            .and_then(|metadata| metadata.into::<S>().ok_or(AssetError::InvalidMetadata)),
        ImportReason::Manual { path } => {
            let metadata = database
                .load_metadata(path)
                .and_then(|metadata| {
                    metadata
                        .into::<S>()
                        .ok_or(AssetError::InvalidMetadata.into())
                })
                .unwrap_or_default();
            Ok(metadata)
        }
    }
}
