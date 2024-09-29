use asset::{Asset, AssetId};
use ecs::core::{DenseMap, Resource};
use ecs::system::{ArgItem, SystemArg};
use ecs::world::event::Event;
use std::collections::HashSet;
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

pub trait RenderAsset: Send + Sync + 'static {
    type Id: Hash + Eq + Copy + Send + Sync + 'static;

    fn world() -> RenderAssetWorld {
        RenderAssetWorld::Render
    }
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

    pub fn get(&self, id: &R::Id) -> Option<&R> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &R::Id) -> Option<&mut R> {
        self.assets.get_mut(id)
    }

    pub fn add(&mut self, id: R::Id, asset: R) -> Option<R> {
        self.assets.insert(id, asset)
    }

    pub fn remove(&mut self, id: &R::Id) -> Option<R> {
        self.assets.remove(id)
    }

    pub fn contains(&self, id: &R::Id) -> bool {
        self.assets.contains(id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn clear(&mut self) {
        self.assets.clear()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&R::Id, &R)> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&R::Id, &mut R)> {
        self.assets.iter_mut()
    }

    pub fn sort(&mut self, sorter: impl FnMut(&(R::Id, R), &(R::Id, R)) -> std::cmp::Ordering) {
        self.assets.sort(sorter);
    }
}

impl<R: RenderAsset> std::ops::Index<usize> for RenderAssets<R> {
    type Output = R;

    fn index(&self, index: usize) -> &Self::Output {
        &self.assets.values()[index]
    }
}

impl<R: RenderAsset> std::ops::IndexMut<usize> for RenderAssets<R> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.assets.values_mut()[index]
    }
}

impl<R: RenderAsset> Default for RenderAssets<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: RenderAsset> Resource for RenderAssets<R> {}

pub struct DiscardedAssets<A: Asset>(HashSet<AssetId>, std::marker::PhantomData<A>);
impl<A: Asset> Resource for DiscardedAssets<A> {}
impl<A: Asset> std::ops::Deref for DiscardedAssets<A> {
    type Target = HashSet<AssetId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<A: Asset> std::ops::DerefMut for DiscardedAssets<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<A: Asset> Default for DiscardedAssets<A> {
    fn default() -> Self {
        Self(Default::default(), Default::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetUsage {
    Keep,
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderAssetWorld {
    Main,
    Render,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtractedResource {
    BindGroup,
    Pipeline,
}

pub trait RenderAssetExtractor: 'static {
    type Source: Asset;
    type Target: RenderAsset;
    type Arg: SystemArg;

    fn extract(
        id: &AssetId,
        source: &mut Self::Source,
        arg: &mut ArgItem<Self::Arg>,
        assets: &mut RenderAssets<Self::Target>,
    ) -> Option<AssetUsage>;

    fn remove(id: &AssetId, assets: &mut RenderAssets<Self::Target>, arg: &mut ArgItem<Self::Arg>);

    fn extracted_resource() -> Option<ExtractedResource> {
        None
    }
}

pub trait RenderResourceExtractor: Send + Sync + 'static {
    type Event: Event;
    type Target: Resource;
    type Arg: SystemArg;

    fn extract(arg: &ArgItem<Self::Arg>) -> Option<Self::Target>;

    fn extracted_resource() -> Option<ExtractedResource> {
        None
    }
}

pub struct ExtractResource<R: RenderResourceExtractor> {
    pub _marker: std::marker::PhantomData<R>,
}

impl<R: RenderResourceExtractor> ExtractResource<R> {
    pub fn new() -> Self {
        Self {
            _marker: Default::default(),
        }
    }
}

impl<R: RenderResourceExtractor> Event for ExtractResource<R> {
    type Output = ();

    fn invoke(self, world: &mut ecs::world::World) -> Option<Self::Output> {
        let resource = {
            let arg = R::Arg::get(world);
            R::extract(&arg)?
        };
        world.add_resource(resource);
        Some(())
    }
}
