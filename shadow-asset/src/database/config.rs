use crate::{
    asset::AssetId,
    block::{AssetBlock, MetadataBlock},
    bytes::ToBytes,
};
use std::{
    io::{self},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct AssetDatabaseConfig {
    assets: PathBuf,
    cache: PathBuf,
    library: PathBuf,
    blocks: PathBuf,
}

impl AssetDatabaseConfig {
    pub fn new(assets: impl Into<PathBuf>, cache: impl Into<PathBuf>) -> Self {
        let assets = assets.into();
        let cache = cache.into();

        let library = cache.join("assets.lib");
        let blocks = cache.join("blocks");

        AssetDatabaseConfig {
            assets,
            cache,
            library,
            blocks,
        }
    }

    pub fn assets(&self) -> &Path {
        &self.assets
    }

    pub fn cache(&self) -> &Path {
        &self.cache
    }

    pub fn library(&self) -> &Path {
        &self.library
    }

    pub fn blocks(&self) -> &Path {
        &self.blocks
    }

    pub fn asset(&self, path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
        let path = self.assets.join(path);
        std::fs::read(path)
    }

    pub fn metadata(&self, path: impl AsRef<Path>) -> io::Result<MetadataBlock> {
        let path = path.as_ref().to_path_buf().with_extension("meta");
        std::fs::read(path).and_then(|data| {
            MetadataBlock::from_bytes(&data).ok_or(io::ErrorKind::InvalidData.into())
        })
    }

    pub fn block(&self, id: &AssetId) -> io::Result<AssetBlock> {
        let path = self.blocks.join(id.to_string());
        std::fs::read(path)
            .and_then(|data| AssetBlock::from_bytes(&data).ok_or(io::ErrorKind::InvalidData.into()))
    }
}
