use crate::{
    asset::{Asset, AssetPath},
    database::{AssetDatabase, AssetStatus},
};
use shadow_ecs::ecs::event::Event;
use std::{marker::PhantomData, path::PathBuf};

pub struct ImportAsset<A: Asset> {
    path: AssetPath,
    _marker: PhantomData<A>,
}

impl<A: Asset> ImportAsset<A> {
    pub fn new(path: impl Into<AssetPath>) -> Self {
        Self {
            path: path.into(),
            _marker: PhantomData,
        }
    }
}

impl<A: Asset> Event for ImportAsset<A> {
    type Output = PathBuf;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        match &self.path {
            AssetPath::Id(id) => {
                let database = world.resource::<AssetDatabase>();
                if !database.status(id).unloaded() {
                    return None;
                } else {
                    match database.config().import(id).ok() {
                        Some(info) => {
                            database.set_status(id, AssetStatus::Importing);
                            return Some(info.path().to_path_buf());
                        }
                        None => {
                            database.set_status(id, AssetStatus::Failed);
                            return None;
                        }
                    }
                }
            }
            AssetPath::Path(path) => {
                let database = world.resource::<AssetDatabase>();
                let path = database.config().assets().join(path);
                if !database.is_importing_path(&path) {
                    let meta = database.registery().meta::<A>();
                    meta.load_id(&path.with_extension(".meta")).and_then(|id| {
                        database.status(&id).unloaded().then(|| {
                            database.set_status(&id, AssetStatus::Importing);
                            database.add_import_path(path.clone());
                            path.clone()
                        })
                    })
                } else {
                    return None;
                }
            }
        }
    }
}
