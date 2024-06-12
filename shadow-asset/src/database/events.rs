use crate::asset::{Asset, AssetId, AssetPath, AssetType};
use shadow_ecs::ecs::{
    event::{Event, Events},
    world::World,
};
use std::path::PathBuf;

use super::{
    library::{AssetStatus, BlockInfo, SourceInfo},
    task::{DatabaseState, DatabaseTask},
    AssetDatabase,
};

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
            AssetStatus::Importing | AssetStatus::Loading | AssetStatus::Processing => None,
            AssetStatus::None | AssetStatus::Failed | AssetStatus::Done => {
                let (id, path) = self.id_and_path(database)?;
                database.set_source(path.clone(), SourceInfo::new(id, 0, 0));
                database.set_status(id, AssetStatus::Importing);

                if let Some(block) = database.block(&id) {
                    if block.filepath() != path {
                        let block = BlockInfo::new(path.clone(), AssetType::of::<A>());
                        database.set_block(id, block.clone());
                    }
                }

                Some(AssetImport::new(path, id))
            }
        }
    }
}

impl<A: Asset> DatabaseTask for ImportAsset<A> {
    fn run(&self, events: &Events) {
        events.add(ImportAsset::<A>::new(self.path().clone()));
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
        let db = world.resource::<AssetDatabase>();
        db.counter().increment();

        Some(self.path.clone())
    }
}

pub struct FolderImported;
impl Event for FolderImported {
    type Output = ();

    fn invoke(&mut self, world: &mut World) -> Option<()> {
        let db = world.resource::<AssetDatabase>();
        db.counter().decrement();

        Some(())
    }
}

impl DatabaseTask for ImportFolder {
    fn run(&self, events: &Events) {
        events.add(ImportFolder::new(self.path.clone()));
    }
}

#[derive(Clone)]
pub struct LoadAsset<A: Asset> {
    path: AssetPath,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> LoadAsset<A> {
    pub(crate) fn new(path: impl Into<AssetPath>) -> Self {
        LoadAsset {
            path: path.into(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }
}

impl<A: Asset> Event for LoadAsset<A> {
    type Output = AssetId;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();

        let id = match database.status(&self.path) {
            AssetStatus::None | AssetStatus::Failed | AssetStatus::Done => match &self.path {
                AssetPath::Id(id) => Some(*id),
                AssetPath::Path(path) => database.source(path).map(|source| source.id()),
            },
            AssetStatus::Importing | AssetStatus::Processing | AssetStatus::Loading => {
                return None;
            }
        }?;

        database.set_status(id, AssetStatus::Loading);
        Some(id)
    }
}

impl<A: Asset> DatabaseTask for LoadAsset<A> {
    fn run(&self, events: &Events) {
        events.add(LoadAsset::<A>::new(self.path().clone()));
    }
}

pub struct LoadLibrary;

impl Event for LoadLibrary {
    type Output = ();

    fn invoke(&mut self, world: &mut World) -> Option<()> {
        let db = world.resource::<AssetDatabase>().clone();
        world.tasks().spawn(move || {
            let _ = db.library.load();
            db.set_state(DatabaseState::Ready);
        });

        Some(())
    }
}

impl DatabaseTask for LoadLibrary {
    fn run(&self, events: &shadow_ecs::ecs::event::Events) {
        events.add(LoadLibrary)
    }
}

pub struct SaveLibrary;

impl Event for SaveLibrary {
    type Output = ();

    fn invoke(&mut self, world: &mut World) -> Option<()> {
        let db = world.resource::<AssetDatabase>().clone();
        world.tasks().spawn(move || {
            let _ = db.library.save();
            db.set_state(DatabaseState::Ready);
        });
        Some(())
    }
}

impl DatabaseTask for SaveLibrary {
    fn run(&self, events: &shadow_ecs::ecs::event::Events) {
        events.add(SaveLibrary)
    }
}

pub struct TaskCounterUpdated;
impl Event for TaskCounterUpdated {
    type Output = ();

    fn invoke(&mut self, world: &mut World) -> Option<()> {
        let db = world.resource::<AssetDatabase>();
        db.update();
        Some(())
    }
}
