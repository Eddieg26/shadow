use std::path::PathBuf;

use crate::{
    events::{ImportAssets, LoadAssets, LoadRequest, RemoveAssets, StartAssetEvent},
    importer::ImportError,
    AssetDatabase, AssetFileSystem, AssetId,
};
use shadow_ecs::{
    event::{Event, Events},
    task::TaskPool,
};

pub fn on_start_asset_event(
    _: &[<StartAssetEvent as Event>::Output],
    db: &AssetDatabase,
    fs: &AssetFileSystem,
    events: &Events,
    tasks: &TaskPool,
) {
    let mut db_events = db.events();
    if !db_events.running() {
        db_events.set_running(true);
        let db = db.clone();
        let fs = fs.clone();
        let events = events.clone();
        tasks.spawn(move || {
            while let Some(event) = db.events().pop() {
                event.execute(&fs, &db, &events)
            }

            db.events().set_running(false);
        })
    }
}

pub fn on_import_error(errors: &[ImportError], db: &AssetDatabase) {
    let removed = errors.iter().map(|e| e.path.clone()).collect::<Vec<_>>();

    db.events().push_back(RemoveAssets::new(removed))
}

pub fn on_reload_assets(ids: &[AssetId], db: &AssetDatabase) {
    let ids = ids.iter().map(|id| LoadRequest::new(*id, false)).collect();

    db.events().push_back(LoadAssets::new(ids))
}

pub fn on_import_asset(paths: &[PathBuf], db: &AssetDatabase) {
    let paths = paths.iter().map(|p| p.clone()).collect();

    db.events().push_back(ImportAssets::new(paths))
}

pub fn on_load_asset(requests: &[LoadRequest], db: &AssetDatabase) {
    let requests = requests.to_vec();

    db.events().push_back(LoadAssets::new(requests));
}
