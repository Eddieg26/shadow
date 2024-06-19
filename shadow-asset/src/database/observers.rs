use super::events::{
    AssetErrorExt, AssetFailed, AssetImported, FolderImported, ImportAsset, ImportFolder,
    ImportInfo, ImportReason,
};
use crate::{
    asset::{AssetMetadata, AssetType, Folder, FolderSettings, Settings},
    block::AssetBlock,
    database::{
        library::{BlockInfo, SourceInfo},
        AssetDatabase,
    },
    loader::{AssetLoader, AssetPipeline, AssetPostProcessor, AssetProcessor, LoadContext},
    registry::AssetPipelineRegistry,
};
use shadow_ecs::ecs::{
    event::{Event, Events},
    task::TaskManager,
};
use std::{borrow::Cow, collections::HashSet, path::Path};

pub enum ImportError {
    UnknownExtension,
    MetadataNotFound,
    AssetModified { asset: Vec<u8>, metadata: Vec<u8> },
    Io(std::io::Error),
}

fn path_ext(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

fn import_file(
    path: &Path,
    database: &AssetDatabase,
    registry: &AssetPipelineRegistry,
    events: &Events,
) {
    let loader = if let Some(loader) = path_ext(path).and_then(|ext| registry.meta_by_ext(ext)) {
        loader
    } else {
        return;
    };

    let inner = || {
        let metadata = loader
            .load_meta(path)
            .map_err(|_| ImportError::MetadataNotFound)?;

        let source = match database.source(path) {
            Some(source) => source,
            None => SourceInfo::new(metadata.id(), 0, 0),
        };

        let modified = database.config().modified(path);

        if modified != source.modified() || !database.config().block_exists(&metadata.id()) {
            let asset = std::fs::read(path).map_err(|e| ImportError::Io(e))?;
            let (_, metadata) = metadata.take();
            Err(ImportError::AssetModified { asset, metadata })
        } else {
            let asset = std::fs::read(path).map_err(|e| ImportError::Io(e))?;
            let checksum = SourceInfo::calculate_checksum(&asset, metadata.data());

            if checksum != source.checksum() {
                let (_, metadata) = metadata.take();
                Err(ImportError::AssetModified { asset, metadata })
            } else {
                Ok(())
            }
        }
    };

    if let Err(err) = inner() {
        match err {
            ImportError::MetadataNotFound => {
                let reason = ImportReason::manual(path);
                loader.import(&database, &events, reason);
            }
            ImportError::AssetModified { asset, metadata } => {
                let reason = ImportReason::asset_modified(path, asset, metadata);
                loader.import(&database, &events, reason);
            }
            _ => {}
        }
    }
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

            database.library.add_import(&path);

            let mut metadata = match database.load_metadata::<FolderSettings>(&path) {
                Some(metadata) => metadata,
                None => AssetMetadata::<FolderSettings>::default(),
            };

            let source = SourceInfo::new(metadata.id(), 0, 0);
            database.library.set_source(path.clone(), source);

            let mut children = HashSet::new();

            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                let path = &entry.path();
                children.insert(path.to_path_buf());

                if path.is_dir() {
                    if database.importing(path) {
                        events.add(ImportFolder::new(path));
                    } else {
                        paths.push(path.to_path_buf());
                    }
                } else {
                    import_file(path, &database, &registry, &events);
                }
            }

            database.block(&metadata.id()).unwrap_or_else(|| {
                let block = BlockInfo::new(path.clone(), AssetType::of::<Folder>());
                database.library.set_block(metadata.id(), block.clone());
                block
            });

            metadata.settings_mut().retain(|child| {
                let contains = children.contains(child);
                if !contains {
                    database.library.remove_source(child);
                }
                contains
            });

            database
                .save_metadata::<Folder, FolderSettings>(&path, &[], &metadata)
                .ok();

            events.add(FolderImported::new(path));
        }
    });
}

pub fn import_assets<P: AssetPipeline>(
    imports: &[<ImportAsset<P::Asset> as Event>::Output],
    events: &Events,
    database: &AssetDatabase,
    tasks: &TaskManager,
) {
    let imports = imports.to_vec();
    let events = events.clone();
    let database = database.clone();

    tasks.spawn(move || {
        for info in &imports {
            match import_asset::<P>(info, &database) {
                Ok(event) => {
                    // TODO: Reload dependent assets that have already been loaded
                    events.add(event);
                }
                Err(event) => events.add(event),
            }
        }
    });
}

fn import_asset<P: AssetPipeline>(
    import: &ImportInfo,
    database: &AssetDatabase,
) -> Result<AssetImported<P::Asset>, AssetFailed<P::Asset>> {
    let path = import.path();
    let reason = import.reason();

    let mut metadata = load_metadata::<P::Settings>(database, reason);

    let bytes = match reason {
        ImportReason::AssetModified { asset, .. } => Cow::Borrowed(asset),
        _ => Cow::Owned(std::fs::read(path).map_err(|e| e.into_event(path))?),
    };

    let (asset, dependencies) = {
        let mut ctx = LoadContext::new(path, &bytes, &mut metadata);
        let asset = P::Loader::load(&mut ctx);
        (asset, ctx.dependencies().to_vec())
    };

    let mut asset = asset.map_err(|e| e.into_event(path))?;

    P::Processor::process(&mut asset, &mut metadata);
    P::PostProcessor::post_process(&mut asset, &mut metadata);

    database
        .save_metadata::<P::Asset, P::Settings>(path, &bytes, &metadata)
        .map_err(|e| e.into_event(path))?;

    let block = AssetBlock::new(&asset, metadata.settings(), &dependencies);
    database
        .save_asset::<P::Asset, P::Settings>(path, &block, &metadata)
        .map_err(|e| e.into_event(path))?;

    let event = AssetImported::new(metadata.id(), path, asset);
    Ok(event)
}

fn load_metadata<S: Settings>(database: &AssetDatabase, reason: &ImportReason) -> AssetMetadata<S> {
    let metadata = match reason {
        ImportReason::AssetModified { metadata, .. } => {
            let contents = String::from_utf8(metadata.to_vec()).unwrap_or_default();
            toml::from_str::<AssetMetadata<S>>(&contents).ok()
        }
        ImportReason::Added { .. } => Some(AssetMetadata::<S>::default()),
        ImportReason::DependencyModified { asset, .. } => database
            .block(asset)
            .and_then(|block| database.load_metadata::<S>(block.filepath())),
        ImportReason::Manual { path } => database.load_metadata::<S>(path),
    };

    metadata.unwrap_or_default()
}
