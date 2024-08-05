use super::{state::AssetState, AssetDatabase};
use crate::asset::{Asset, AssetId, AssetPath, AssetType, Assets};
use shadow_ecs::world::{
    event::{Event, Events},
    World,
};
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
};

pub mod import;
pub mod load;

pub trait AssetEvent: 'static {
    fn execute(&self, database: &AssetDatabase, events: &Events);
}

impl<A: AssetEvent> From<A> for Box<dyn AssetEvent> {
    fn from(event: A) -> Self {
        Box::new(event)
    }
}

pub struct AssetEvents {
    events: VecDeque<Box<dyn AssetEvent>>,
    running: bool,
}

impl AssetEvents {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            running: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn push(&mut self, event: impl AssetEvent) {
        self.events.push_back(event.into());
    }

    pub fn push_front(&mut self, event: impl AssetEvent) {
        self.events.push_front(event.into());
    }

    pub fn pop(&mut self) -> Option<Box<dyn AssetEvent>> {
        self.events.pop_front()
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}

pub struct ImportFolder {
    path: PathBuf,
}

impl ImportFolder {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

pub struct ImportAsset {
    path: PathBuf,
}

impl ImportAsset {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl Event for ImportAsset {
    type Output = PathBuf;

    fn invoke(self, _: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        Some(self.path)
    }
}

pub struct ImportAssets {
    paths: Vec<PathBuf>,
}

impl ImportAssets {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }
}

pub struct RemoveAsset {
    path: PathBuf,
}

impl RemoveAsset {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl Event for RemoveAsset {
    type Output = PathBuf;

    fn invoke(self, _: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        Some(self.path)
    }
}

pub struct RemoveAssets {
    paths: Vec<PathBuf>,
}

impl RemoveAssets {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }
}

#[derive(Clone)]
pub struct LoadAsset {
    path: AssetPath,
    load_dependencies: bool,
}

impl LoadAsset {
    pub fn new(path: impl Into<AssetPath>) -> Self {
        Self {
            path: path.into(),
            load_dependencies: true,
        }
    }

    pub fn hard(path: impl Into<AssetPath>) -> Self {
        Self {
            path: path.into(),
            load_dependencies: true,
        }
    }

    pub fn soft(path: impl Into<AssetPath>) -> Self {
        Self {
            path: path.into(),
            load_dependencies: false,
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn load_dependencies(&self) -> bool {
        self.load_dependencies
    }
}

impl Event for LoadAsset {
    type Output = Self;

    fn invoke(self, _: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct LoadAssets {
    loads: Vec<LoadAsset>,
}

impl LoadAssets {
    pub fn new(loads: Vec<LoadAsset>) -> Self {
        Self { loads }
    }

    pub fn hard(paths: impl IntoIterator<Item = impl Into<AssetPath>>) -> Self {
        Self {
            loads: paths.into_iter().map(LoadAsset::hard).collect(),
        }
    }

    pub fn soft(paths: impl IntoIterator<Item = impl Into<AssetPath>>) -> Self {
        Self {
            loads: paths.into_iter().map(LoadAsset::soft).collect(),
        }
    }

    pub fn loads(&self) -> &[LoadAsset] {
        &self.loads
    }
}

pub struct UnloadAsset {
    path: AssetPath,
}

impl UnloadAsset {
    pub fn new(path: impl Into<AssetPath>) -> Self {
        Self { path: path.into() }
    }
}

impl Event for UnloadAsset {
    type Output = ();

    fn invoke(self, world: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        let id = match self.path {
            AssetPath::Id(id) => id,
            AssetPath::Path(path) => database.library_mut().id(&path).cloned()?,
        };

        let state = database.states_mut().unload(&id)?;
        let registry = database.registry();
        let metadata = registry.get_metadata(state.ty())?;
        let event = metadata.unloaded(id, state, world)?;
        world.events().add(event);

        None
    }
}

pub struct AssetUnloaded<A: Asset> {
    id: AssetId,
    asset: A,
    state: AssetState,
}

impl<A: Asset> AssetUnloaded<A> {
    pub fn new(id: AssetId, asset: A, state: AssetState) -> Self {
        Self { id, asset, state }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn asset(&self) -> &A {
        &self.asset
    }

    pub fn state(&self) -> &AssetState {
        &self.state
    }
}

impl<A: Asset> Event for AssetUnloaded<A> {
    type Output = Self;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self)
    }
}

pub struct AssetLoaded<A: Asset> {
    id: AssetId,
    asset: A,
    state: AssetState,
}

impl<A: Asset> AssetLoaded<A> {
    pub fn new(id: AssetId, asset: A, dependencies: HashSet<AssetId>) -> Self {
        Self {
            id,
            asset,
            state: AssetState::new(AssetType::of::<A>(), dependencies),
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn asset(&self) -> &A {
        &self.asset
    }

    pub fn state(&self) -> &AssetState {
        &self.state
    }
}

impl<A: Asset> Event for AssetLoaded<A> {
    type Output = AssetId;

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource_mut::<AssetDatabase>();
        let assets = world.resource_mut::<Assets<A>>();
        let mut states = database.states_mut();

        assets.add(self.id, self.asset);
        states.load(self.id, self.state);

        Some(self.id)
    }
}
