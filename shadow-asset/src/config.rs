use crate::{
    asset::{AssetId, AssetInfo, AssetMetadata, Settings},
    bytes::AsBytes,
};
use shadow_ecs::ecs::core::Resource;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct AssetConfig {
    assets: PathBuf,
    cache: PathBuf,
}

impl AssetConfig {
    pub fn new(assets: impl Into<PathBuf>, cache: impl Into<PathBuf>) -> Self {
        Self {
            assets: assets.into(),
            cache: cache.into(),
        }
    }

    pub fn assets(&self) -> &PathBuf {
        &self.assets
    }

    pub fn cache(&self) -> &PathBuf {
        &self.cache
    }

    pub fn meta_path(&self, asset_path: &Path) -> PathBuf {
        let mut meta_path = asset_path.to_path_buf().into_os_string();
        meta_path.push(".meta");
        PathBuf::from(&meta_path)
    }

    pub fn cached_asset_path(&self, id: &AssetId) -> PathBuf {
        self.cache.join(id.to_string())
    }

    pub fn asset_info_path(&self, asset_path: &Path) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        asset_path.hash(&mut hasher);
        let hash = hasher.finish().to_string();
        let mut info_path = self.cache.join("lib");
        info_path.push(hash);
        info_path
    }

    pub fn save_metadata<S: Settings>(
        &self,
        asset_path: &Path,
        metadata: &AssetMetadata<S>,
    ) -> io::Result<PathBuf> {
        let meta_path = self.meta_path(asset_path);
        if let Some(parent) = meta_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = metadata.as_bytes();
        std::fs::write(&meta_path, &bytes)?;
        Ok(meta_path)
    }

    pub fn save_asset_info(&self, asset_path: &Path, info: &AssetInfo) -> io::Result<PathBuf> {
        let info_path = self.asset_info_path(asset_path);
        if let Some(parent) = info_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = info.as_bytes();
        std::fs::write(&info_path, &bytes)?;
        Ok(info_path)
    }

    pub fn load_asset_info(&self, asset_path: &Path) -> io::Result<AssetInfo> {
        let info_path = self.asset_info_path(asset_path);
        let bytes = std::fs::read(&info_path)?;
        let info = AssetInfo::from_bytes(&bytes).ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Failed to load asset info",
        ))?;
        Ok(info)
    }
}

impl Resource for AssetConfig {}
