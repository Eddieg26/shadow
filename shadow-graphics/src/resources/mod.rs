use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub mod buffer;
pub mod material;
pub mod pipeline;
pub mod shader;
pub mod texture;
pub mod mesh;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(u64);

impl ResourceId {
    pub fn gen() -> Self {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(0);
        Self(hasher.finish())
    }
}

impl From<&str> for ResourceId {
    fn from(name: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl From<String> for ResourceId {
    fn from(name: String) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        Self(hasher.finish())
    }
}
