use shadow_asset::asset::AssetId;
use std::hash::{DefaultHasher, Hash, Hasher};

pub mod buffer;
pub mod pipeline;
pub mod shader;
pub mod texture;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct GpuResourceId(u64);

impl GpuResourceId {
    pub fn gen() -> Self {
        let id = ulid::Ulid::new();
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        Self(hasher.finish())
    }

    pub fn raw(id: u64) -> Self {
        Self(id)
    }
}

impl From<AssetId> for GpuResourceId {
    fn from(id: AssetId) -> Self {
        Self(*id)
    }
}

impl From<&AssetId> for GpuResourceId {
    fn from(id: &AssetId) -> Self {
        Self(**id)
    }
}

impl From<&str> for GpuResourceId {
    fn from(id: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl std::ops::Deref for GpuResourceId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
