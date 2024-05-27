use crate::bytes::AsBytes;
use shadow_ecs::ecs::core::Resource;
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};
use ulid::Ulid;

pub trait Asset: AsBytes + Send + Sync + 'static {}

pub trait Settings: AsBytes + Default + Send + Sync + 'static {}

impl Settings for () {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetId(u64);

impl AssetId {
    pub fn new() -> AssetId {
        let id = Ulid::new();
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        AssetId(hasher.finish())
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl AsBytes for AssetId {
    fn as_bytes(&self) -> Vec<u8> {
        self.0.as_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value = u64::from_bytes(bytes)?;
        Some(AssetId(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetType(u64);

impl AssetType {
    pub fn of<A: Asset>() -> AssetType {
        let type_id = std::any::TypeId::of::<A>();
        let mut hasher = DefaultHasher::new();
        type_id.hash(&mut hasher);
        AssetType(hasher.finish())
    }
}

impl AsBytes for AssetType {
    fn as_bytes(&self) -> Vec<u8> {
        self.0.as_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value = u64::from_bytes(bytes)?;
        Some(AssetType(value))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetDependency {
    id: AssetId,
    ty: AssetType,
}

impl AssetDependency {
    pub fn new(id: AssetId, ty: AssetType) -> Self {
        Self { id, ty }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }
}

impl AsBytes for AssetDependency {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.as_bytes());
        bytes.extend_from_slice(&self.ty.as_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(&bytes[0..8])?;
        let ty = AssetType::from_bytes(&bytes[8..])?;
        Some(Self::new(id, ty))
    }
}

pub enum AssetPath {
    Id(AssetId),
    Path(PathBuf),
}

impl From<AssetId> for AssetPath {
    fn from(id: AssetId) -> Self {
        AssetPath::Id(id)
    }
}

impl From<PathBuf> for AssetPath {
    fn from(path: PathBuf) -> Self {
        AssetPath::Path(path)
    }
}

impl From<String> for AssetPath {
    fn from(path: String) -> Self {
        AssetPath::Path(PathBuf::from(path))
    }
}

impl From<&str> for AssetPath {
    fn from(path: &str) -> Self {
        AssetPath::Path(PathBuf::from(path))
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

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn settings(&self) -> &S {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut S {
        &mut self.settings
    }
}

impl<S: Settings> AsBytes for AssetMetadata<S> {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.as_bytes());
        bytes.extend_from_slice(&self.settings.as_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(&bytes[0..8])?;
        let settings = S::from_bytes(&bytes[8..])?;
        Some(Self::new(id, settings))
    }
}

impl<S: Settings> Default for AssetMetadata<S> {
    fn default() -> Self {
        Self {
            id: AssetId::new(),
            settings: S::default(),
        }
    }
}

pub struct AssetInfo {
    id: AssetId,
    checksum: u64,
}

impl AssetInfo {
    pub fn new(id: AssetId, checksum: u64) -> Self {
        Self { id, checksum }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn checksum(&self) -> u64 {
        self.checksum
    }

    pub fn calculate_checksum(asset: &[u8], settings: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        asset.hash(&mut hasher);
        settings.hash(&mut hasher);
        hasher.finish()
    }
}

impl AsBytes for AssetInfo {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.as_bytes());
        bytes.extend_from_slice(&self.checksum.as_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(&bytes[0..8])?;
        let checksum = u64::from_bytes(&bytes[8..])?;
        Some(Self::new(id, checksum))
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

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains_key(id)
    }

    pub fn insert(&mut self, id: AssetId, asset: A) -> Option<A> {
        self.assets.insert(id, asset)
    }

    pub fn get(&self, id: &AssetId) -> Option<&A> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut A> {
        self.assets.get_mut(id)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<A> {
        self.assets.remove(id)
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

    pub fn insert(&mut self, id: AssetId, settings: S) {
        self.settings.insert(id, settings);
    }

    pub fn get(&self, id: &AssetId) -> Option<&S> {
        self.settings.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut S> {
        self.settings.get_mut(id)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<S> {
        self.settings.remove(id)
    }
}

impl<S: Settings> Resource for AssetSettings<S> {}
