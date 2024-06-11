use crate::{
    asset::{Asset, AssetId, AssetPath},
    database::{
        library::{AssetStatus, SourceInfo},
        AssetDatabase,
    },
};
use shadow_ecs::ecs::event::Event;
use std::path::PathBuf;

pub struct AssetImport {
    path: PathBuf,
    id: AssetId,
}

impl AssetImport {
    pub fn new(path: PathBuf, id: AssetId) -> Self {
        AssetImport { path, id }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn id(&self) -> AssetId {
        self.id
    }
}

pub struct ImportAsset<A: Asset> {
    path: AssetPath,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> ImportAsset<A> {
    pub fn new(path: impl Into<AssetPath>) -> Self {
        ImportAsset {
            path: path.into(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    fn id_and_path(&self, database: &AssetDatabase) -> Option<(AssetId, PathBuf)> {
        match &self.path {
            AssetPath::Id(id) => {
                let block = database.block(id)?;
                Some((*id, block.filepath().to_path_buf()))
            }
            AssetPath::Path(path) => match database.source(path) {
                Some(source) => Some((source.id(), path.clone())),
                None => Some((AssetId::gen(), path.clone())),
            },
        }
    }
}

impl<A: Asset> Event for ImportAsset<A> {
    type Output = AssetImport;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();

        match database.status(&self.path) {
            AssetStatus::Importing | AssetStatus::Loading | AssetStatus::Processing => {
                database.state().enqueue_import::<A>(&self.path);
                None
            }
            AssetStatus::None | AssetStatus::Failed | AssetStatus::Done => {
                let (id, path) = self.id_and_path(database)?;
                database.add_source(path.clone(), SourceInfo::new(id, 0, 0));
                database
                    .state()
                    .set_asset_status(id, AssetStatus::Importing);

                Some(AssetImport::new(path, id))
            }
        }
    }
}

pub struct ImportFolder {
    path: PathBuf,
}

impl ImportFolder {
    pub fn new(path: PathBuf) -> Self {
        ImportFolder { path }
    }
}

impl Event for ImportFolder {
    type Output = PathBuf;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();

        Some(self.path.clone())
    }
}
