use serde::ser::SerializeStruct;
use shadow_ecs::ecs::core::Resource;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use crate::bytes::ToBytes;

pub trait Asset: Send + Sync + 'static {}

pub trait Settings:
    Send + Sync + Default + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static
{
}

impl Asset for () {}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefaultSettings;

impl ToBytes for DefaultSettings {
    fn to_bytes(&self) -> Vec<u8> {
        Vec::new()
    }

    fn from_bytes(_bytes: &[u8]) -> Option<Self> {
        Some(DefaultSettings)
    }
}

impl serde::Serialize for DefaultSettings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let obj = serializer.serialize_struct("DefaultSettings", 0)?;
        obj.end()
    }
}

impl<'de> serde::Deserialize<'de> for DefaultSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DefaultSettingsVisitor;

        impl<'de> serde::de::Visitor<'de> for DefaultSettingsVisitor {
            type Value = DefaultSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct DefaultSettings")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                while let Some(_) = map.next_entry::<String, serde::de::IgnoredAny>()? {}
                Ok(DefaultSettings)
            }
        }

        deserializer.deserialize_struct("DefaultSettings", &[], DefaultSettingsVisitor)
    }
}

impl Settings for DefaultSettings {}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct AssetId(u64);

impl AssetId {
    pub fn new(value: u64) -> Self {
        AssetId(value)
    }

    pub fn gen() -> Self {
        let id = ulid::Ulid::new();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher);
        AssetId(hasher.finish())
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
        Some(AssetId(u64::from_bytes(bytes)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetPath {
    Id(AssetId),
    Path(PathBuf),
}

impl From<AssetId> for AssetPath {
    fn from(id: AssetId) -> Self {
        AssetPath::Id(id)
    }
}

impl<A: AsRef<Path>> From<A> for AssetPath {
    fn from(path: A) -> Self {
        AssetPath::Path(path.as_ref().to_path_buf())
    }
}

impl From<&AssetPath> for AssetPath {
    fn from(path: &AssetPath) -> Self {
        path.clone()
    }
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[repr(C)]
pub struct AssetType(u64);

impl AssetType {
    pub fn of<A: Asset>() -> Self {
        let ty = std::any::TypeId::of::<A>();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        ty.hash(&mut hasher);
        AssetType(hasher.finish())
    }
}

impl ToBytes for AssetType {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(AssetType(u64::from_bytes(bytes)?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Type(u64);

impl Type {
    pub fn of<T: 'static>() -> Self {
        let ty = std::any::TypeId::of::<T>();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        ty.hash(&mut hasher);
        Type(hasher.finish())
    }
}

pub struct AssetMetadata<S: Settings> {
    id: AssetId,
    settings: S,
}

impl<S: Settings> AssetMetadata<S> {
    pub fn new(settings: S) -> Self {
        AssetMetadata {
            id: AssetId::gen(),
            settings,
        }
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

impl<S: Settings> serde::Serialize for AssetMetadata<S> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("AssetMetadata", 2)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("settings", &self.settings)?;
        state.end()
    }
}

impl<'de, S: Settings> serde::Deserialize<'de> for AssetMetadata<S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct AssetMetadataVisitor<S: Settings>(std::marker::PhantomData<S>);

        impl<'de, S: Settings> serde::de::Visitor<'de> for AssetMetadataVisitor<S> {
            type Value = AssetMetadata<S>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct AssetMetadata")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut id = None;
                let mut settings = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "id" => {
                            if id.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        "settings" => {
                            if settings.is_some() {
                                return Err(serde::de::Error::duplicate_field("settings"));
                            }
                            settings = Some(map.next_value()?);
                        }
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                let id = id.ok_or_else(|| serde::de::Error::missing_field("id"))?;
                let settings =
                    settings.ok_or_else(|| serde::de::Error::missing_field("settings"))?;
                Ok(AssetMetadata { id, settings })
            }
        }

        deserializer.deserialize_struct(
            "AssetMetadata",
            &["id", "settings"],
            AssetMetadataVisitor(Default::default()),
        )
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

    pub fn get(&self, id: &AssetId) -> Option<&A> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut A> {
        self.assets.get_mut(id)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<A> {
        self.assets.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &A)> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut A)> {
        self.assets.iter_mut()
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn clear(&mut self) {
        self.assets.clear();
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

    pub fn get(&self, id: &AssetId) -> Option<&S> {
        self.settings.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut S> {
        self.settings.get_mut(id)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<S> {
        self.settings.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &S)> {
        self.settings.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut S)> {
        self.settings.iter_mut()
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.settings.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.settings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.settings.is_empty()
    }

    pub fn clear(&mut self) {
        self.settings.clear();
    }
}

impl<S: Settings> Resource for AssetSettings<S> {}
