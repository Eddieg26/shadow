use crate::asset::AssetId;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct AssetConfig {
    root: PathBuf,
    assets: PathBuf,
    cache: PathBuf,
    library: PathBuf,
    artifacts: PathBuf,
}

impl AssetConfig {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let assets = root.join("assets");
        let cache = root.join("cache");

        let library = cache.join("library.lib");
        let artifacts = cache.join("artifacts");

        AssetConfig {
            root,
            assets,
            cache,
            library,
            artifacts,
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

    pub fn library(&self) -> &Path {
        &self.library
    }

    pub fn artifacts(&self) -> &Path {
        &self.artifacts
    }

    pub fn artifact(&self, id: &AssetId) -> PathBuf {
        self.artifacts.join(id.to_string())
    }

    pub fn metadata(&self, path: &Path) -> PathBuf {
        PathBuf::from(format!("{}.meta", path.display()))
    }
}
