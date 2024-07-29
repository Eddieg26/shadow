use super::events::AssetLoaded;
use crate::{
    events::{
        AssetEventExt, AssetUnload, ImportAssets, LoadAssets, LoadRequest, RemoveAssets,
        StartAssetEvent,
    },
    importer::{ImportError, LoadError},
    Asset, AssetDatabase, AssetId, AssetPath,
};
use shadow_ecs::{
    event::{Event, EventStorage, Events},
    task::TaskPool,
};
use std::path::PathBuf;

pub fn on_start_asset_event(
    _: &[<StartAssetEvent as Event>::Output],
    db: &AssetDatabase,
    events: &Events,
    tasks: &TaskPool,
) {
    let mut db_events = db.events();
    if !db_events.running() {
        db_events.set_running(true);
        let db = db.clone();
        let events = events.clone();
        tasks.spawn(move || {
            while let Some(event) = db.events().pop() {
                event.execute(&db, &events)
            }

            db.events().set_running(false);
        })
    }
}

pub fn on_import_asset(paths: &[PathBuf], events: &Events) {
    let paths = paths.iter().map(|p| p.clone()).collect();

    events.add_asset_event(ImportAssets::new(paths));
}

pub fn on_load_error(errors: &[LoadError], db: &AssetDatabase, main_events: &Events) {
    let mut events = EventStorage::new();
    for error in errors {
        let id = match &error.path {
            AssetPath::Id(id) => *id,
            AssetPath::Path(path) => match db.path_id(path) {
                Some(id) => id,
                None => continue,
            },
        };

        let states = db.states();

        if let Some(state) = states.get(&id) {
            let importers = db.importers();
            match importers.get(state.ty()) {
                Some(importer) => importer.asset_unload(id, &mut events),
                None => continue,
            };
        }
    }

    main_events.extend(events.into());
}

pub fn on_import_error(errors: &[ImportError], events: &Events) {
    let removed = errors.iter().map(|e| e.path.clone()).collect::<Vec<_>>();

    events.add_asset_event(RemoveAssets::new(removed));
}

pub fn on_load_asset(ids: &[AssetId], events: &Events) {
    let requests = ids.iter().map(|id| LoadRequest::soft(*id)).collect();

    events.add_asset_event(LoadAssets::new(requests));
}

pub fn on_unload_asset<A: Asset>(unloads: &[AssetUnload<A>], db: &AssetDatabase, events: &Events) {
    let states = db.states();
    let ids = unloads.iter().map(|u| u.id());
    let dependents = states.dependents(ids).drain();
    let loads = dependents.iter().map(|id| LoadRequest::soft(*id)).collect();

    events.add_asset_event(LoadAssets::new(loads))
}

pub fn on_asset_loaded<A: Asset>(
    ids: &[<AssetLoaded<A> as Event>::Output],
    db: &AssetDatabase,
    events: &Events,
) {
    let dependents = db.states().dependents(ids.iter()).drain();
    let loads = dependents.iter().map(|id| LoadRequest::soft(*id)).collect();

    events.add_asset_event(LoadAssets::new(loads));
}

pub fn on_asset_removed(ids: &[AssetId], db: AssetDatabase) {
    let states = db.states();
    let mut unloads = EventStorage::new();
    for id in ids {
        if let Some(state) = states.get(id) {
            let importers = db.importers();
            let importer = importers.get(state.ty()).unwrap();
            importer.asset_unload(*id, &mut unloads);
        }
    }
}
