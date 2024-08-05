use super::{
    events::{AssetUnloaded, ImportAssets, LoadAsset, LoadAssets, RemoveAssets, UnloadAsset},
    AssetDatabase,
};
use crate::{
    asset::{Asset, AssetId},
    loader::{AssetError, AssetErrorKind},
};
use shadow_ecs::{core::DenseSet, world::event::Events};
use std::path::PathBuf;

pub fn on_import_asset(paths: &[PathBuf], database: &AssetDatabase) {
    database.events().push(ImportAssets::new(paths.to_vec()));
}

pub fn on_remove_asset(paths: &[PathBuf], database: &AssetDatabase) {
    database.events().push(RemoveAssets::new(paths.to_vec()));
}

pub fn on_load_asset(loads: &[LoadAsset], database: &AssetDatabase) {
    database.events().push(LoadAssets::new(loads.to_vec()))
}

pub fn on_asset_error(errors: &[AssetError], database: &AssetDatabase, events: &Events) {
    let mut remove = Vec::new();
    let mut unloads = Vec::new();

    for error in errors {
        match error.kind() {
            AssetErrorKind::Import(path) => remove.push(path.clone()),
            AssetErrorKind::Load(path) => unloads.push(UnloadAsset::new(path.clone())),
        }
    }

    database.events().push(RemoveAssets::new(remove));
    events.extend(unloads);
}

pub fn on_asset_unloaded<A: Asset>(unloaded: &[AssetUnloaded<A>], database: &AssetDatabase) {
    let states = database.states();
    let mut reloads = DenseSet::new();

    for unloaded in unloaded {
        let dependents = states.dependents(&unloaded.id());
        reloads.extend(dependents);
    }

    database.events().push(LoadAssets::soft(reloads));
}

pub fn on_asset_loaded<A: Asset>(loaded: &[AssetId], database: &AssetDatabase) {
    let states = database.states();
    let mut reloads = DenseSet::new();

    for id in loaded {
        let dependents = states.dependents(id);
        reloads.extend(dependents);
    }

    database.events().push(LoadAssets::soft(reloads));
}
