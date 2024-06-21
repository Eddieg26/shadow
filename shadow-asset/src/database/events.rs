use super::{
    library::{AssetStatus, SourceInfo},
    queue::AssetAction,
    AssetDatabase,
};
use crate::{
    asset::{Asset, AssetId, AssetPath, Assets},
    block::MetadataBlock,
    errors::AssetError,
};
use shadow_ecs::ecs::{event::Event, world::World};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum ImportReason {
    Manual {
        path: PathBuf,
    },
    Added {
        path: PathBuf,
        source: SourceInfo,
    },
    Modified {
        path: PathBuf,
        source: SourceInfo,
        previous: SourceInfo,
        asset: Vec<u8>,
        metadata: MetadataBlock,
    },
    Removed {
        path: PathBuf,
        id: AssetId,
    },
}

impl ImportReason {
    pub fn path(&self) -> PathBuf {
        match self {
            ImportReason::Manual { path, .. } => path.clone(),
            ImportReason::Added { path, .. } => path.clone(),
            ImportReason::Modified { path, .. } => path.clone(),
            ImportReason::Removed { path, .. } => path.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ImportInfo {
    pub path: PathBuf,
    pub reason: ImportReason,
}

impl ImportInfo {
    pub fn new(path: PathBuf, reason: ImportReason) -> Self {
        Self { path, reason }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn reason(&self) -> &ImportReason {
        &self.reason
    }
}

pub struct ImportAsset<A: Asset> {
    reason: Option<ImportReason>,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> ImportAsset<A> {
    pub fn new(path: impl AsRef<Path>) -> Self {
        ImportAsset {
            reason: Some(ImportReason::Manual {
                path: path.as_ref().to_path_buf(),
            }),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn with_reason(mut self, reason: ImportReason) -> Self {
        self.reason = Some(reason);
        self
    }
}

impl<A: Asset> Event for ImportAsset<A> {
    type Output = ImportInfo;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let reason = self.reason.take()?;
        let database = world.resource::<AssetDatabase>();
        let path = reason.path();

        match &reason {
            ImportReason::Modified { path, metadata, .. } => {
                if let Some(block) = database.block(&metadata.id()) {
                    let block = block.with_path(path.clone());
                    database.library.set_block(metadata.id(), block);
                }
            }
            _ => {}
        }

        match database.status(&path) {
            AssetStatus::Importing | AssetStatus::Loading => {
                database.enqueue_action(path, AssetAction::Import { reason });
                None
            }
            AssetStatus::None | AssetStatus::Failed | AssetStatus::Done => {
                database.library.add_import(&path);
                Some(ImportInfo::new(path, reason))
            }
        }
    }
}

pub struct ImportFolder {
    path: PathBuf,
}

impl ImportFolder {
    pub fn new(path: impl AsRef<Path>) -> Self {
        ImportFolder {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl Event for ImportFolder {
    type Output = PathBuf;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        match database.status(&self.path) {
            AssetStatus::Importing => {
                let action = AssetAction::Import {
                    reason: ImportReason::Manual {
                        path: self.path.clone(),
                    },
                };
                database.enqueue_action(self.path.clone(), action);
                None
            }
            _ => Some(database.config().assets().join(&self.path)),
        }
    }
}

pub struct FolderImported {
    path: PathBuf,
}

impl FolderImported {
    pub fn new(path: impl AsRef<Path>) -> Self {
        FolderImported {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn get_import(&self, database: &AssetDatabase) -> Option<ImportFolder> {
        while let Some(action) = database.dequeue_action(&self.path) {
            match action {
                AssetAction::Import { .. } => {
                    return Some(ImportFolder::new(self.path.clone()));
                }
                _ => {}
            }
        }

        None
    }
}

impl Event for FolderImported {
    type Output = PathBuf;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        database.library.remove_import(&self.path);

        if let Some(action) = database.dequeue_action(&self.path) {
            match action {
                AssetAction::Import { .. } => {
                    let event = ImportFolder::new(self.path.clone());
                    world.events().add(event);
                }
                _ => {
                    if let Some(event) = self.get_import(database) {
                        world.events().add(event);
                    }
                }
            }
        }

        Some(self.path.clone())
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
            AssetStatus::Importing | AssetStatus::Loading => {
                let (id, path) = match &self.path {
                    AssetPath::Id(id) => {
                        let path = database.block(id)?.filepath().to_path_buf();
                        (*id, path)
                    }
                    AssetPath::Path(path) => {
                        let info = database.source(path)?;
                        (info.id(), path.clone())
                    }
                };
                database.enqueue_action(path, AssetAction::Load { id });
                None
            }
        }?;

        database.library.set_status(id, AssetStatus::Loading);
        Some(id)
    }
}

#[derive(Clone)]
pub struct AssetImported<A: Asset> {
    id: AssetId,
    path: PathBuf,
    asset: Option<A>,
}

impl<A: Asset> AssetImported<A> {
    pub(crate) fn new(id: AssetId, path: impl AsRef<Path>, asset: A) -> Self {
        AssetImported {
            id,
            path: path.as_ref().to_path_buf(),
            asset: Some(asset),
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn asset(&self) -> Option<&A> {
        self.asset.as_ref()
    }
}

impl<A: Asset> Event for AssetImported<A> {
    type Output = Self;

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        database.library.set_status(self.id, AssetStatus::None);

        if let Some(action) = database.dequeue_action(&self.path) {
            match action {
                AssetAction::Import { reason } => {
                    let event = ImportAsset::<A>::new(self.path.clone()).with_reason(reason);
                    world.events().add(event);
                }
                AssetAction::Load { id } => {
                    let event = LoadAsset::<A>::new(id);
                    world.events().add(event);
                }
            }
        }

        let asset = self.asset.take()?;
        Some(AssetImported::new(self.id, self.path.clone(), asset))
    }
}

pub struct AssetFailure<A: Asset> {
    path: AssetPath,
    asset: Option<A>,
    error: AssetError,
}

impl<A: Asset> AssetFailure<A> {
    pub(crate) fn new(path: impl Into<AssetPath>, asset: Option<A>, error: AssetError) -> Self {
        AssetFailure {
            path: path.into(),
            asset,
            error,
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn asset(&self) -> Option<&A> {
        self.asset.as_ref()
    }

    pub fn error(&self) -> &AssetError {
        &self.error
    }
}

pub struct AssetFailed<A: Asset> {
    path: AssetPath,
    error: Option<AssetError>,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetFailed<A> {
    pub(crate) fn new(path: impl Into<AssetPath>, error: AssetError) -> Self {
        AssetFailed {
            path: path.into(),
            error: Some(error),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn error(&self) -> Option<&AssetError> {
        self.error.as_ref()
    }
}

impl<A: Asset> Event for AssetFailed<A> {
    type Output = AssetFailure<A>;

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let error = self.error.take()?;
        let database = world.resource::<AssetDatabase>();
        match &self.path {
            AssetPath::Id(id) => {
                database.library.set_status(*id, AssetStatus::Failed);
                let asset = world.resource_mut::<Assets<A>>().remove(id);
                Some(AssetFailure::new(id, asset, error))
            }
            AssetPath::Path(path) => {
                if let Some(info) = database.source(&path) {
                    database.library.set_status(info.id(), AssetStatus::Failed);
                    let asset = world.resource_mut::<Assets<A>>().remove(&info.id());
                    Some(AssetFailure::new(info.id(), asset, error))
                } else {
                    Some(AssetFailure::new(path, None, error))
                }
            }
        }
    }
}

pub trait AssetErrorExt {
    fn into_event<A: Asset>(self, path: impl Into<AssetPath>) -> AssetFailed<A>;
}

impl AssetErrorExt for AssetError {
    fn into_event<A: Asset>(self, path: impl Into<AssetPath>) -> AssetFailed<A> {
        match self {
            AssetError::AssetNotFound(id) => AssetFailed::new(id, AssetError::AssetNotFound(id)),
            AssetError::InvalidPath(path) => {
                AssetFailed::new(path.clone(), AssetError::InvalidPath(path))
            }
            AssetError::InvalidMetadata => AssetFailed::new(path, AssetError::InvalidMetadata),
            AssetError::InvalidData => AssetFailed::new(path, AssetError::InvalidData),
            AssetError::InvalidExtension(path) => {
                AssetFailed::new(path.clone(), AssetError::InvalidExtension(path))
            }
            AssetError::Io(e) => AssetFailed::new(path, AssetError::Io(e)),
        }
    }
}

impl AssetErrorExt for std::io::Error {
    fn into_event<A: Asset>(self, path: impl Into<AssetPath>) -> AssetFailed<A> {
        AssetFailed::new(path, AssetError::Io(self))
    }
}

pub struct LoadLibrary;

impl Event for LoadLibrary {
    type Output = ();

    fn invoke(&mut self, world: &mut World) -> Option<()> {
        let db = world.resource::<AssetDatabase>().clone();
        world.tasks().spawn(move || {
            let _ = db.library.load();
        });

        Some(())
    }
}
pub struct SaveLibrary;

impl Event for SaveLibrary {
    type Output = ();

    fn invoke(&mut self, world: &mut World) -> Option<()> {
        let db = world.resource::<AssetDatabase>().clone();
        world.tasks().spawn(move || {
            let _ = db.library.save();
        });
        Some(())
    }
}
