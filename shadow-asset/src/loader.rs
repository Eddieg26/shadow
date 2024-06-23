use crate::asset::{Asset, AssetId, AssetMetadata, BasicSettings, Settings};
use shadow_ecs::ecs::system::SystemArg;
use std::path::Path;

pub enum LoadContextType<'a> {
    Processed { bytes: &'a [u8] },
    UnProcessed { bytes: &'a [u8], path: &'a Path },
}

pub struct LoadContext<'a, S: Settings> {
    ty: LoadContextType<'a>,
    metadata: &'a mut AssetMetadata<S>,
    dependencies: Vec<AssetId>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(ty: LoadContextType<'a>, metadata: &'a mut AssetMetadata<S>) -> Self {
        LoadContext {
            ty,
            metadata,
            dependencies: Vec::new(),
        }
    }

    pub fn processed(bytes: &'a [u8], metadata: &'a mut AssetMetadata<S>) -> Self {
        Self::new(LoadContextType::Processed { bytes }, metadata)
    }

    pub fn unprocessed(
        path: &'a Path,
        bytes: &'a [u8],
        metadata: &'a mut AssetMetadata<S>,
    ) -> Self {
        Self::new(LoadContextType::UnProcessed { bytes, path }, metadata)
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

    fn load(ctx: &mut LoadContext<Self::Settings>) -> Result<Self::Asset, String>;
    fn extensions() -> &'static [&'static str];
}

pub trait AssetSaver: 'static {
    type Asset: Asset;
    type Settings: Settings;

    fn save(asset: &Self::Asset) -> &[u8];
}

pub trait AssetProcessor: 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Arg: SystemArg;

    fn process(
        asset: &mut Self::Asset,
        settings: &mut Self::Settings,
        arg: &Self::Arg,
    ) -> Result<(), String>;
}

pub trait AssetPostProcessor: 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Arg: SystemArg;

    fn post_process(
        asset: &mut Self::Asset,
        settings: &mut Self::Settings,
        arg: &Self::Arg,
    ) -> Result<(), String>;
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
    type Arg = ();

    fn process(_: &mut Self::Asset, _: &mut Self::Settings, _: &Self::Arg) -> Result<(), String> {
        Ok(())
    }
}

impl<L: AssetLoader> AssetPostProcessor for BasicProcessor<L> {
    type Asset = L::Asset;
    type Settings = L::Settings;
    type Arg = ();

    fn post_process(
        _: &mut Self::Asset,
        _: &mut Self::Settings,
        _: &Self::Arg,
    ) -> Result<(), String> {
        Ok(())
    }
}

impl AssetLoader for () {
    type Asset = ();
    type Settings = BasicSettings;

    fn load(_: &mut LoadContext<Self::Settings>) -> Result<Self::Asset, String> {
        Ok(())
    }

    fn extensions() -> &'static [&'static str] {
        &[]
    }
}

impl AssetSaver for () {
    type Asset = ();
    type Settings = BasicSettings;

    fn save(_: &Self::Asset) -> &[u8] {
        &[]
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
