use std::path::{Path, PathBuf};

use crate::asset::AssetId;

pub struct AssetConfig {
    root: PathBuf,
    assets: PathBuf,
    cache: PathBuf,
}

impl AssetConfig {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let assets = root.join("assets");
        let cache = root.join("cache");
        Self {
            root,
            assets,
            cache,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn assets(&self) -> &Path {
        &self.assets
    }

    pub fn cache(&self) -> &Path {
        &self.cache
    }

    pub fn set_assets(&mut self, assets: impl Into<PathBuf>) {
        self.assets = assets.into();
    }

    pub fn set_cache(&mut self, cache: impl Into<PathBuf>) {
        self.cache = cache.into()
    }
}

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

    pub fn block_exists(&self, id: &AssetId) -> bool {
        self.blocks.join(id.to_string()).exists()
    }

    pub fn modified(&self, path: impl AsRef<Path>) -> u64 {
        std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}
