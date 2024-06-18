use crate::{
    asset::{Asset, AssetId, AssetMetadata, AssetPath, Settings},
    block::AssetBlock,
    bytes::ToBytes,
    errors::AssetError,
    registry::AssetPipelineRegistry,
};
use config::AssetDatabaseConfig;
use events::{ImportAsset, ImportFolder, ImportReason, LoadAsset, LoadLibrary, SaveLibrary};
use library::{AssetLibrary, AssetStatus, BlockInfo, SourceInfo};
use queue::{AssetAction, AssetQueue};
use shadow_ecs::ecs::{core::Resource, event::Events};
use std::path::Path;

pub mod config;
pub mod events;
pub mod library;
pub mod observers;
pub mod queue;

#[derive(Clone)]
pub struct AssetDatabase {
    events: Events,
    config: AssetDatabaseConfig,
    library: AssetLibrary,
    queue: AssetQueue,
}

impl AssetDatabase {
    pub fn new(config: AssetDatabaseConfig, events: &Events) -> Self {
        AssetDatabase {
            library: AssetLibrary::new(config.library()),
            events: events.clone(),
            queue: AssetQueue::new(),
            config,
        }
    }

    pub fn config(&self) -> &AssetDatabaseConfig {
        &self.config
    }

    pub fn status(&self, path: impl Into<AssetPath>) -> AssetStatus {
        self.library.status(path)
    }

    pub fn importing(&self, path: impl AsRef<Path>) -> bool {
        self.library.importing(path)
    }

    pub fn source(&self, path: &Path) -> Option<SourceInfo> {
        self.library.source(path)
    }

    pub fn block(&self, id: &AssetId) -> Option<BlockInfo> {
        self.library.block(id)
    }

    pub fn import<A: Asset>(&self, path: impl Into<AssetPath>) -> Option<()> {
        let path: AssetPath = path.into();

        let path = match path {
            AssetPath::Id(id) => self
                .block(&id)
                .and_then(|info| Some(info.filepath().to_path_buf())),
            AssetPath::Path(path) => Some(path),
        }?;

        self.events.add(ImportAsset::<A>::new(path));
        Some(())
    }

    pub fn import_folder(&self, path: impl AsRef<Path>) {
        self.events
            .add(ImportFolder::new(path.as_ref().to_path_buf()));
    }

    pub fn load<A: Asset>(&self, path: impl Into<AssetPath>) {
        self.events.add(LoadAsset::<A>::new(path));
    }

    pub fn save_lib(&self) {
        self.events.add(SaveLibrary);
    }

    pub fn load_lib(&self) {
        self.events.add(LoadLibrary)
    }

    pub fn load_metadata<S: Settings>(&self, path: impl AsRef<Path>) -> Option<AssetMetadata<S>> {
        let path = path.as_ref().to_path_buf().with_extension("meta");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|data| toml::from_str(&data).ok())
    }

    pub fn load_block(&self, id: &AssetId) -> std::io::Result<AssetBlock> {
        let path = self.config.blocks().join(id.to_string());
        std::fs::read(path).and_then(|data| {
            AssetBlock::from_bytes(&data).ok_or(std::io::ErrorKind::InvalidData.into())
        })
    }
}

impl AssetDatabase {
    fn save_metadata<A: Asset, S: Settings>(
        &self,
        path: impl AsRef<Path>,
        asset: &[u8],
        metadata: &AssetMetadata<S>,
    ) -> Result<AssetId, AssetError> {
        let data = {
            let meta_path = path.as_ref().with_extension("meta");
            let bytes = toml::to_string(metadata).map_err(|e| {
                println!("Failed to serialize metadata: {:?}", e);
                AssetError::InvalidMetadata
            })?;
            std::fs::write(meta_path, &bytes).map_err(|e| AssetError::from(e))?;
            bytes
        };

        let modified = self.config.modified(&path);
        let info = SourceInfo::from(metadata.id(), asset, data.as_bytes(), modified);
        self.library.set_source(path, info);
        Ok(metadata.id())
    }

    fn save_asset<A: Asset, S: Settings>(
        &self,
        path: impl AsRef<Path>,
        block: &AssetBlock,
        metadata: &AssetMetadata<S>,
        dependencies: &[AssetId],
    ) -> Result<AssetId, AssetError> {
        let cache_path = self.config.blocks().join(metadata.id().to_string());
        std::fs::write(&cache_path, block.to_bytes()).map_err(|e| AssetError::from(e))?;
        let info = BlockInfo::of::<A>(path.as_ref().to_path_buf());
        self.library.set_block(metadata.id(), info, dependencies);
        Ok(metadata.id())
    }

    fn import_dependents(&self, id: AssetId, registry: &AssetPipelineRegistry) {
        if let Some(dependents) = self.library.dependents().get(&id) {
            for dependent in dependents {
                if let Some(info) = self.block(dependent) {
                    if let Some(loader) = registry.meta(info.ty()) {
                        let reason = ImportReason::dependency_modified(*dependent, id);
                        loader.import(self, &self.events, reason);
                    }
                }
            }
        }
    }

    fn enqueue_action(&self, path: impl AsRef<Path>, action: AssetAction) {
        self.queue.push(path, action);
    }

    fn dequeue_action(&self, path: &Path) -> Option<AssetAction> {
        self.queue.pop(path)
    }
}

impl Resource for AssetDatabase {}
