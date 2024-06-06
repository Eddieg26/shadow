use crate::{
    asset::{Asset, AssetId, AssetMetadata, DefaultSettings, Settings},
    bytes::AsBytes,
};
use shadow_ecs::ecs::core::Resource;
use std::{marker::PhantomData, path::Path};

pub struct LoadContext<'a, S: Settings> {
    path: &'a Path,
    metadata: &'a AssetMetadata<S>,
    dependencies: Vec<AssetId>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(path: &'a Path, metadata: &'a AssetMetadata<S>) -> Self {
        Self {
            path,
            metadata,
            dependencies: vec![],
        }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn metadata(&self) -> &AssetMetadata<S> {
        &self.metadata
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, dependency: AssetId) -> &mut Self {
        self.dependencies.push(dependency);
        self
    }
}

pub trait AssetLoader: 'static {
    type Asset: Asset;
    type Settings: Settings;

    fn load(ctx: &mut LoadContext<Self::Settings>) -> Result<Self::Asset, std::io::Error>;
    fn extensions() -> &'static [&'static str];
}

pub trait AssetProccessor: 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Resource: Resource;

    fn process(asset: &mut Self::Asset, settings: &Self::Settings, resource: &Self::Resource);
}

pub trait AssetPostProcesser: 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Resource: Resource;

    fn process(asset: &mut Self::Asset, settings: &Self::Settings, resource: &Self::Resource);
}

pub struct DefaultProcesser<A: Asset, S: Settings> {
    _marker: PhantomData<(A, S)>,
}

impl<A: Asset, S: Settings> DefaultProcesser<A, S> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<A: Asset, S: Settings> AssetProccessor for DefaultProcesser<A, S> {
    type Asset = A;
    type Settings = S;
    type Resource = ();

    fn process(_: &mut Self::Asset, _: &Self::Settings, _: &Self::Resource) {}
}

impl<A: Asset, S: Settings> AssetPostProcesser for DefaultProcesser<A, S> {
    type Asset = A;
    type Settings = S;
    type Resource = ();

    fn process(_: &mut Self::Asset, _: &Self::Settings, _: &Self::Resource) {}
}

pub trait AssetPipeline: 'static {
    type Asset: Asset;
    type Settings: Settings;

    type Loader: AssetLoader<Asset = Self::Asset, Settings = Self::Settings>;
    type Processer: AssetProccessor<Asset = Self::Asset, Settings = Self::Settings>;
    type PostProcesser: AssetPostProcesser<Asset = Self::Asset, Settings = Self::Settings>;
}

pub struct PlainText {
    content: String,
}

impl Asset for PlainText {}

impl AsBytes for PlainText {
    fn as_bytes(&self) -> Vec<u8> {
        self.content.as_bytes().iter().copied().collect::<Vec<_>>()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self {
            content: String::from_bytes(bytes)?,
        })
    }
}

impl AssetLoader for PlainText {
    type Asset = PlainText;
    type Settings = DefaultSettings;

    fn load(ctx: &mut LoadContext<Self::Settings>) -> Result<Self::Asset, std::io::Error> {
        let bytes = std::fs::read(ctx.path)?;
        Ok(Self {
            content: String::from_bytes(&bytes).ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Data is invalid",
            ))?,
        })
    }

    fn extensions() -> &'static [&'static str] {
        todo!()
    }
}

impl AssetPipeline for PlainText {
    type Asset = PlainText;
    type Settings = DefaultSettings;
    type Loader = Self;
    type Processer = DefaultProcesser<Self::Asset, Self::Settings>;
    type PostProcesser = DefaultProcesser<Self::Asset, Self::Settings>;
}
