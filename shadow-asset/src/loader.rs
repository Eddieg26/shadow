use crate::asset::{Asset, AssetDependency, AssetMetadata, Settings};
use std::{io, path::PathBuf};

pub struct LoadContext<'a, S: Settings> {
    path: &'a PathBuf,
    metadata: &'a AssetMetadata<S>,
    dependencies: Vec<AssetDependency>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(path: &'a PathBuf, metadata: &'a AssetMetadata<S>) -> Self {
        Self {
            path,
            metadata,
            dependencies: Vec::new(),
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn metadata(&self) -> &AssetMetadata<S> {
        self.metadata
    }

    pub fn add_dependency(&mut self, id: AssetDependency) {
        self.dependencies.push(id);
    }

    pub fn dependencies(&self) -> &[AssetDependency] {
        &self.dependencies
    }
}

pub trait AssetLoader: 'static {
    type Asset: Asset;
    type Settings: Settings;

    fn load(ctx: &mut LoadContext<Self::Settings>) -> io::Result<Self::Asset>;
    fn extensions() -> &'static [&'static str];
}
