use crate::asset::{Asset, AssetId, AssetPath};
use config::AssetDatabaseConfig;
use events::{ImportAsset, ImportFolder, LoadAsset, LoadLibrary, SaveLibrary};
use library::{AssetLibrary, AssetStatus, BlockInfo, SourceInfo};
use shadow_ecs::ecs::{core::Resource, event::Events};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{atomic::AtomicUsize, Arc, Mutex, RwLock},
};
use task::{DatabaseState, DatabaseTasks};

pub mod config;
pub mod events;
pub mod library;
pub mod observers;
pub mod task;

#[derive(Clone)]
pub struct AssetDatabase {
    state: State,
    events: Events,
    config: AssetDatabaseConfig,
    library: AssetLibrary,
    trackers: AssetTracker,
    tasks: DatabaseTasks,
    counter: TaskCounter,
}

impl AssetDatabase {
    pub fn new(config: AssetDatabaseConfig, events: &Events) -> Self {
        let library = AssetLibrary::new(config.library());
        AssetDatabase {
            config,
            state: State::new(DatabaseState::Ready),
            events: events.clone(),
            library: library.clone(),
            trackers: AssetTracker::new(),
            tasks: DatabaseTasks::new(),
            counter: TaskCounter::new(),
        }
    }

    pub fn config(&self) -> &AssetDatabaseConfig {
        &self.config
    }

    pub fn state(&self) -> DatabaseState {
        self.state.lock().clone()
    }

    pub fn status(&self, path: impl Into<AssetPath>) -> AssetStatus {
        let path: AssetPath = path.into();
        match path {
            AssetPath::Id(id) => self.trackers.status(id),
            AssetPath::Path(path) => self
                .library
                .source(&path)
                .map(|source| self.trackers.status(source.id()))
                .unwrap_or(AssetStatus::None),
        }
    }

    pub fn source(&self, path: &Path) -> Option<SourceInfo> {
        self.library.source(path)
    }

    pub fn block(&self, id: &AssetId) -> Option<BlockInfo> {
        self.library.block(id)
    }

    pub fn import<A: Asset>(&self, path: impl Into<AssetPath>) {
        match self.state() {
            DatabaseState::Importing | DatabaseState::Ready => {
                self.events.add(ImportAsset::<A>::new(path));
                self.state.set(DatabaseState::Importing);
            }
            _ => self
                .tasks
                .add(DatabaseState::Importing, ImportAsset::<A>::new(path)),
        }
    }

    pub fn import_folder(&self, path: impl AsRef<Path>) {
        match self.state() {
            DatabaseState::Importing | DatabaseState::Ready => {
                self.events
                    .add(ImportFolder::new(path.as_ref().to_path_buf()));
                self.state.set(DatabaseState::Importing);
            }
            _ => self.tasks.add(
                DatabaseState::Importing,
                ImportFolder::new(path.as_ref().to_path_buf()),
            ),
        }
    }

    pub fn load<A: Asset>(&self, path: impl Into<AssetPath>) {
        match self.state() {
            DatabaseState::LoadingAssets | DatabaseState::Ready => {
                self.events.add(LoadAsset::<A>::new(path));
                self.state.set(DatabaseState::LoadingAssets);
            }
            _ => self
                .tasks
                .add(DatabaseState::LoadingAssets, LoadAsset::<A>::new(path)),
        }
    }

    pub fn save_lib(&self) {
        match self.state() {
            DatabaseState::Ready => {
                self.events.add(SaveLibrary);
                self.state.set(DatabaseState::Saving);
            }
            _ => self.tasks.add(DatabaseState::Saving, SaveLibrary),
        }
    }

    pub fn load_lib(&self) {
        match self.state() {
            DatabaseState::Ready => {
                self.events.add(LoadLibrary);
                self.state.set(DatabaseState::Loading);
            }
            _ => self.tasks.add(DatabaseState::Loading, LoadLibrary),
        }
    }
}

impl AssetDatabase {
    pub fn update(&self) {
        let state = self.state();
        if state != DatabaseState::Ready && self.counter.count() == 0 {
            self.set_state(DatabaseState::Ready);
        }

        if self.state() == DatabaseState::Ready {
            if let Some((state, tasks)) = self.tasks.pop() {
                match state {
                    DatabaseState::Importing | DatabaseState::LoadingAssets => {
                        tasks.iter().for_each(|task| task.run(&self.events))
                    }
                    DatabaseState::Ready | DatabaseState::Saving | DatabaseState::Loading => {
                        tasks[0].run(&self.events)
                    }
                }
            }
        }
    }

    fn set_state(&self, state: DatabaseState) {
        self.state.set(state);
    }

    fn set_status(&self, id: AssetId, status: AssetStatus) {
        self.trackers.set_status(id, status);
    }

    fn set_source(&self, path: PathBuf, info: SourceInfo) -> Option<SourceInfo> {
        self.library.add_source(path, info)
    }

    fn set_block(&self, id: AssetId, info: BlockInfo) -> Option<BlockInfo> {
        self.library.add_block(id, info)
    }

    fn counter(&self) -> &TaskCounter {
        &self.counter
    }

    fn tracker(&self) -> &AssetTracker {
        &self.trackers
    }
}

impl Resource for AssetDatabase {}

#[derive(Clone)]
pub struct State(Arc<Mutex<DatabaseState>>);

impl State {
    fn new(state: DatabaseState) -> Self {
        State(Arc::new(Mutex::new(state)))
    }

    fn lock(&self) -> std::sync::MutexGuard<DatabaseState> {
        self.0.lock().unwrap()
    }

    fn set(&self, state: DatabaseState) {
        *self.lock() = state;
    }
}

#[derive(Clone)]
pub struct AssetTracker {
    assets: Arc<RwLock<HashMap<AssetId, AssetStatus>>>,
    dependencies: Arc<RwLock<HashMap<AssetId, Vec<AssetId>>>>,
}

impl AssetTracker {
    pub fn new() -> Self {
        AssetTracker {
            assets: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn status(&self, id: AssetId) -> AssetStatus {
        self.assets
            .read()
            .unwrap()
            .get(&id)
            .copied()
            .unwrap_or(AssetStatus::None)
    }

    pub fn set_status(&self, id: AssetId, status: AssetStatus) {
        self.assets.write().unwrap().insert(id, status);
    }

    pub fn set_dependencies(&self, id: AssetId, dependencies: Vec<AssetId>) {
        self.dependencies.write().unwrap().insert(id, dependencies);
    }

    pub fn can_process(&self, id: AssetId) -> bool {
        let dependencies = self.dependencies.read().unwrap();
        if let Some(dependencies) = dependencies.get(&id) {
            dependencies.iter().all(|id| {
                matches!(
                    self.status(*id),
                    AssetStatus::Done | AssetStatus::Failed | AssetStatus::None
                )
            })
        } else {
            true
        }
    }

    pub fn is_dependencies_done(&self, id: AssetId) -> bool {
        let dependencies = self.dependencies.read().unwrap();
        if let Some(dependencies) = dependencies.get(&id) {
            dependencies
                .iter()
                .all(|id| matches!(self.status(*id), AssetStatus::Done | AssetStatus::Failed))
        } else {
            true
        }
    }
}

#[derive(Clone)]
pub struct TaskCounter(Arc<AtomicUsize>);

impl TaskCounter {
    pub fn new() -> Self {
        TaskCounter(Arc::new(AtomicUsize::new(0)))
    }

    pub fn increment(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn decrement(&self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn count(&self) -> usize {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}
