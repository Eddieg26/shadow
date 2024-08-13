use shadow_asset::asset::AssetId;
use shadow_ecs::core::DenseMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use texture::{GpuTexture, Texture};

pub mod buffer;
pub mod pipeline;
pub mod shader;
pub mod texture;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct GpuResourceId(u64);

impl GpuResourceId {
    pub fn new() -> Self {
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

pub struct GpuResources {
    textures: DenseMap<GpuResourceId, GpuTexture>,
}

impl GpuResources {
    pub fn new() -> Self {
        Self {
            textures: DenseMap::new(),
        }
    }

    pub fn texture(&self, id: impl Into<GpuResourceId>) -> Option<&GpuTexture> {
        self.textures.get(&id.into())
    }

    pub fn add_texture<T: Texture>(
        &mut self,
        device: &wgpu::Device,
        id: impl Into<GpuResourceId>,
        texture: &T,
    ) {
        let id = id.into();
        let texture = GpuTexture::create(device, texture);
        self.textures.insert(id, texture);
    }

    pub fn remove_texture(&mut self, id: impl Into<GpuResourceId>) -> Option<GpuTexture> {
        self.textures.remove(&id.into())
    }
}
