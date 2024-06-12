use shadow_ecs::ecs::system::{ArgItem, SystemArg};

use crate::asset::{Asset, AssetId, AssetMetadata, Settings};
use std::path::Path;

pub struct LoadContext<'a, S: Settings> {
    path: &'a Path,
    bytes: &'a [u8],
    metadata: &'a AssetMetadata<S>,
    dependencies: Vec<AssetId>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(path: &'a Path, bytes: &'a [u8], metadata: &'a AssetMetadata<S>) -> Self {
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

    pub fn metadata(&self) -> &AssetMetadata<S> {
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

pub trait AssetProcesser: 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Args: SystemArg;

    fn process(
        id: &AssetId,
        asset: &mut Self::Asset,
        settings: &Self::Settings,
        args: &ArgItem<Self::Args>,
    ) -> Result<(), std::io::Error>;
}
