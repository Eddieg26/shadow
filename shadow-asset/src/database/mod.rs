use crate::asset::{AssetId, AssetPath};
use config::AssetDatabaseConfig;
use library::{AssetStatus, BlockInfo, SourceInfo};
use shadow_ecs::ecs::core::Resource;
use state::DatabaseState;
use std::path::{Path, PathBuf};

pub mod config;
pub mod events;
pub mod library;
pub mod queue;
pub mod state;
pub mod task;

pub struct AssetDatabase {
    config: AssetDatabaseConfig,
    state: DatabaseState,
}

impl AssetDatabase {
    pub fn new(config: AssetDatabaseConfig) -> Self {
        AssetDatabase {
            config,
            state: DatabaseState::new(),
        }
    }

    pub fn config(&self) -> &AssetDatabaseConfig {
        &self.config
    }

    pub fn state(&self) -> &DatabaseState {
        &self.state
    }

    pub fn status(&self, path: impl Into<AssetPath>) -> AssetStatus {
        self.state.status(path)
    }

    pub fn source(&self, path: &Path) -> Option<SourceInfo> {
        self.state.library().source(path)
    }

    pub fn block(&self, id: &AssetId) -> Option<BlockInfo> {
        self.state.library().block(id)
    }
}

impl AssetDatabase {
    pub(crate) fn add_source(&self, path: PathBuf, info: SourceInfo) -> Option<SourceInfo> {
        self.state.library().add_source(path, info)
    }

    pub(crate) fn add_block(&self, id: AssetId, info: BlockInfo) -> Option<BlockInfo> {
        self.state.library().add_block(id, info)
    }
}

impl Resource for AssetDatabase {}
