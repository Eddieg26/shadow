use super::events::{AssetLoaded, AssetMetas, ImportAsset, LoadAsset, SettingsLoaded};
use crate::{
    asset::{AssetId, AssetInfo, AssetMetadata},
    bytes::AsBytes,
    config::AssetConfig,
    loader::{AssetLoader, LoadContext},
    pack::AssetPack,
    tracker::AssetTrackers,
};
use shadow_ecs::ecs::{
    event::Events,
    system::observer::{IntoObserver, Observer},
    task::TaskManager,
};
use std::path::PathBuf;

pub fn on_import_folder(
    paths: &[PathBuf],
    config: &AssetConfig,
    metas: &AssetMetas,
    events: &Events,
) {
    for path in paths {
        let entries = match std::fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let path = entry.path();
            if path.is_dir() {
                on_import_folder(&[path], config, metas, events);
            } else if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                if let Some(meta) = metas.get_by_ext(ext) {
                    meta.import(events, path.clone());
                }
            }
        }
    }
}

pub fn on_import_assets<L: AssetLoader>() -> Observer<ImportAsset<L::Asset>> {
    let observer = |paths: &[PathBuf], config: &AssetConfig, tasks: &TaskManager| {
        let paths = paths
            .iter()
            .map(|path| path.to_path_buf())
            .collect::<Vec<_>>();
        let config = config.clone();
        let tasks = tasks.clone();

        tasks.spawn(move || {
            for path in &paths {
                let asset_bytes = match std::fs::read(&path) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };

                let meta_path = config.meta_path(path);
                let metadata = {
                    if let Ok(bytes) = std::fs::read(&meta_path) {
                        match AssetMetadata::<L::Settings>::from_bytes(&bytes) {
                            Some(data) => data,
                            None => {
                                let data = AssetMetadata::<L::Settings>::default();
                                let _ = config.save_metadata(&path, &data);
                                data
                            }
                        }
                    } else {
                        let data = AssetMetadata::<L::Settings>::default();
                        let _ = config.save_metadata(&path, &data);
                        data
                    }
                };

                let checksum =
                    AssetInfo::calculate_checksum(&asset_bytes, &metadata.settings().as_bytes());

                let info_path = config.asset_info_path(path);
                let info = {
                    if let Ok(bytes) = std::fs::read(&info_path) {
                        match AssetInfo::from_bytes(&bytes) {
                            Some(data) => {
                                if data.id() != metadata.id() {
                                    let data = AssetInfo::new(metadata.id(), 0);
                                    let _ = config.save_asset_info(&path, &data);
                                    data
                                } else {
                                    data
                                }
                            }
                            None => {
                                let data = AssetInfo::new(metadata.id(), 0);
                                let _ = config.save_asset_info(&path, &data);
                                data
                            }
                        }
                    } else {
                        let data = AssetInfo::new(metadata.id(), 0);
                        let _ = config.save_asset_info(&path, &data);
                        data
                    }
                };

                if info.checksum() != checksum {
                    let data = AssetInfo::new(metadata.id(), checksum);
                    let _ = config.save_asset_info(&path, &data);

                    let mut ctx = LoadContext::new(path, &metadata);
                    if let Ok(asset) = L::load(&mut ctx) {
                        let depenencies = ctx.dependencies().iter().map(|dep| *dep).collect();
                        let pack = AssetPack::save(&asset, metadata.settings(), depenencies);
                        let cached_path = config.cached_asset_path(&metadata.id());
                        let _ = std::fs::write(&cached_path, &pack);
                    }
                }
            }
        });
    };

    observer.into_observer()
}

pub fn on_load_assets<L: AssetLoader>() -> Observer<LoadAsset<L::Asset>> {
    let observer = |ids: &[AssetId],
                    config: &AssetConfig,
                    events: &Events,
                    metas: &AssetMetas,
                    trackers: &AssetTrackers,
                    tasks: &TaskManager| {
        let ids = ids.iter().map(|id| *id).collect::<Vec<_>>();
        let config = config.clone();
        let events = events.clone();
        let metas = metas.clone();
        let trackers = trackers.clone();
        let tasks = tasks.clone();
        tasks.spawn(move || {
            for id in &ids {
                let path = config.cached_asset_path(id);
                let bytes = match std::fs::read(&path) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        trackers.fail(id);
                        continue;
                    }
                };

                let (asset, settings, dependencies) =
                    match AssetPack::<L::Asset, L::Settings>::parse(&bytes) {
                        Some(pack) => pack.take(),
                        None => {
                            trackers.fail(id);
                            continue;
                        }
                    };

                events.add(AssetLoaded::new(*id, asset));
                events.add(SettingsLoaded::new(*id, settings));

                let result = match trackers.load(*id, &dependencies) {
                    Some(result) => result,
                    None => continue,
                };

                for dependency in result.unloaded() {
                    if let Some(meta) = metas.get_dyn(dependency.ty()) {
                        meta.load(&events, dependency.id());
                    }
                }

                for dep in result.finished() {
                    if let Some(meta) = metas.get_dyn(dep.ty()) {
                        meta.process(&events, dep.id());
                    }
                }
            }
        });
    };

    observer.into_observer()
}
