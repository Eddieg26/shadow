use crate::bytes::IntoBytes;
use serde::ser::SerializeStruct;
use shadow_ecs::core::{DenseMap, Resource};
use std::{
    any::TypeId,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct AssetId(u64);

impl AssetId {
    pub fn gen() -> Self {
        let id = ulid::Ulid::new();
        let mut hasher = crc32fast::Hasher::new();
        id.hash(&mut hasher);
        AssetId(hasher.finish())
    }

    pub fn raw(id: u64) -> Self {
        Self(id)
    }
}

impl std::ops::Deref for AssetId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoBytes for AssetId {
    fn into_bytes(&self) -> Vec<u8> {
        self.0.into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        u64::from_bytes(bytes).map(Self::raw)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct AssetType(u32);

impl AssetType {
    pub fn of<A: Asset>() -> Self {
        let id = TypeId::of::<A>();
        let mut hasher = crc32fast::Hasher::new();
        id.hash(&mut hasher);
        AssetType(hasher.finalize())
    }

    pub fn raw(id: u32) -> Self {
        Self(id)
    }
}

impl std::ops::Deref for AssetType {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoBytes for AssetType {
    fn into_bytes(&self) -> Vec<u8> {
        self.0.into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        u32::from_bytes(bytes).map(Self::raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AssetKind {
    Main,
    Sub,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetPath {
    Id(AssetId),
    Path(PathBuf),
}

impl From<AssetId> for AssetPath {
    fn from(value: AssetId) -> Self {
        AssetPath::Id(value)
    }
}

impl From<&AssetId> for AssetPath {
    fn from(value: &AssetId) -> Self {
        AssetPath::Id(*value)
    }
}

impl<A: AsRef<Path>> From<A> for AssetPath {
    fn from(value: A) -> Self {
        AssetPath::Path(value.as_ref().to_path_buf())
    }
}

pub trait Asset: Send + Sync + 'static {}
pub trait Settings: Default + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DefaultSettings;

impl serde::Serialize for DefaultSettings {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: serde::Serializer,
    {
        let state = serializer.serialize_struct("DefaultSettings", 0)?;
        state.end()
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
                while let Some(_) = map.next_key::<serde::de::IgnoredAny>()? {}
                Ok(DefaultSettings)
            }
        }

        deserializer.deserialize_struct("DefaultSettings", &[], DefaultSettingsVisitor)
    }
}

impl Settings for DefaultSettings {}

pub struct AssetSettings<S: Settings> {
    id: AssetId,
    settings: S,
}

impl<S: Settings> AssetSettings<S> {
    pub fn new(id: AssetId, settings: S) -> Self {
        Self { id, settings }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn take(self) -> (AssetId, S) {
        (self.id, self.settings)
    }
}

impl<S: Settings> Default for AssetSettings<S> {
    fn default() -> Self {
        Self {
            id: AssetId::gen(),
            settings: Default::default(),
        }
    }
}

impl<S: Settings> std::ops::Deref for AssetSettings<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.settings
    }
}

impl<S: Settings> std::ops::DerefMut for AssetSettings<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.settings
    }
}

impl<S: Settings> serde::Serialize for AssetSettings<S> {
    fn serialize<Ser>(&self, ser: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: serde::Serializer,
    {
        let mut object = ser.serialize_struct("Metadata", 2)?;
        object.serialize_field("id", &self.id)?;
        object.serialize_field("settings", &self.settings)?;
        object.end()
    }
}

impl<'de, S: Settings> serde::Deserialize<'de> for AssetSettings<S> {
    fn deserialize<D>(de: D) -> Result<AssetSettings<S>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Id,
            Settings,
        }

        struct Visitor<S: Settings>(std::marker::PhantomData<S>);

        impl<'de, S: Settings> serde::de::Visitor<'de> for Visitor<S> {
            type Value = AssetSettings<S>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct AssetSettings")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
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

                Ok(AssetSettings { id, settings })
            }
        }

        const FIELDS: &[&str] = &["id", "settings"];
        de.deserialize_struct("Metadata", FIELDS, Visitor(std::marker::PhantomData))
    }
}

#[derive(Debug)]
pub struct Assets<A: Asset> {
    assets: DenseMap<AssetId, A>,
}

impl<A: Asset> Assets<A> {
    pub fn new() -> Self {
        Self {
            assets: DenseMap::new(),
        }
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains(id)
    }

    pub fn get(&self, id: &AssetId) -> Option<&A> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut A> {
        self.assets.get_mut(id)
    }

    pub fn add(&mut self, id: AssetId, asset: A) -> Option<A> {
        self.assets.insert(id, asset)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<A> {
        self.assets.remove(id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn iter(&self) -> std::iter::Zip<std::slice::Iter<AssetId>, std::slice::Iter<A>> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut A)> {
        self.assets.iter_mut()
    }

    pub fn ids(&self) -> &[AssetId] {
        self.assets.keys()
    }

    pub fn assets(&self) -> &[A] {
        self.assets.values()
    }

    pub fn assets_mut(&mut self) -> &mut [A] {
        self.assets.values_mut()
    }

    pub fn clear(&mut self) {
        self.assets.clear();
    }
}

impl<A: Asset> Resource for Assets<A> {}

impl<A: Asset> Default for Assets<A> {
    fn default() -> Self {
        Self::new()
    }
}
