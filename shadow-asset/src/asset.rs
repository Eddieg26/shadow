use crate::bytes::AsBytes;
use shadow_ecs::ecs::core::Resource;
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};
use ulid::Ulid;

pub trait Asset: AsBytes + Send + Sync + 'static {}

impl Asset for () {}

pub trait Settings:
    serde::Serialize + serde::de::DeserializeOwned + AsBytes + Default + Send + Sync + 'static
{
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DefaultSettings(u8);

impl AsBytes for DefaultSettings {
    fn as_bytes(&self) -> Vec<u8> {
        Vec::new()
    }

    fn from_bytes(_: &[u8]) -> Option<Self> {
        Some(DefaultSettings(0))
    }
}

impl Settings for DefaultSettings {}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
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

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
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

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(serde::Serialize)]
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

impl<'de, S: Settings> serde::Deserialize<'de> for AssetMetadata<S> {
    fn deserialize<D>(deserializer: D) -> Result<AssetMetadata<S>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Id,
            Settings,
        }

        struct Visitor<S: Settings>(std::marker::PhantomData<S>);
        impl<'de, S: Settings> serde::de::Visitor<'de> for Visitor<S> {
            type Value = AssetMetadata<S>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a tuple of AssetId and Settings")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<AssetMetadata<S>, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let id = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &"a tuple of length 2"))?;
                let settings = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &"a tuple of length 2"))?;
                Ok(AssetMetadata::new(id, settings))
            }

            fn visit_map<V: serde::de::MapAccess<'de>>(
                self,
                mut map: V,
            ) -> Result<Self::Value, V::Error> {
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

        deserializer.deserialize_struct(
            "AssetMetadata",
            &["id", "settings"],
            Visitor(std::marker::PhantomData),
        )
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

    pub fn get_unchecked(&self, id: &AssetId) -> &A {
        self.assets.get(id).unwrap()
    }

    pub fn get_unchecked_mut(&mut self, id: &AssetId) -> &mut A {
        self.assets.get_mut(id).unwrap()
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
