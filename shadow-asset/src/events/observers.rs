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
