use crate::asset::{
    Asset, AssetId, AssetMetadata, AssetSettings, AssetType, Assets, Settings, Type,
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub struct LoadContext<'a, S: Settings> {
    path: &'a Path,
    asset: &'a [u8],
    metadata: &'a AssetMetadata<S>,
    dependencies: Vec<AssetId>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(path: &'a Path, asset: &'a [u8], metadata: &'a AssetMetadata<S>) -> Self {
        LoadContext {
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

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    LoaderNotFound,
    InvalidAsset { message: String },
    InvalidMetadata { message: String },
}

pub trait AssetCacher: 'static {
    type Asset: Asset;

    fn cache(asset: &Self::Asset) -> Vec<u8>;
    fn load(data: &[u8]) -> Result<Self::Asset, LoadError>;
}

pub trait AssetLoader: 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Cacher: AssetCacher<Asset = Self::Asset>;

    fn load(context: &mut LoadContext<Self::Settings>) -> Result<Self::Asset, LoadError>;
    fn extensions() -> &'static [&'static str];
}

pub enum ProcessError {
    AssetNotFound { id: AssetId },
    SettingsNotFound { id: AssetId },
    InvalidAsset { message: String },
    InvalidSettings { message: String },
}

pub trait AssetProcessor: 'static {
    type Loader: AssetLoader;

    fn process(
        asset: &mut <Self::Loader as AssetLoader>::Asset,
        settings: &<Self::Loader as AssetLoader>::Settings,
        assets: &AssetStorage,
    ) -> Result<(), ProcessError>;
}

pub enum SaveError {
    Io(std::io::Error),
    AssetNotFound { id: AssetId },
    InvalidAsset { message: String },
}

pub enum ImportError {
    Load(LoadError),
    Process(ProcessError),
    Save(SaveError),
}

impl ImportError {
    pub fn new(value: impl Into<ImportError>) -> Self {
        value.into()
    }
}

impl From<LoadError> for ImportError {
    fn from(error: LoadError) -> Self {
        ImportError::Load(error)
    }
}

impl From<ProcessError> for ImportError {
    fn from(error: ProcessError) -> Self {
        ImportError::Process(error)
    }
}

impl From<SaveError> for ImportError {
    fn from(error: SaveError) -> Self {
        ImportError::Save(error)
    }
}

pub trait BaseStorage: 'static {
    fn clear(&mut self);
    fn remove(&mut self, id: &AssetId);
    fn contains(&self, id: &AssetId) -> bool;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<A: Asset> BaseStorage for Assets<A> {
    fn clear(&mut self) {
        Assets::<A>::clear(self);
    }

    fn remove(&mut self, id: &AssetId) {
        Assets::<A>::remove(self, id);
    }

    fn contains(&self, id: &AssetId) -> bool {
        Assets::<A>::contains(self, id)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl<S: Settings> BaseStorage for AssetSettings<S> {
    fn clear(&mut self) {
        AssetSettings::<S>::clear(self);
    }

    fn remove(&mut self, id: &AssetId) {
        AssetSettings::<S>::remove(self, id);
    }

    fn contains(&self, id: &AssetId) -> bool {
        AssetSettings::<S>::contains(self, id)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct AssetStorage {
    assets: HashMap<AssetType, Box<dyn BaseStorage>>,
    settings: HashMap<Type, Box<dyn BaseStorage>>,
}

impl AssetStorage {
    pub fn new() -> Self {
        AssetStorage {
            assets: HashMap::new(),
            settings: HashMap::new(),
        }
    }

    pub fn asset<A: Asset>(&self, id: &AssetId) -> Option<&A> {
        self.assets
            .get(&AssetType::of::<A>())
            .and_then(|storage| storage.as_any().downcast_ref::<Assets<A>>())
            .and_then(|storage| storage.get(id))
    }

    pub fn insert<A: Asset>(&mut self, id: AssetId, asset: A) {
        self.assets
            .entry(AssetType::of::<A>())
            .or_insert_with(|| Box::new(Assets::<A>::new()))
            .as_any_mut()
            .downcast_mut::<Assets<A>>()
            .unwrap()
            .insert(id, asset);
    }

    pub fn remove<A: Asset>(&mut self, id: &AssetId) -> Option<A> {
        self.assets
            .get_mut(&AssetType::of::<A>())
            .and_then(|storage| storage.as_any_mut().downcast_mut::<Assets<A>>())
            .and_then(|storage| storage.remove(id))
    }

    pub fn remove_asset_by_ty(&mut self, id: &AssetId, ty: &AssetType) {
        if let Some(assets) = self.assets.get_mut(ty) {
            assets.remove(id);
        }
    }

    pub fn settings<S: Settings>(&self, id: &AssetId) -> Option<&S> {
        self.settings
            .get(&Type::of::<S>())
            .and_then(|storage| storage.as_any().downcast_ref::<AssetSettings<S>>())
            .and_then(|storage| storage.get(id))
    }

    pub fn insert_settings<S: Settings>(&mut self, id: AssetId, settings: S) {
        self.settings
            .entry(Type::of::<S>())
            .or_insert_with(|| Box::new(AssetSettings::<S>::new()))
            .as_any_mut()
            .downcast_mut::<AssetSettings<S>>()
            .unwrap()
            .insert(id, settings);
    }

    pub fn remove_settings<S: Settings>(&mut self, id: &AssetId) -> Option<S> {
        self.settings
            .get_mut(&Type::of::<S>())
            .and_then(|storage| storage.as_any_mut().downcast_mut::<AssetSettings<S>>())
            .and_then(|storage| storage.remove(id))
    }

    pub fn remove_settings_by_ty(&mut self, id: &AssetId, ty: &Type) {
        if let Some(settings) = self.settings.get_mut(ty) {
            settings.remove(id);
        }
    }

    pub fn contains_asset(&self, id: &AssetId, ty: &AssetType) -> bool {
        self.assets
            .get(ty)
            .map(|storage| storage.contains(id))
            .unwrap_or(false)
    }

    pub fn contains_settings(&self, id: &AssetId, ty: &Type) -> bool {
        self.settings
            .get(ty)
            .map(|storage| storage.contains(id))
            .unwrap_or(false)
    }

    pub fn clear(&mut self) {
        self.assets.clear();
        self.settings.clear();
    }
}
