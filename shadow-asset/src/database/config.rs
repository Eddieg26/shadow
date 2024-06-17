use shadow_ecs::ecs::core::Resource;
use std::path::{Path, PathBuf};

pub struct AssetConfig {
    assets: PathBuf,
    cache: PathBuf,
}

impl AssetConfig {
    pub fn new() -> Self {
        Self {
            assets: PathBuf::from("assets"),
            cache: PathBuf::from("cache"),
        }
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

impl Resource for AssetConfig {}

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

    pub fn normalize(path: impl AsRef<Path>) -> PathBuf {
        let mut path = path.as_ref().to_path_buf();
        path.as_mut_os_string()
            .to_str()
            .map(|path| path.replace("\\", "/"))
            .map(PathBuf::from)
            .unwrap_or(path)
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
