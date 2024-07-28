use crate::{
    events::{AssetUnload, ImportAssets, LoadAssets, LoadRequest, RemoveAssets, StartAssetEvent},
    importer::{ImportError, LoadError},
    Asset, AssetDatabase, AssetFileSystem, AssetPath,
};
use shadow_ecs::{
    event::{Event, EventStorage, Events},
    task::TaskPool,
};
use std::path::PathBuf;

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

pub fn on_import_asset(paths: &[PathBuf], db: &AssetDatabase) {
    let paths = paths.iter().map(|p| p.clone()).collect();

    db.events().push_back(ImportAssets::new(paths))
}

pub fn on_load_error(errors: &[LoadError], db: &AssetDatabase) {
    let mut events = EventStorage::new();
    for error in errors {
        let id = match &error.path {
            AssetPath::Id(id) => *id,
            AssetPath::Path(path) => {
                if let Some(id) = db.path_id(path) {
                    id
                } else {
                    continue;
                }
            }
        };

        let tracker = db.tracker();

        if let Some(state) = tracker.state(&id) {
            let importers = db.importers();
            let importer = importers.importer(state.ty()).unwrap();
            importer.asset_unload(id, &mut events);
        }
    }
}

pub fn on_import_error(errors: &[ImportError], db: &AssetDatabase) {
    let removed = errors.iter().map(|e| e.path.clone()).collect::<Vec<_>>();

    db.events().push_back(RemoveAssets::new(removed))
}

pub fn on_load_asset(requests: &[LoadRequest], db: &AssetDatabase) {
    let requests = requests.to_vec();

    db.events().push_back(LoadAssets::new(requests));
}

pub fn on_unload_asset<A: Asset>(unloads: &[AssetUnload<A>], db: &AssetDatabase) {
    let tracker = db.tracker();
    let ids = unloads.iter().map(|u| u.id());
    let dependents = tracker.dependents(ids).drain();
    let loads = dependents.iter().map(|id| LoadRequest::soft(*id)).collect();

    db.events().push_back(LoadAssets::new(loads));
}
