use ahash::AHasher;
use std::hash::{Hash, Hasher};

pub mod map;
pub mod set;

pub use map::*;
pub use set::*;

fn hash_value(value: impl Hash) -> u64 {
    let mut hasher = AHasher::default();
    value.hash(&mut hasher);

    hasher.finish()
}
