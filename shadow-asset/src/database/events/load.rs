use super::{AssetEvent, StartAssetEvent};
use crate::{
    asset::{Asset, AssetId, AssetPath, Assets},
    database::{state::AssetState, AssetDatabase},
    loader::{AssetError, LoadErrorKind, LoadedAssets},
};
use shadow_ecs::{
    core::DenseSet,
    world::{
        event::{Event, Events},
        World,
    },
};
use std::collections::HashSet;

#[derive(Clone, Debug)]
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

    pub fn observer(loads: &[LoadAsset], events: &Events) {
        events.add(LoadAssets::new(loads.to_vec()))
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
    pub fn new(loads: impl IntoIterator<Item = LoadAsset>) -> Self {
        Self {
            loads: loads.into_iter().collect(),
        }
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

impl Event for LoadAssets {
    type Output = ();

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        world.events().add(StartAssetEvent::new(self));
        None
    }
}

impl AssetEvent for LoadAssets {
    fn execute(&mut self, database: &AssetDatabase, events: &Events) {
        let mut errors = vec![];
        let mut assets = LoadedAssets::new();
        let mut loaded_ids = DenseSet::new();
        let config = database.config();

        let registry = database.registry();
        for load in &self.loads {
            let id = match &load.path {
                AssetPath::Id(id) => *id,
                AssetPath::Path(path) => match database.library().id(&path).copied() {
                    Some(id) => id,
                    None => continue,
                },
            };

            if assets.contains(&id) {
                continue;
            }

            let meta = match database.config().load_artifact_meta(id) {
                Ok(artifact) => artifact,
                Err(error) => {
                    errors.push(AssetError::load(id, error));
                    continue;
                }
            };

            let loader = match registry.get_metadata(meta.ty()) {
                Some(loader) => loader,
                None => {
                    errors.push(AssetError::load(id, LoadErrorKind::NoLoader));
                    continue;
                }
            };

            let asset =
                match loader.load(id, &registry, config, &mut assets, load.load_dependencies) {
                    Ok(asset) => asset,
                    Err(error) => {
                        errors.push(error);
                        continue;
                    }
                };

            loaded_ids.insert(id);
            assets.add_erased(id, asset);
        }

        let registry = database.registry();
        let mut loaded = vec![];
        for id in loaded_ids.drain() {
            let asset = match assets.remove(&id) {
                Some(loaded) => loaded,
                _ => continue,
            };

            let metadata = match registry.get_metadata(asset.meta().ty()) {
                Some(metadata) => metadata,
                None => continue,
            };

            loaded.push(metadata.loaded(asset));
        }

        events.extend(loaded);
        events.extend(errors);
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
            AssetPath::Path(path) => database.library().id(&path).cloned()?,
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

    pub fn observer(unloaded: &[AssetUnloaded<A>], database: &AssetDatabase, events: &Events) {
        let states = database.states();
        let mut reloads = DenseSet::new();

        for unloaded in unloaded {
            let dependents = states.dependents(&unloaded.id());
            reloads.extend(dependents);
        }

        events.add(LoadAssets::soft(reloads));
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
    dependencies: HashSet<AssetId>,
}

impl<A: Asset> AssetLoaded<A> {
    pub fn new(id: AssetId, asset: A, dependencies: HashSet<AssetId>) -> Self {
        Self {
            id,
            asset,
            dependencies,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn asset(&self) -> &A {
        &self.asset
    }

    pub fn observer(loaded: &[AssetId], database: &AssetDatabase, events: &Events) {
        let states = database.states();
        let mut reloads = DenseSet::new();

        for id in loaded {
            let dependents = states.dependents(id);
            reloads.extend(dependents);
        }

        if !reloads.is_empty() {
            events.add(LoadAssets::soft(reloads));
        }
    }
}

impl<A: Asset> Event for AssetLoaded<A> {
    type Output = AssetId;

    fn invoke(self, world: &mut World) -> Option<Self::Output> {
        let database = world.resource_mut::<AssetDatabase>();
        let assets = world.resource_mut::<Assets<A>>();
        let mut states = database.states_mut();

        assets.add(self.id, self.asset);
        states.load(self.id, AssetState::new::<A>(self.dependencies));

        Some(self.id)
    }
}
