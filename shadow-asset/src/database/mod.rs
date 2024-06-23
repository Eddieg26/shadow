use crate::asset::{Asset, AssetId, AssetMetadata, AssetPath, Settings};
use config::AssetDatabaseConfig;
use events::{ImportAsset, ImportFolder, LoadAsset, LoadLibrary, SaveLibrary};
use library::{AssetLibrary, AssetStatus, BlockInfo, SourceInfo};
use queue::{AssetAction, AssetQueue};
use shadow_ecs::ecs::{core::Resource, event::Events};
use std::path::Path;

pub mod config;
pub mod events;
pub mod library;
pub mod observers;
pub mod queue;
pub mod storage;

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

    pub fn status<S: AssetStatus>(&self, id: &S::Id) -> S {
        self.library.status(id)
    }

    pub fn source(&self, path: &Path) -> Option<SourceInfo> {
        self.library.source(path)
    }

    pub fn block(&self, id: &AssetId) -> Option<BlockInfo> {
        self.library.block(id)
    }

    pub fn import<A: Asset>(&self, path: impl Into<AssetPath>) {
        let path: AssetPath = path.into();

        match path {
            AssetPath::Id(id) => match self
                .block(&id)
                .and_then(|info| Some(info.filepath().to_path_buf()))
            {
                Some(path) => self.events.add(ImportAsset::<A>::new(path)),
                None => {}
            },
            AssetPath::Path(path) => self.events.add(ImportAsset::<A>::new(path)),
        };
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
}

impl AssetDatabase {
    fn load_metadata<S: Settings>(&self, path: impl AsRef<Path>) -> Option<AssetMetadata<S>> {
        let mut path = path.as_ref().to_path_buf();
        path.extend([".meta"].iter());
        std::fs::read_to_string(path)
            .ok()
            .and_then(|data| toml::from_str(&data).ok())
    }

    fn enqueue_action(&self, path: impl AsRef<Path>, action: AssetAction) {
        self.queue.push(path, action);
    }

    fn dequeue_action<A: Asset>(&self, path: &Path) -> bool {
        match self.queue.pop(path) {
            Some(AssetAction::Import { reason }) => {
                let event = ImportAsset::<A>::with_reason(reason);
                self.events.add(event);
            }
            Some(AssetAction::ImportFolder) => {
                let event = ImportFolder::new(path);
                self.events.add(event);
            }
            Some(AssetAction::Load { id }) => {
                let event = LoadAsset::<A>::new(id);
                self.events.add(event);
            }
            _ => return false,
        }

        true
    }
}

impl Resource for AssetDatabase {}
