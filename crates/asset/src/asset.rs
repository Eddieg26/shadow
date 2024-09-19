use ecs::core::{DenseMap, Resource};
use serde::ser::SerializeStruct;
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

    pub fn sub(&self, sub: usize) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        self.0.hash(&mut hasher);
        sub.hash(&mut hasher);
        AssetId(hasher.finish())
    }

    pub fn name(&self, name: &str) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        self.0.hash(&mut hasher);
        name.hash(&mut hasher);
        AssetId(hasher.finish())
    }

    pub const fn raw(id: u64) -> Self {
        Self(id)
    }
}

impl From<AssetId> for u64 {
    fn from(value: AssetId) -> Self {
        value.0
    }
}

impl From<&str> for AssetId {
    fn from(value: &str) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        value.hash(&mut hasher);
        AssetId(hasher.finish())
    }
}

impl std::ops::Deref for AssetId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
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

pub enum AssetHandle<A: Asset> {
    Id(AssetId),
    Asset(A),
}

impl<A: Asset> From<AssetId> for AssetHandle<A> {
    fn from(value: AssetId) -> Self {
        AssetHandle::Id(value)
    }
}

impl<A: Asset> From<&AssetId> for AssetHandle<A> {
    fn from(value: &AssetId) -> Self {
        AssetHandle::Id(*value)
    }
}

impl<A: Asset> From<A> for AssetHandle<A> {
    fn from(value: A) -> Self {
        AssetHandle::Asset(value)
    }
}

pub trait Asset: Send + Sync + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static {}
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

pub enum AssetAction<A: Asset> {
    Added(AssetId),
    Updated(AssetId),
    Removed(AssetId),
    None(AssetId, std::marker::PhantomData<A>),
}

impl<A: Asset> AssetAction<A> {
    pub fn id(&self) -> &AssetId {
        match self {
            Self::Added(id) => id,
            Self::Updated(id) => id,
            Self::Removed(id) => id,
            Self::None(id, _) => id,
        }
    }
}

impl<A: Asset> std::fmt::Debug for AssetAction<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Added(id) => write!(f, "Added({})", id),
            Self::Updated(id) => write!(f, "Updated({})", id),
            Self::Removed(id) => write!(f, "Removed({})", id),
            Self::None(id, _) => write!(f, "None({})", id),
        }
    }
}

pub struct AssetActions<A: Asset> {
    actions: Vec<AssetAction<A>>,
    _phantom: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetActions<A> {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn add(&mut self, id: AssetId) {
        self.actions.push(AssetAction::Added(id));
    }

    pub fn update(&mut self, id: AssetId) {
        self.actions.push(AssetAction::Updated(id));
    }

    pub fn remove(&mut self, id: AssetId) {
        self.actions.push(AssetAction::Removed(id));
    }

    pub fn iter(&self) -> std::slice::Iter<AssetAction<A>> {
        self.actions.iter()
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub(crate) fn clear(&mut self) {
        self.actions.clear();
    }
}

impl<A: Asset> std::fmt::Debug for AssetActions<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_list().entries(self.actions.iter()).finish()
    }
}

impl<A: Asset> Resource for AssetActions<A> {}

impl<A: Asset> Default for AssetActions<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: Asset> IntoIterator for AssetActions<A> {
    type Item = AssetAction<A>;
    type IntoIter = std::vec::IntoIter<AssetAction<A>>;

    fn into_iter(self) -> Self::IntoIter {
        self.actions.into_iter()
    }
}
