use asset::{Asset, AssetId};
use ecs::core::Resource;
use ecs::system::{ArgItem, SystemArg};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::core::{RenderDevice, RenderQueue};

pub mod buffer;
pub mod mesh;
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ReadWrite {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderAssetUsage {
    Keep,
    Discard,
}

pub trait RenderResource: Resource {}

impl RenderResource for RenderDevice {}
impl RenderResource for RenderQueue {}

pub trait ExtractArg<'a>: SystemArg {}

impl<'a, R: RenderResource> ExtractArg<'a> for &R {}
impl<'a, R: RenderResource> ExtractArg<'a> for &mut R {}
impl<'a, A: ExtractArg<'a>, B: ExtractArg<'a>> ExtractArg<'a> for (A, B) {}
impl<'a, A: ExtractArg<'a>, B: ExtractArg<'a>, C: ExtractArg<'a>> ExtractArg<'a> for (A, B, C) {}
impl<'a, A: ExtractArg<'a>, B: ExtractArg<'a>, C: ExtractArg<'a>, D: ExtractArg<'a>> ExtractArg<'a>
    for (A, B, C, D)
{
}

pub trait RenderAsset: 'static {
    type Asset: Asset;
    type Arg<'a>: ExtractArg<'a>;
    type Error: std::error::Error;

    fn extract<'a>(
        id: &AssetId,
        asset: &mut Self::Asset,
        arg: &mut ArgItem<Self::Arg<'a>>,
    ) -> Result<RenderAssetUsage, Self::Error>;
    fn remove<'a>(id: AssetId, arg: &mut ArgItem<Self::Arg<'a>>);
}
