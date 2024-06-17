use crate::bytes::ToBytes;
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use shadow_ecs::ecs::core::Resource;
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

pub trait Asset: ToBytes + Send + Sync + 'static {}

impl Asset for () {}

pub trait Settings:
    ToBytes + Default + Serialize + for<'a> Deserialize<'a> + Send + Sync + 'static
{
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BasicSettings;

impl ToBytes for BasicSettings {
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            Some(BasicSettings)
        } else {
            None
        }
    }
}

impl Settings for BasicSettings {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl From<&AssetId> for AssetPath {
    fn from(id: &AssetId) -> Self {
        AssetPath::Id(*id)
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

    pub fn with_id(mut self, id: AssetId) -> Self {
        self.id = id;
        self
    }

    pub fn settings(&self) -> &S {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut S {
        &mut self.settings
    }

    pub fn take(self) -> (AssetId, S) {
        (self.id, self.settings)
    }
}

impl<S: Settings> Default for AssetMetadata<S> {
    fn default() -> Self {
        AssetMetadata {
            id: AssetId::gen(),
            settings: S::default(),
        }
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

impl<S: Settings> Serialize for AssetMetadata<S> {
    fn serialize<D>(&self, serializer: D) -> Result<D::Ok, D::Error>
    where
        D: serde::Serializer,
    {
        let id = &self.id;
        let settings = &self.settings;

        let mut state = serializer.serialize_struct("AssetMetadata", 2)?;
        state.serialize_field("id", id)?;
        state.serialize_field("settings", settings)?;
        state.end()
    }
}

impl<'a, S: Settings> Deserialize<'a> for AssetMetadata<S> {
    fn deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Id,
            Settings,
        }

        struct Visitor<S: Settings>(std::marker::PhantomData<S>);

        impl<'a, S: Settings> serde::de::Visitor<'a> for Visitor<S> {
            type Value = AssetMetadata<S>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct AssetMetadata")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'a>,
            {
                let mut id = None;
                let mut settings = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        Field::Settings => {
                            if settings.is_some() {
                                return Err(serde::de::Error::duplicate_field("settings"));
                            }
                            settings = Some(map.next_value()?);
                        }
                    }
                }

                let id = id.ok_or_else(|| serde::de::Error::missing_field("id"))?;
                let settings =
                    settings.ok_or_else(|| serde::de::Error::missing_field("settings"))?;

                Ok(AssetMetadata { id, settings })
            }
        }

        const FIELDS: &[&str] = &["id", "settings"];
        deserializer.deserialize_struct("AssetMetadata", FIELDS, Visitor(std::marker::PhantomData))
    }
}

pub struct Folder;
impl Asset for Folder {}

impl ToBytes for Folder {
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            Some(Folder)
        } else {
            None
        }
    }
}

pub struct Assets<A: Asset> {
    assets: HashMap<AssetId, A>,
}

impl<A: Asset> Assets<A> {
    pub fn new() -> Self {
        Assets {
            assets: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, asset: A) -> Option<A> {
        self.assets.insert(id, asset)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<A> {
        self.assets.remove(id)
    }

    pub fn get(&self, id: &AssetId) -> Option<&A> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut A> {
        self.assets.get_mut(id)
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &A)> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut A)> {
        self.assets.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

impl<A: Asset> Resource for Assets<A> {}

pub struct AssetSettings<S: Settings> {
    settings: HashMap<AssetId, S>,
}

impl<S: Settings> AssetSettings<S> {
    pub fn new() -> Self {
        AssetSettings {
            settings: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, settings: S) -> Option<S> {
        self.settings.insert(id, settings)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<S> {
        self.settings.remove(id)
    }

    pub fn get(&self, id: &AssetId) -> Option<&S> {
        self.settings.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut S> {
        self.settings.get_mut(id)
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.settings.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &S)> {
        self.settings.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut S)> {
        self.settings.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.settings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.settings.is_empty()
    }
}

impl<S: Settings> Resource for AssetSettings<S> {}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FolderSettings {
    children: HashSet<PathBuf>,
}

impl FolderSettings {
    pub fn new() -> Self {
        FolderSettings {
            children: HashSet::new(),
        }
    }

    pub fn insert(&mut self, path: PathBuf) -> bool {
        self.children.insert(path)
    }

    pub fn remove(&mut self, path: &Path) -> bool {
        self.children.remove(path)
    }

    pub fn set_children(&mut self, children: HashSet<PathBuf>) {
        self.children = children;
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.children.contains(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.children.iter()
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl ToBytes for FolderSettings {
    fn to_bytes(&self) -> Vec<u8> {
        self.children.iter().cloned().collect::<Vec<_>>().to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let children = Vec::<PathBuf>::from_bytes(bytes)?;
        let mut settings = FolderSettings::new();
        for child in children {
            settings.insert(child);
        }
        Some(settings)
    }
}

impl Settings for FolderSettings {}
