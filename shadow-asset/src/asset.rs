use std::{
    any::TypeId,
    collections::HashMap,
    hash::{Hash, Hasher},
    path::PathBuf,
};

use shadow_ecs::ecs::core::Resource;

use crate::bytes::AsBytes;

pub trait Asset: AsBytes + Send + Sync + 'static {}

impl Asset for () {}

pub trait Settings: AsBytes + Default + Send + Sync + 'static {}

impl Settings for () {}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefaultSettings;

impl Settings for DefaultSettings {}

impl AsBytes for DefaultSettings {
    fn as_bytes(&self) -> Vec<u8> {
        0.as_bytes()
    }

    fn from_bytes(_: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetId(u64);

impl AssetId {
    pub fn new() -> Self {
        let id = ulid::Ulid::new();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher);
        AssetId(hasher.finish())
    }

    pub fn raw(value: u64) -> Self {
        AssetId(value)
    }
}

impl ToString for AssetId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl AsBytes for AssetId {
    fn as_bytes(&self) -> Vec<u8> {
        self.0.as_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(AssetId(u64::from_bytes(bytes)?))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetType(u64);

impl AssetType {
    pub fn of<A: Asset>() -> Self {
        let ty = TypeId::of::<A>();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        ty.hash(&mut hasher);
        AssetType(hasher.finish())
    }

    pub fn raw(value: u64) -> Self {
        AssetType(value)
    }
}

impl AsBytes for AssetType {
    fn as_bytes(&self) -> Vec<u8> {
        self.0.as_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(AssetType(u64::from_bytes(bytes)?))
    }
}

#[derive(Clone)]
pub enum AssetPath {
    Id(AssetId),
    Path(PathBuf),
}

impl<'a> From<&'a AssetPath> for AssetPath {
    fn from(value: &'a AssetPath) -> Self {
        value.clone()
    }
}

impl From<AssetId> for AssetPath {
    fn from(value: AssetId) -> Self {
        AssetPath::Id(value)
    }
}

impl From<PathBuf> for AssetPath {
    fn from(value: PathBuf) -> Self {
        AssetPath::Path(value)
    }
}

impl From<String> for AssetPath {
    fn from(value: String) -> Self {
        AssetPath::Path(value.into())
    }
}

impl<'a> From<&'a str> for AssetPath {
    fn from(value: &'a str) -> Self {
        AssetPath::Path(value.into())
    }
}

pub struct AssetMetadata<S: Settings> {
    id: AssetId,
    settings: S,
}

impl<S: Settings> AssetMetadata<S> {
    pub fn new(id: AssetId, settings: S) -> Self {
        Self { id, settings }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn settings(&self) -> &S {
        &self.settings
    }
}

impl<S: Settings> Default for AssetMetadata<S> {
    fn default() -> Self {
        Self::new(AssetId::new(), S::default())
    }
}

impl<S: Settings> AsBytes for AssetMetadata<S> {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend(self.id.as_bytes().iter());
        bytes.extend(self.settings.as_bytes().iter());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(bytes)?;
        let settings = S::from_bytes(&bytes[8..])?;
        Some(Self::new(id, settings))
    }
}

pub struct Assets<A: Asset> {
    assets: HashMap<AssetId, A>,
}

impl<A: Asset> Assets<A> {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn get(&self, id: &AssetId) -> Option<&A> {
        self.assets.get(id)
    }

    pub fn add(&mut self, id: AssetId, asset: A) -> Option<A> {
        self.assets.insert(id, asset)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<A> {
        self.assets.remove(id)
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<AssetId, A> {
        self.assets.keys()
    }

    pub fn values(&self) -> std::collections::hash_map::Values<AssetId, A> {
        self.assets.values()
    }

    pub fn values_mut(&mut self) -> std::collections::hash_map::ValuesMut<AssetId, A> {
        self.assets.values_mut()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<AssetId, A> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<AssetId, A> {
        self.assets.iter_mut()
    }

    pub fn clear(&mut self) {
        self.assets.clear()
    }
}

impl<A: Asset> Resource for Assets<A> {}

pub struct AssetSettings<S: Settings> {
    settings: HashMap<AssetId, S>,
}

impl<S: Settings> AssetSettings<S> {
    pub fn new() -> Self {
        Self {
            settings: HashMap::new(),
        }
    }

    pub fn get(&self, id: &AssetId) -> Option<&S> {
        self.settings.get(id)
    }

    pub fn add(&mut self, id: AssetId, settings: S) -> Option<S> {
        self.settings.insert(id, settings)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<S> {
        self.settings.remove(id)
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<AssetId, S> {
        self.settings.keys()
    }

    pub fn values(&self) -> std::collections::hash_map::Values<AssetId, S> {
        self.settings.values()
    }

    pub fn values_mut(&mut self) -> std::collections::hash_map::ValuesMut<AssetId, S> {
        self.settings.values_mut()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<AssetId, S> {
        self.settings.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<AssetId, S> {
        self.settings.iter_mut()
    }

    pub fn clear(&mut self) {
        self.settings.clear()
    }
}

impl<S: Settings> Resource for AssetSettings<S> {}
