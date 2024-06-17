use crate::asset::{Asset, AssetId, AssetMetadata, BasicSettings, Settings};
use std::path::Path;

pub struct LoadContext<'a, S: Settings> {
    path: &'a Path,
    bytes: &'a [u8],
    metadata: &'a mut AssetMetadata<S>,
    dependencies: Vec<AssetId>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(path: &'a Path, bytes: &'a [u8], metadata: &'a mut AssetMetadata<S>) -> Self {
        LoadContext {
            path,
            bytes,
            metadata,
            dependencies: Vec::new(),
        }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }

    pub fn metadata(&mut self) -> &mut AssetMetadata<S> {
        self.metadata
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.push(id);
    }
}

pub trait AssetLoader: 'static {
    type Asset: Asset;
    type Settings: Settings;

    fn load(ctx: &mut LoadContext<Self::Settings>) -> Self::Asset;
    fn extensions() -> &'static [&'static str];
}

pub trait AssetProcessor: 'static {
    type Asset: Asset;
    type Settings: Settings;

    fn process(asset: &mut Self::Asset, metadata: &mut AssetMetadata<Self::Settings>);
}

pub trait AssetPostProcessor: 'static {
    type Asset: Asset;
    type Settings: Settings;

    fn post_process(asset: &mut Self::Asset, metadata: &mut AssetMetadata<Self::Settings>);
}

pub trait AssetPipeline: 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Loader: AssetLoader<Asset = Self::Asset, Settings = Self::Settings>;
    type Processor: AssetProcessor<Asset = Self::Asset, Settings = Self::Settings>;
    type PostProcessor: AssetPostProcessor<Asset = Self::Asset, Settings = Self::Settings>;
}

pub struct BasicProcessor<L: AssetLoader>(std::marker::PhantomData<L>);

impl<L: AssetLoader> AssetProcessor for BasicProcessor<L> {
    type Asset = L::Asset;
    type Settings = L::Settings;

    fn process(_: &mut Self::Asset, _: &mut AssetMetadata<Self::Settings>) {}
}

impl<L: AssetLoader> AssetPostProcessor for BasicProcessor<L> {
    type Asset = L::Asset;
    type Settings = L::Settings;

    fn post_process(_: &mut Self::Asset, _: &mut AssetMetadata<Self::Settings>) {}
}

impl AssetLoader for () {
    type Asset = ();
    type Settings = BasicSettings;

    fn load(_: &mut LoadContext<Self::Settings>) -> Self::Asset {
        ()
    }

    fn extensions() -> &'static [&'static str] {
        &[]
    }
}

impl AssetPipeline for () {
    type Asset = ();
    type Settings = BasicSettings;

    type Loader = ();
    type Processor = BasicProcessor<()>;
    type PostProcessor = BasicProcessor<()>;
}
