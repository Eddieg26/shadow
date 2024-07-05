use crate::asset::AssetId;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct AssetConfig {
    root: PathBuf,
    assets: PathBuf,
    cache: PathBuf,
    artifacts: PathBuf,
    sources_db: PathBuf,
    artifacts_db: PathBuf,
    dependency_map: PathBuf,
}

impl AssetConfig {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let assets = root.join("assets");
        let cache = root.join("cache");

        let artifacts = cache.join("artifacts");
        let sources_db = cache.join("sources.db");
        let artifacts_db = cache.join("artifacts.db");
        let dependency_map = cache.join("map");

        AssetConfig {
            root,
            assets,
            cache,
            artifacts,
            sources_db,
            artifacts_db,
            dependency_map,
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

    pub fn artifacts(&self) -> &Path {
        &self.artifacts
    }

    pub fn sources_db(&self) -> &Path {
        &self.sources_db
    }

    pub fn artifacts_db(&self) -> &Path {
        &self.artifacts_db
    }

    pub fn dependency_map(&self) -> &Path {
        &self.dependency_map
    }

    pub fn artifact(&self, id: &AssetId) -> PathBuf {
        self.artifacts.join(id.to_string())
    }

    pub fn metadata(&self, path: &Path) -> PathBuf {
        PathBuf::from(format!("{}.meta", path.display()))
    }
}
