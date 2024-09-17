use asset::{Asset, AssetId};
use ecs::core::{DenseMap, Resource};
use ecs::system::{ArgItem, SystemArg};
use std::hash::{Hash, Hasher};

pub mod binding;
pub mod buffer;
pub mod mesh;
pub mod pipeline;
pub mod shader;
pub mod texture;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(u64);
impl ResourceId {
    pub fn raw(id: u64) -> Self {
        Self(id)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

impl From<&str> for ResourceId {
    fn from(name: &str) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        name.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl From<String> for ResourceId {
    fn from(name: String) -> Self {
        let mut hasher = crc32fast::Hasher::new();
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

pub trait RenderAsset: Send + Sync + 'static {
    type Id: Hash + Eq + Copy + Send + Sync + From<AssetId> + 'static;
}

pub struct RenderAssets<R: RenderAsset> {
    assets: DenseMap<R::Id, R>,
}

impl<R: RenderAsset> RenderAssets<R> {
    pub fn new() -> Self {
        Self {
            assets: DenseMap::new(),
        }
    }

    pub fn add(&mut self, id: R::Id, asset: R) {
        self.assets.insert(id, asset);
    }

    pub fn get(&self, id: &R::Id) -> Option<&R> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &R::Id) -> Option<&mut R> {
        self.assets.get_mut(id)
    }

    pub fn remove(&mut self, id: &R::Id) -> Option<R> {
        self.assets.remove(id)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtractedResource {
    BindGroup,
    Pipeline,
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

    fn extracted_resource() -> Option<ExtractedResource> {
        None
    }
}
