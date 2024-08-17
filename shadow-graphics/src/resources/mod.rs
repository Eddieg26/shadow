use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

pub mod buffer;
pub mod pipeline;
pub mod shader;
pub mod texture;

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
