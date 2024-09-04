use asset::{Asset, AssetId};
use ecs::core::{DenseMap, Resource};
use ecs::system::{ArgItem, SystemArg};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub mod binding;
pub mod buffer;
pub mod material;
pub mod mesh;
pub mod pipeline;
pub mod shader;
pub mod texture;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(u64);

impl ResourceId {
    pub fn gen() -> Self {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(0);
        Self(hasher.finish())
    }
}

impl From<&str> for ResourceId {
    fn from(name: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl From<String> for ResourceId {
    fn from(name: String) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl From<AssetId> for ResourceId {
    fn from(id: AssetId) -> Self {
        Self(u64::from(id))
    }
}

impl From<&AssetId> for ResourceId {
    fn from(id: &AssetId) -> Self {
        Self(u64::from(*id))
    }
}

#[derive(
    Copy, Default, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum ReadWrite {
    Enabled,

    #[default]
    Disabled,
}

pub trait ExtractArg<'a>: SystemArg {}

impl<'a, R: Resource> ExtractArg<'a> for &R {}
impl<'a> ExtractArg<'a> for () {}
impl<'a, A: ExtractArg<'a>, B: ExtractArg<'a>> ExtractArg<'a> for (A, B) {}
impl<'a, A: ExtractArg<'a>, B: ExtractArg<'a>, C: ExtractArg<'a>> ExtractArg<'a> for (A, B, C) {}
impl<'a, A: ExtractArg<'a>, B: ExtractArg<'a>, C: ExtractArg<'a>, D: ExtractArg<'a>> ExtractArg<'a>
    for (A, B, C, D)
{
}

pub trait RenderAsset: Send + Sync + 'static {}

pub struct RenderAssets<R: RenderAsset> {
    assets: DenseMap<ResourceId, R>,
}

impl<R: RenderAsset> RenderAssets<R> {
    pub fn new() -> Self {
        Self {
            assets: DenseMap::new(),
        }
    }

    pub fn insert(&mut self, id: ResourceId, asset: R) {
        self.assets.insert(id, asset);
    }

    pub fn get(&self, id: &ResourceId) -> Option<&R> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &ResourceId) -> Option<&mut R> {
        self.assets.get_mut(id)
    }

    pub fn remove(&mut self, id: &ResourceId) {
        self.assets.remove(id);
    }
}

impl<R: RenderAsset> Default for RenderAssets<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: RenderAsset> Resource for RenderAssets<R> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetUsage {
    Keep,
    Discard,
}

pub trait RenderAssetExtractor: 'static {
    type Source: Asset;
    type Target: RenderAsset;
    type Arg<'a>: ExtractArg<'a>;

    fn extract<'a>(source: &mut Self::Source, arg: &ArgItem<Self::Arg<'a>>)
        -> Option<Self::Target>;
    fn update<'a>(
        _source: &mut Self::Source,
        _asset: &mut Self::Target,
        _arg: &ArgItem<Self::Arg<'a>>,
    ) {
    }

    fn usage(_source: &Self::Source) -> AssetUsage {
        AssetUsage::Keep
    }
}
