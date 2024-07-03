use super::{
    pipeline::AssetError,
    tracker::{ImportStatus, LoadStatus},
    AssetDatabase,
};
use crate::asset::Asset;
use shadow_ecs::ecs::event::Event;
use std::path::{Path, PathBuf};

pub struct ImportAsset {
    path: PathBuf,
}

impl ImportAsset {
    pub(super) fn new(path: impl AsRef<Path>) -> Self {
        ImportAsset {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl Event for ImportAsset {
    type Output = PathBuf;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();

        match database.import_status(&self.path) {
            ImportStatus::Failed | ImportStatus::None => {
                database.set_import_status(self.path.clone(), ImportStatus::Importing);
            }
            _ => return None,
        }

        Some(self.path.clone())
    }
}

pub struct ImportFolder {
    path: PathBuf,
}

impl ImportFolder {
    pub(super) fn new(path: impl Into<PathBuf>) -> Self {
        ImportFolder { path: path.into() }
    }
}

impl Event for ImportFolder {
    type Output = PathBuf;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();

        let sources = database.library.sources();
        if let Some(source) = sources.get(&self.path) {
            match database.load_status(&source.id()) {
                LoadStatus::Loading => return None,
                _ => {}
            }
        }

        match database.import_status(&self.path) {
            ImportStatus::Failed | ImportStatus::None => {
                database.set_import_status(self.path.clone().into(), ImportStatus::Importing);
            }
            _ => return None,
        }

        Some(database.config().assets().join(&self.path))
    }
}

pub struct RemoveAsset {
    path: PathBuf,
}

impl RemoveAsset {
    pub(super) fn new(path: impl AsRef<Path>) -> Self {
        RemoveAsset {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl Event for RemoveAsset {
    type Output = PathBuf;

    fn invoke(&mut self, _: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        Some(self.path.clone())
    }
}

pub struct ImportFailed<A: Asset> {
    path: PathBuf,
    error: Option<AssetError>,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> ImportFailed<A> {
    pub(super) fn new(path: PathBuf, error: AssetError) -> Self {
        ImportFailed {
            path,
            error: Some(error),
            _marker: Default::default(),
        }
    }
}

impl<A: Asset> Event for ImportFailed<A> {
    type Output = (PathBuf, AssetError);

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let error = self.error.take()?;
        let database = world.resource::<AssetDatabase>();
        database.set_import_status(self.path.clone(), ImportStatus::Failed);

        Some((self.path.clone(), error))
    }
}
