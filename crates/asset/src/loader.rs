use crate::{
    artifact::ArtifactMeta,
    asset::{Asset, AssetId, AssetPath, AssetSettings, Settings},
    database::registry::ImportedAsset,
    io::{AssetIoError, AssetReader},
    DefaultSettings,
};
use ecs::{
    core::{internal::blob::BlobCell, DenseMap},
    world::{event::Event, World},
};
use std::{
    collections::HashSet,
    error::Error,
    path::{Path, PathBuf},
};

pub struct LoadContext<'a, S: Settings> {
    settings: &'a AssetSettings<S>,
    dependencies: HashSet<AssetId>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(settings: &'a AssetSettings<S>) -> Self {
        Self {
            settings,
            dependencies: HashSet::new(),
        }
    }

    pub fn settings(&self) -> &AssetSettings<S> {
        self.settings
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn finish(self) -> HashSet<AssetId> {
        self.dependencies
    }
}

pub trait AssetLoader: 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Error: Error + Send + Sync + 'static;

    fn load(
        ctx: &mut LoadContext<Self::Settings>,
        reader: &mut dyn AssetReader,
    ) -> Result<Self::Asset, Self::Error>;
    fn extensions() -> &'static [&'static str];
}

pub struct ProcessContext<'a, S: Settings> {
    id: AssetId,
    settings: &'a mut S,
    assets: &'a mut LoadedAssets,
    sub_assets: Vec<ImportedAsset>,
}

impl<'a, S: Settings> ProcessContext<'a, S> {
    pub fn new(id: AssetId, settings: &'a mut S, assets: &'a mut LoadedAssets) -> Self {
        Self {
            id,
            settings,
            assets,
            sub_assets: Vec::new(),
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn asset<A: Asset>(&self, id: &AssetId) -> Option<&A> {
        self.assets.get::<A>(id)
    }

    pub fn settings(&self) -> &S {
        self.settings
    }

    pub fn settings_mut(&mut self) -> &mut S {
        self.settings
    }

    pub fn add_sub_asset<A: Asset>(&mut self, id: AssetId, asset: A) {
        let dependencies = HashSet::from([self.id]);
        let meta = ArtifactMeta::new::<A>(id, 0, dependencies).with_parent(self.id);
        let asset = ImportedAsset::new(asset, DefaultSettings, meta);
        self.sub_assets.push(asset);
    }

    pub fn finish(self) -> Vec<ImportedAsset> {
        self.sub_assets
    }
}

pub trait AssetProcessor: 'static {
    type Loader: AssetLoader;
    type Error: Error + Send + Sync + 'static;

    fn process(
        asset: &mut <Self::Loader as AssetLoader>::Asset,
        ctx: &mut ProcessContext<<Self::Loader as AssetLoader>::Settings>,
    ) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub struct AssetError {
    error: Box<dyn Error + Send + Sync + 'static>,
    kind: AssetErrorKind,
}

impl AssetError {
    pub fn new(kind: AssetErrorKind, error: impl Error + Send + Sync + 'static) -> Self {
        Self {
            error: Box::new(error),
            kind,
        }
    }

    pub fn import(path: impl AsRef<Path>, error: impl Error + Send + Sync + 'static) -> Self {
        Self {
            error: Box::new(error),
            kind: AssetErrorKind::Import(path.as_ref().to_path_buf()),
        }
    }

    pub fn load(path: impl Into<AssetPath>, error: impl Error + Send + Sync + 'static) -> Self {
        Self {
            error: Box::new(error),
            kind: AssetErrorKind::Load(path.into()),
        }
    }

    pub fn error(&self) -> &dyn Error {
        &*self.error
    }

    pub fn kind(&self) -> &AssetErrorKind {
        &self.kind
    }
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Asset: {:?} Load error: {}", self.kind, self.error)
    }
}

impl Error for AssetError {}

impl Event for AssetError {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetErrorKind {
    Import(PathBuf),
    Load(AssetPath),
}

#[derive(Debug)]
pub enum LoadErrorKind {
    Io(AssetIoError),
    NoExtension,
    NoLoader,
    NoSerializer,
    InvalidExtension(String),
}

impl std::fmt::Display for LoadErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadErrorKind::Io(err) => write!(f, "IO error: {}", err),
            LoadErrorKind::NoLoader => write!(f, "No importer found"),
            LoadErrorKind::InvalidExtension(ext) => write!(f, "Invalid extension: {}", ext),
            LoadErrorKind::NoExtension => write!(f, "No extension found"),
            LoadErrorKind::NoSerializer => write!(f, "No serializer found"),
        }
    }
}

impl Error for LoadErrorKind {}

pub struct LoadedAsset {
    pub asset: BlobCell,
    pub meta: ArtifactMeta,
}

impl LoadedAsset {
    pub fn new<A: Asset>(asset: A, meta: ArtifactMeta) -> Self {
        Self {
            asset: BlobCell::new(asset),
            meta,
        }
    }

    pub fn asset<A: Asset>(&self) -> &A {
        self.asset.value()
    }

    pub fn asset_mut<A: Asset>(&mut self) -> &mut A {
        self.asset.value_mut()
    }

    pub fn meta(&self) -> &ArtifactMeta {
        &self.meta
    }

    pub fn into<A: Asset>(self) -> A {
        self.asset.take()
    }
}

pub struct LoadedMetadata {
    pub id: AssetId,
    data: String,
}

impl LoadedMetadata {
    pub fn new(id: AssetId, data: String) -> Self {
        Self { id, data }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_bytes()
    }

    pub fn into_data(self) -> String {
        self.data
    }
}

pub struct LoadedAssets {
    assets: DenseMap<AssetId, LoadedAsset>,
}

impl LoadedAssets {
    pub fn new() -> Self {
        Self {
            assets: DenseMap::new(),
        }
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains(id)
    }

    pub fn get<A: Asset>(&self, id: &AssetId) -> Option<&A> {
        self.assets.get(id).map(|asset| asset.asset())
    }

    pub fn get_mut<A: Asset>(&mut self, id: &AssetId) -> Option<&mut A> {
        self.assets.get_mut(id).map(|asset| asset.asset_mut())
    }

    pub fn add<A: Asset>(&mut self, asset: A, meta: ArtifactMeta) -> Option<LoadedAsset> {
        self.assets.insert(meta.id(), LoadedAsset::new(asset, meta))
    }

    pub fn get_erased(&self, id: &AssetId) -> Option<&LoadedAsset> {
        self.assets.get(id)
    }

    pub fn get_erased_mut(&mut self, id: &AssetId) -> Option<&mut LoadedAsset> {
        self.assets.get_mut(id)
    }

    pub fn add_erased(&mut self, id: AssetId, asset: LoadedAsset) -> Option<LoadedAsset> {
        self.assets.insert(id, asset)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<LoadedAsset> {
        self.assets.remove(id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn ids(&self) -> &[AssetId] {
        self.assets.keys()
    }

    pub fn clear(&mut self) {
        self.assets.clear();
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (AssetId, LoadedAsset)> + '_ {
        self.assets.drain()
    }
}
