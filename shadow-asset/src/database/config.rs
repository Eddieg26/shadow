use crate::asset::{AssetId, AssetMetadata, Settings};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct AssetConfig {
    root: PathBuf,
    assets: PathBuf,
    cache: PathBuf,
    artifacts: PathBuf,
    sources_db: PathBuf,
    artifacts_db: PathBuf,
}

impl AssetConfig {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let assets = root.join("assets");
        let cache = root.join("cache");

        let artifacts = cache.join("artifacts");
        let sources_db = cache.join("sources.db");
        let artifacts_db = cache.join("artifacts.db");

        AssetConfig {
            root,
            assets,
            cache,
            artifacts,
            sources_db,
            artifacts_db,
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

    pub fn asset<'a>(&self, path: &'a PathBuf) -> Cow<'a, PathBuf> {
        if path.starts_with(&self.assets) {
            Cow::Borrowed(path)
        } else {
            Cow::Owned(self.assets.join(path))
        }
    }

    pub fn artifact(&self, id: &AssetId) -> PathBuf {
        self.artifacts.join(id.to_string())
    }

    pub fn metadata(&self, path: &Path) -> PathBuf {
        PathBuf::from(format!("{}.meta", path.display()))
    }

    pub fn load_metadata<S: Settings>(&self, path: &Path) -> std::io::Result<AssetMetadata<S>> {
        let meta = self.metadata(path);
        if !meta.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Metadata not found: {}", meta.display()),
            ));
        }

        let content = std::fs::read_to_string(&meta)?;

        toml::from_str(&content).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse metadata: {}", e),
            )
        })
    }

    pub fn save_metadata<S: Settings>(
        &self,
        path: &Path,
        metadata: &AssetMetadata<S>,
    ) -> std::io::Result<String> {
        let meta = self.metadata(path);
        let data = toml::to_string(metadata).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize metadata: {}", e),
            )
        })?;

        std::fs::write(meta, &data)?;

        Ok(data)
    }
}
