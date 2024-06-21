use crate::{
    asset::{Asset, AssetId, AssetMetadata, BasicSettings, Settings},
    block::AssetBlock,
    bytes::ToBytes,
    errors::AssetError,
};
use std::path::Path;

pub enum LoadContextType<'a> {
    Processed { block: AssetBlock },
    UnProcessed { bytes: &'a [u8], path: &'a Path },
}

pub struct LoadContext<'a, S: Settings> {
    ty: LoadContextType<'a>,
    metadata: &'a mut AssetMetadata<S>,
    dependencies: Vec<AssetId>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(ty: LoadContextType, metadata: &'a mut AssetMetadata<S>) -> Self {
        LoadContext {
            ty,
            metadata,
            dependencies: Vec::new(),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.ty {
            LoadContextType::Processed { .. } => None,
            LoadContextType::UnProcessed { path, .. } => Some(path),
        }
    }

    pub fn ty(&self) -> &LoadContextType {
        &self.ty
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

    fn load(ctx: &mut LoadContext<Self::Settings>) -> Result<Self::Asset, AssetError>;
    fn extensions() -> &'static [&'static str];
}

pub trait AssetSaver: 'static {
    type Asset: Asset;
    type Settings: Settings;

    fn save(asset: &Self::Asset, metadata: &AssetMetadata<Self::Settings>) -> AssetBlock;
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
    type Saver: AssetSaver<Asset = Self::Asset, Settings = Self::Settings>;
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

    fn load(_: &mut LoadContext<Self::Settings>) -> Result<Self::Asset, AssetError> {
        Ok(())
    }

    fn extensions() -> &'static [&'static str] {
        &[]
    }
}

impl AssetSaver for () {
    type Asset = ();
    type Settings = BasicSettings;

    fn save(_: &Self::Asset, _: &AssetMetadata<Self::Settings>) -> AssetBlock {
        AssetBlock::new(&().to_bytes(), &BasicSettings, &vec![])
    }
}

impl AssetPipeline for () {
    type Asset = ();
    type Settings = BasicSettings;

    type Loader = ();
    type Saver = ();
    type Processor = BasicProcessor<()>;
    type PostProcessor = BasicProcessor<()>;
}
