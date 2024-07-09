use crate::{
    asset::{Asset, AssetId, AssetMetadata, AssetPath, Settings},
    bytes::ToBytes,
    database::AssetDatabase,
};
use shadow_ecs::ecs::{event::Event, world::World};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub struct ImportContext<'a, S: Settings> {
    path: &'a Path,
    asset: &'a [u8],
    metadata: &'a AssetMetadata<S>,
    dependencies: Vec<AssetId>,
}

impl<'a, S: Settings> ImportContext<'a, S> {
    pub fn new(path: &'a Path, asset: &'a [u8], metadata: &'a AssetMetadata<S>) -> Self {
        ImportContext {
            path,
            asset,
            metadata,
            dependencies: vec![],
        }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn asset(&self) -> &[u8] {
        self.asset
    }

    pub fn metadata(&self) -> &AssetMetadata<S> {
        self.metadata
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.push(id);
    }

    pub(crate) fn finish(self) -> HashSet<AssetId> {
        self.dependencies.into_iter().collect()
    }
}

pub trait AssetLoader: 'static {
    type Asset: Asset;

    fn save(asset: &Self::Asset) -> Vec<u8>;
    fn load(data: &[u8]) -> Result<Self::Asset, String>;
}

impl<T: Asset + ToBytes> AssetLoader for T {
    type Asset = T;

    fn save(asset: &Self::Asset) -> Vec<u8> {
        asset.to_bytes()
    }

    fn load(data: &[u8]) -> Result<Self::Asset, String> {
        Self::Asset::from_bytes(data).ok_or("Failed to load asset".to_string())
    }
}

pub trait AssetProcessor: 'static {
    type Importer: AssetImporter;

    fn process(
        asset: &mut <Self::Importer as AssetImporter>::Asset,
        settings: &<Self::Importer as AssetImporter>::Settings,
    ) -> Result<(), String>;
}

pub struct DefaultProcessor<I: AssetImporter>(std::marker::PhantomData<I>);

impl<I: AssetImporter> AssetProcessor for DefaultProcessor<I> {
    type Importer = I;

    fn process(
        _: &mut <Self::Importer as AssetImporter>::Asset,
        _: &<Self::Importer as AssetImporter>::Settings,
    ) -> Result<(), String> {
        Ok(())
    }
}

pub trait AssetImporter: 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Loader: AssetLoader<Asset = Self::Asset>;
    type Processor: AssetProcessor<Importer = Self>;

    fn import(context: &mut ImportContext<Self::Settings>) -> Result<Self::Asset, String>;
    fn extensions() -> &'static [&'static str];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStatus {
    None,
    Loading,
    Done,
    Failed,
}

pub struct Folder;

impl Folder {
    pub const EXT: &'static str = "folder";
}

impl Asset for Folder {}

impl ToBytes for Folder {
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn from_bytes(_: &[u8]) -> Option<Self> {
        Some(Folder)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FolderSettings {
    children: HashSet<PathBuf>,
}

impl FolderSettings {
    pub fn new() -> FolderSettings {
        FolderSettings {
            children: HashSet::new(),
        }
    }

    pub fn children(&self) -> std::collections::hash_set::Iter<PathBuf> {
        self.children.iter()
    }

    pub fn set_children(&mut self, children: HashSet<PathBuf>) -> Vec<PathBuf> {
        let removed = self.children.difference(&children).cloned().collect();
        self.children = children;

        removed
    }

    pub fn has_child(&self, path: &PathBuf) -> bool {
        self.children.contains(path)
    }
}

impl Settings for FolderSettings {}

impl AssetImporter for Folder {
    type Asset = Folder;
    type Settings = FolderSettings;
    type Loader = Folder;
    type Processor = DefaultProcessor<Folder>;

    fn import(
        _: &mut crate::importer::ImportContext<Self::Settings>,
    ) -> Result<Self::Asset, String> {
        Ok(Folder)
    }

    fn extensions() -> &'static [&'static str] {
        &[Folder::EXT]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportStep {
    Import,
    Process,
    Save,
}

#[derive(Debug, Clone)]
pub struct ImportFailed {
    path: PathBuf,
    id: AssetId,
    step: ImportStep,
    message: String,
}

impl ImportFailed {
    pub fn new(path: PathBuf, id: AssetId, step: ImportStep, message: String) -> Self {
        Self {
            path,
            id,
            step,
            message,
        }
    }

    pub fn import(path: PathBuf, id: AssetId, message: impl ToString) -> Self {
        Self::new(path, id, ImportStep::Import, message.to_string())
    }

    pub fn process(path: PathBuf, id: AssetId, message: impl ToString) -> Self {
        Self::new(path, id, ImportStep::Process, message.to_string())
    }

    pub fn save(path: PathBuf, id: AssetId, message: impl ToString) -> Self {
        Self::new(path, id, ImportStep::Save, message.to_string())
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn step(&self) -> &ImportStep {
        &self.step
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Event for ImportFailed {
    type Output = ImportFailed;

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        let mut library = database.library_mut();
        match library.status(&self.id) {
            AssetStatus::Loading | AssetStatus::Done => {
                library.set_status(self.id, AssetStatus::Failed);
            }
            _ => (),
        }

        Some(self)
    }
}

pub struct LoadFailed<A: Asset> {
    path: AssetPath,
    message: String,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> LoadFailed<A> {
    pub fn new(path: AssetPath, message: String) -> Self {
        Self {
            path,
            message,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl<A: Asset> Event for LoadFailed<A> {
    type Output = LoadFailed<A>;

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        match &self.path {
            AssetPath::Id(id) => {
                database.library_mut().set_status(*id, AssetStatus::Failed);
            }
            AssetPath::Path(path) => match database.library().source(path) {
                Some(source) => {
                    database
                        .library_mut()
                        .set_status(source.id(), AssetStatus::Failed);
                }
                None => (),
            },
        }

        Some(self)
    }
}
