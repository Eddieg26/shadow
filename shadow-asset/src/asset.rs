use crate::bytes::ToBytes;
use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

pub trait Asset: ToBytes + Send + Sync + 'static {}

impl Asset for () {}

pub trait Settings: ToBytes + Default + Send + Sync + 'static {}

pub struct DefaultSettings;

impl ToBytes for DefaultSettings {
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            Some(DefaultSettings)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetId(u64);

impl AssetId {
    pub fn new(id: u64) -> Self {
        AssetId(id)
    }

    pub fn gen() -> Self {
        let id = ulid::Ulid::new();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher);
        AssetId(hasher.finish())
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl ToString for AssetId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl ToBytes for AssetId {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        u64::from_bytes(bytes).map(AssetId)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetType(u64);

impl AssetType {
    pub fn of<A: Asset>() -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::any::TypeId::of::<A>().hash(&mut hasher);
        AssetType(hasher.finish())
    }

    pub fn new(id: u64) -> Self {
        AssetType(id)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl ToBytes for AssetType {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        u64::from_bytes(bytes).map(AssetType)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetPath {
    Id(AssetId),
    Path(PathBuf),
}

impl AssetPath {
    pub fn id(id: AssetId) -> Self {
        AssetPath::Id(id)
    }

    pub fn path(path: PathBuf) -> Self {
        AssetPath::Path(path)
    }

    pub fn as_id(&self) -> Option<AssetId> {
        match self {
            AssetPath::Id(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_path(&self) -> Option<&PathBuf> {
        match self {
            AssetPath::Path(path) => Some(path),
            _ => None,
        }
    }
}

impl From<AssetId> for AssetPath {
    fn from(id: AssetId) -> Self {
        AssetPath::Id(id)
    }
}

impl From<&AssetPath> for AssetPath {
    fn from(path: &AssetPath) -> Self {
        path.clone()
    }
}

impl<A: AsRef<Path>> From<A> for AssetPath {
    fn from(path: A) -> Self {
        AssetPath::Path(path.as_ref().to_path_buf())
    }
}

pub struct AssetMetadata<S: Settings> {
    id: AssetId,
    settings: S,
}

impl<S: Settings> AssetMetadata<S> {
    pub fn new(id: AssetId, settings: S) -> Self {
        AssetMetadata { id, settings }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn settings(&self) -> &S {
        &self.settings
    }

    pub fn take(self) -> (AssetId, S) {
        (self.id, self.settings)
    }
}

impl<S: Settings> ToBytes for AssetMetadata<S> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.id.to_bytes();
        bytes.extend_from_slice(&self.settings.to_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(bytes)?;
        let settings = S::from_bytes(&bytes[8..])?;
        Some(AssetMetadata { id, settings })
    }
}
