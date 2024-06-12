use super::{
    events::{AssetImport, ProcessAsset, TaskCounterUpdated},
    library::AssetStatus,
};
use crate::{
    asset::{AssetId, AssetSettings, Assets},
    database::{
        events::FolderImported,
        library::{BlockInfo, SourceInfo},
        AssetDatabase,
    },
    loader::{AssetLoader, AssetProcesser, LoadContext},
    registry::AssetLoaderRegistry,
};
use shadow_ecs::ecs::{
    event::Events,
    system::{access::WorldAccess, observer::Observer, SystemArg},
    task::TaskManager,
    world::World,
};
use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

fn path_ext(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

pub fn on_import_folders(
    paths: &[PathBuf],
    events: &Events,
    database: &AssetDatabase,
    registry: &AssetLoaderRegistry,
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

            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                let path = &entry.path();
                if path.is_dir() {
                    paths.push(path.to_path_buf());
                } else if let Some(loader) =
                    path_ext(path).and_then(|ext| registry.meta_by_ext(ext))
                {
                    let metadata = match database.config().metadata(path).ok() {
                        Some(metadata) => metadata,
                        None => {
                            loader.import(&events, &path.into());
                            continue;
                        }
                    };

                    let source = match database.source(path) {
                        Some(source) => source,
                        None => {
                            loader.import(&events, &path.into());
                            continue;
                        }
                    };

                    database.block(&source.id()).unwrap_or_else(|| {
                        let block = BlockInfo::new(path.clone(), loader.ty());
                        database.set_block(source.id(), block.clone());
                        block
                    });

                    if metadata.id() != source.id() {
                        loader.import(&events, &path.into());
                        continue;
                    }

                    let modified = match path.metadata().ok().and_then(|meta| meta.modified().ok())
                    {
                        Some(modified) => SourceInfo::system_time_to_secs(modified),
                        None => SourceInfo::system_time_to_secs(SystemTime::now()),
                    };

                    if modified != source.modified() {
                        loader.import(&events, &path.into());
                        continue;
                    }

                    let asset = match database.config().asset(path) {
                        Ok(asset) => asset,
                        Err(_) => continue,
                    };

                    let checksum = SourceInfo::calculate_checksum(&asset, metadata.data());
                    if checksum != source.checksum() {
                        loader.import(&events, &path.into());
                    }
                }
            }
        }

        for _ in paths {
            events.add(FolderImported);
        }
    });
}

pub fn on_import_asset<L: AssetLoader>(
    imports: &[AssetImport],
    events: &Events,
    database: &AssetDatabase,
    registry: &AssetLoaderRegistry,
    assets: &mut Assets<L::Asset>,
    settings: &mut AssetSettings<L::Settings>,
) {
    let mut import_asset = |import: &AssetImport| -> std::io::Result<()> {
        let path = import.path();

        let metadata = database
            .config()
            .metadata(path)?
            .into::<L::Settings>()
            .ok_or(std::io::ErrorKind::InvalidData)?;

        let bytes = std::fs::read(path)?;
        let mut ctx = LoadContext::new(path, &bytes, &metadata);
        let asset = L::load(&mut ctx);

        for dependency in ctx.dependencies() {
            if let Some(info) = database.block(dependency) {
                if let Some(loader) = registry.meta(info.ty()) {
                    loader.import(&events, &info.filepath().into());
                }
            }
        }

        assets.insert(import.id(), asset);

        let (_, _settings) = metadata.take();
        settings.insert(import.id(), _settings);
        Ok(())
    };

    for import in imports {
        match import_asset(import) {
            Ok(_) => {
                if database.tracker().can_process(&import.id()) {
                    events.add(ProcessAsset::<L::Asset>::new(import.id()));
                }
            }
            Err(_) => {
                database.set_status(import.id(), AssetStatus::Failed);
                database.counter().decrement();
                events.add(TaskCounterUpdated);
            }
        }
    }
}

pub fn create_on_process<P: AssetProcesser>() -> Observer<ProcessAsset<<P as AssetProcesser>::Asset>>
{
    let observer = |ids: &[AssetId], world: &World| {
        let assets = world.resource_mut::<Assets<P::Asset>>();
        let settings = world.resource::<AssetSettings<P::Settings>>();
        let database = world.resource::<AssetDatabase>();
        let events = world.events();
        let args = P::Args::get(world);
        for id in ids {
            let asset = match assets.get_mut(id) {
                Some(asset) => asset,
                None => continue,
            };

            let settings = match settings.get(id) {
                Some(settings) => settings,
                None => continue,
            };

            match P::process(id, asset, settings, &args) {
                Ok(_) => {
                    if database.tracker().is_dependencies_done(id) {
                        database.set_status(*id, AssetStatus::Done);
                        database.counter().decrement();
                        events.add(TaskCounterUpdated);
                    }
                },
                Err(_) => {
                    database.set_status(*id, AssetStatus::Failed);
                    database.counter().decrement();
                    events.add(TaskCounterUpdated);
                }
            }
        }
    };

    let (reads, writes) = WorldAccess::parse(&P::Args::access());
    Observer::<ProcessAsset<P::Asset>>::new(observer, reads, writes)
}
