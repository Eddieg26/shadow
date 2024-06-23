use super::{
    library::{BlockInfo, ImportStatus, LoadStatus, SourceInfo},
    queue::AssetAction,
    AssetDatabase,
};
use crate::{
    asset::{Asset, AssetId, AssetPath},
    block::MetadataBlock,
    errors::AssetError,
    registry::AssetRegistry,
};
use shadow_ecs::ecs::{event::Event, world::World};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

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
    DependencyModified {
        asset: AssetId,
        dependency: AssetId,
    },
}

impl ImportReason {
    pub fn manual(path: PathBuf) -> ImportReason {
        ImportReason::Manual { path }
    }

    pub fn added(path: PathBuf, source: SourceInfo) -> ImportReason {
        ImportReason::Added { path, source }
    }

    pub fn modified(
        path: PathBuf,
        source: SourceInfo,
        previous: SourceInfo,
        asset: Vec<u8>,
        metadata: MetadataBlock,
    ) -> ImportReason {
        ImportReason::Modified {
            path,
            source,
            previous,
            asset,
            metadata,
        }
    }

    pub fn dependency_modified(asset: AssetId, dependency: AssetId) -> ImportReason {
        ImportReason::DependencyModified { asset, dependency }
    }

    pub fn asset_path(&self) -> AssetPath {
        match self {
            ImportReason::Manual { path } => AssetPath::Path(path.clone()),
            ImportReason::Added { path, .. } => AssetPath::Path(path.clone()),
            ImportReason::Modified { path, .. } => AssetPath::Path(path.clone()),
            ImportReason::DependencyModified { asset, .. } => AssetPath::Id(*asset),
        }
    }

    pub fn path<'a>(&'a self) -> Option<&'a Path> {
        match self {
            ImportReason::Manual { path } => Some(path.as_ref()),
            ImportReason::Added { path, .. } => Some(path.as_ref()),
            ImportReason::Modified { path, .. } => Some(path.as_ref()),
            ImportReason::DependencyModified { .. } => None,
        }
    }
}

#[derive(Clone)]
pub struct ImportInfo {
    pub id: AssetId,
    pub path: PathBuf,
    pub reason: ImportReason,
}

impl ImportInfo {
    pub fn new(id: AssetId, path: PathBuf, reason: ImportReason) -> Self {
        Self { id, path, reason }
    }

    pub fn id(&self) -> AssetId {
        self.id
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

    pub fn with_reason(reason: ImportReason) -> Self {
        ImportAsset {
            reason: Some(reason),
            _marker: std::marker::PhantomData,
        }
    }

    fn id_and_path<'a>(
        &'a self,
        reason: &'a ImportReason,
        database: &'a AssetDatabase,
    ) -> (AssetId, Option<Cow<'a, PathBuf>>) {
        match reason {
            ImportReason::Manual { path } => {
                let id = database
                    .source(path)
                    .map(|source| source.id())
                    .unwrap_or(AssetId::gen());
                (id, Some(Cow::Borrowed(path)))
            }
            ImportReason::Added { path, source } => {
                let id = source.id();
                (id, Some(Cow::Borrowed(path)))
            }
            ImportReason::Modified { path, metadata, .. } => {
                let id = metadata.id();
                (id, Some(Cow::Borrowed(path)))
            }
            ImportReason::DependencyModified { asset, .. } => {
                let id = *asset;
                let path = database
                    .block(asset)
                    .map(|block| block.filepath().to_path_buf());
                (id, path.map(Cow::Owned))
            }
        }
    }
}

impl<A: Asset> Event for ImportAsset<A> {
    type Output = ImportInfo;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let reason = self.reason.take()?;
        let database = world.resource::<AssetDatabase>();
        let (id, path) = self.id_and_path(&reason, database);
        let path = path?;

        if let Some(block) = database.block(&id) {
            let block = block.with_path(path.as_ref().clone());
            database.library.set_block(id, block);
        }

        match database.status::<LoadStatus>(&id) {
            LoadStatus::Loading => {
                let path = path.as_ref().to_path_buf();
                database.enqueue_action(path, AssetAction::Import { reason });
                return None;
            }
            _ => {}
        }

        match database.status::<ImportStatus>(path.as_ref()) {
            ImportStatus::Importing => {
                database.enqueue_action(path.as_ref().clone(), AssetAction::Import { reason });
                return None;
            }
            _ => {}
        }

        database
            .library
            .set_status(path.as_ref().clone(), ImportStatus::Importing);

        Some(ImportInfo::new(id, path.into_owned(), reason))
    }
}

pub struct ImportDependency {
    asset: AssetId,
    dependency: AssetId,
}

impl ImportDependency {
    pub fn new(asset: AssetId, dependency: AssetId) -> Self {
        ImportDependency { asset, dependency }
    }
}

impl Event for ImportDependency {
    type Output = AssetId;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        let registry = world.resource::<AssetRegistry>();
        let block = database.block(&self.asset)?;
        let loader = registry.meta(block.ty())?;
        let path = block.filepath().to_path_buf();
        let reason = ImportReason::Manual { path };
        loader.import_event(&mut world.events().storage(), reason);
        Some(self.dependency)
    }
}

pub struct ImportDependent {
    asset: AssetId,
    dependent: AssetId,
}

impl ImportDependent {
    pub fn new(asset: AssetId, dependent: AssetId) -> Self {
        ImportDependent { asset, dependent }
    }
}

impl Event for ImportDependent {
    type Output = AssetId;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        let registry = world.resource::<AssetRegistry>();
        let block = database.block(&self.asset)?;
        let loader = registry.meta(block.ty())?;
        let reason = ImportReason::DependencyModified {
            asset: self.dependent,
            dependency: self.asset,
        };
        loader.import_event(&mut world.events().storage(), reason);
        Some(self.dependent)
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
        match database.status::<ImportStatus>(&self.path) {
            ImportStatus::Importing => {
                database.enqueue_action(self.path.clone(), AssetAction::ImportFolder);
                None
            }
            _ => {
                database
                    .library
                    .set_status(self.path.clone(), ImportStatus::Importing);
                Some(self.path.clone())
            }
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
}

impl Event for FolderImported {
    type Output = PathBuf;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        database
            .library
            .set_status(self.path.clone(), ImportStatus::None);

        database.dequeue_action::<()>(&self.path);

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

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();

        let (id, path) = match &self.path {
            AssetPath::Id(id) => {
                let block = database.library.block(id)?;
                (*id, block.filepath().to_path_buf())
            }
            AssetPath::Path(path) => {
                let source = database.source(path)?;
                (source.id(), path.to_path_buf())
            }
        };

        match database.status::<LoadStatus>(&id) {
            LoadStatus::Loading => {
                database.enqueue_action(path, AssetAction::Load { id });
                return None;
            }
            _ => {}
        }

        match database.status::<ImportStatus>(&path) {
            ImportStatus::Importing => {
                database.enqueue_action(path.clone(), AssetAction::Load { id });
                return None;
            }
            _ => {}
        }

        database.library.set_status(id, LoadStatus::Loading);

        Some(id)
    }
}

pub struct ProcessAsset<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> ProcessAsset<A> {
    pub fn new(id: AssetId) -> Self {
        ProcessAsset {
            id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Event for ProcessAsset<A> {
    type Output = AssetId;

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        match database.block(&self.id) {
            Some(block) => {
                let path = block.filepath().to_path_buf();
                database
                    .library
                    .set_status(path, ImportStatus::PostProccessing);
                Some(self.id)
            }
            None => None,
        }
    }
}

pub struct PostProcessAsset<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> PostProcessAsset<A> {
    pub fn new(id: AssetId) -> Self {
        PostProcessAsset {
            id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Event for PostProcessAsset<A> {
    type Output = AssetId;

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        match database.block(&self.id) {
            Some(block) => {
                let path = block.filepath().to_path_buf();
                database
                    .library
                    .set_status(path, ImportStatus::PostProccessing);
                Some(self.id)
            }
            None => None,
        }
    }
}

pub struct AssetSaved<A: Asset> {
    id: AssetId,
    path: PathBuf,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetSaved<A> {
    pub fn new(id: AssetId, path: impl AsRef<Path>) -> Self {
        AssetSaved {
            id,
            path: path.as_ref().to_path_buf(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }
}

impl<A: Asset> Event for AssetSaved<A> {
    type Output = AssetId;

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        database
            .library
            .set_status(self.path.clone(), ImportStatus::Done);

        database.dequeue_action::<A>(&self.path);

        Some(self.id)
    }
}

pub struct AssetRemoved {
    path: PathBuf,
}

impl AssetRemoved {
    pub fn new(path: impl AsRef<Path>) -> Self {
        AssetRemoved {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl Event for AssetRemoved {
    type Output = (PathBuf, SourceInfo, Option<BlockInfo>);

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        let source = database.library.remove_source(&self.path)?;

        let block = if let Some(block) = database.library.block(&source.id()) {
            if block.filepath() == &self.path {
                database.library.remove_block(&source.id())
            } else {
                None
            }
        } else {
            None
        };

        Some((self.path.clone(), source, block))
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
    error: Option<AssetError>,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetFailed<A> {
    pub(crate) fn new(error: AssetError) -> Self {
        AssetFailed {
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
        match &error {
            AssetError::Loading { id, .. } => {
                database.library.set_status(*id, LoadStatus::Failed);
                Some(AssetFailure::new(AssetPath::Id(*id), None, error))
            }
            AssetError::Importing { path, .. } => {
                let path = path.to_path_buf();
                database
                    .library
                    .set_status(path.clone(), ImportStatus::Failed);
                Some(AssetFailure::new(path, None, error))
            }
            AssetError::Processing { id, .. } | AssetError::PostProcessing { id, .. } => {
                if let Some(block) = database.block(id) {
                    let path = block.filepath().to_path_buf();
                    database.library.set_status(path, ImportStatus::Failed);
                    Some(AssetFailure::new(AssetPath::Id(*id), None, error))
                } else {
                    Some(AssetFailure::new(AssetPath::Id(*id), None, error))
                }
            }
            AssetError::Saving { path, .. } => {
                let path = path.to_path_buf();
                database
                    .library
                    .set_status(path.clone(), ImportStatus::Failed);
                Some(AssetFailure::new(path, None, error))
            }
        }
    }
}

impl<A: Asset> Into<AssetFailed<A>> for AssetError {
    fn into(self) -> AssetFailed<A> {
        AssetFailed::new(self)
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
