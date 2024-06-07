use super::import::ImportAsset;
use crate::{
    database::AssetDatabase,
    loader::{AssetLoader, LoadContext},
};
use shadow_ecs::ecs::{event::Events, system::observer::Observer};
use std::path::PathBuf;

pub fn on_import_assets<L: AssetLoader>() -> Observer<ImportAsset<L::Asset>> {
    let observer = |paths: &[PathBuf], db: &AssetDatabase, events: &Events| {
        for path in paths {
            let metadata = db
                .config()
                .metadata::<L::Settings>(path)
                .unwrap_or_default();

            let _ = db.config().save_metadata(path, &metadata);

            let mut ctx = LoadContext::<L::Settings>::new(path, &metadata);
            match L::load(&mut ctx) {
                Ok(asset) => todo!(),
                Err(error) => todo!(),
            }
        }
    };

    todo!()
}

pub fn on_import_folders(paths: &[PathBuf], events: &Events, db: &AssetDatabase) {
    let mut paths = paths.iter().cloned().collect::<Vec<_>>();

    while let Some(path) = paths.pop() {
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
                paths.push(path);
            } else if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                if ext != "meta" {
                    let source = match db.config().source(&path) {
                        Ok(source) => source,
                        Err(_) => continue,
                    };

                    // TODO Check if asset has to be imported
                }
                // if let Some(meta) = db.registery().ext_meta(ext) {
                //     meta.import(&events, path.clone());
                // }
            }
        }
    }
}
