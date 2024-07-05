use config::AssetConfig;
use library::{AssetLibrary, AssetLibraryRef, AssetLibraryRefMut, AssetLibraryShared};
use shadow_ecs::ecs::core::Resource;

pub mod config;
pub mod events;
pub mod library;
pub mod observers;
pub mod registry;

pub struct AssetDatabase {
    config: AssetConfig,
    library: AssetLibraryShared,
}

impl AssetDatabase {
    pub fn new(config: AssetConfig) -> AssetDatabase {
        AssetDatabase {
            config,
            library: AssetLibraryShared::new(AssetLibrary::new()),
        }
    }

    pub fn config(&self) -> &AssetConfig {
        &self.config
    }

    pub fn library(&self) -> AssetLibraryRef {
        AssetLibraryRef::from(self.library.read().unwrap())
    }

    pub(crate) fn library_mut(&self) -> AssetLibraryRefMut {
        AssetLibraryRefMut::from(self.library.write().unwrap())
    }
}

impl Resource for AssetDatabase {}
