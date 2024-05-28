use crate::asset::{Asset, AssetDependency, AssetId, AssetType};
use shadow_ecs::ecs::{
    core::Resource,
    storage::dense::{DenseMap, DenseSet},
};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStatus {
    Unloaded,
    Importing,
    Loading,
    Loaded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetTracker {
    id: AssetId,
    ty: AssetType,
    status: AssetStatus,
    dependencies: Vec<AssetId>,
}

impl AssetTracker {
    pub fn new<A: Asset>(id: AssetId, status: AssetStatus) -> Self {
        let ty = AssetType::of::<A>();
        Self {
            id,
            ty,
            status,
            dependencies: vec![],
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn status(&self) -> AssetStatus {
        self.status
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }

    pub fn set_status(&mut self, status: AssetStatus) {
        self.status = status;
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.push(id);
    }

    pub fn remove_dependency(&mut self, id: AssetId) {
        self.dependencies.retain(|&d| d != id);
    }

    pub fn to_dependency(&self) -> AssetDependency {
        AssetDependency::new(self.id, self.ty)
    }

    pub fn ready(&self) -> bool {
        self.dependencies.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct LoadResult {
    finished: DenseSet<AssetDependency>,
    unloaded: DenseSet<AssetDependency>,
    loading: DenseSet<AssetDependency>,
}

impl LoadResult {
    pub fn new() -> Self {
        Self {
            finished: DenseSet::new(),
            unloaded: DenseSet::new(),
            loading: DenseSet::new(),
        }
    }

    pub fn finished(&self) -> &[AssetDependency] {
        self.finished.values()
    }

    pub fn unloaded(&self) -> &[AssetDependency] {
        self.unloaded.values()
    }

    pub fn loading(&self) -> &[AssetDependency] {
        self.loading.values()
    }

    pub fn add_finished(&mut self, dependency: AssetDependency) {
        self.finished.insert(dependency);
    }

    pub fn add_unloaded(&mut self, dependency: AssetDependency) {
        self.unloaded.insert(dependency);
    }

    pub fn add_loading(&mut self, dependency: AssetDependency) {
        self.loading.insert(dependency);
    }

    pub fn is_finished(&self) -> bool {
        self.unloaded.is_empty() && self.loading.is_empty()
    }
}

type Trackers = Arc<RwLock<DenseMap<AssetId, AssetTracker>>>;
type IdMap = Arc<RwLock<DenseMap<PathBuf, AssetId>>>;

#[derive(Debug, Clone)]
pub struct AssetDependents {
    dependents: Arc<RwLock<DenseMap<AssetId, DenseSet<AssetId>>>>,
    id_map: IdMap,
}

impl AssetDependents {
    pub fn new() -> Self {
        Self {
            dependents: Arc::default(),
            id_map: IdMap::default(),
        }
    }

    pub fn add(&self, id: AssetId, dependent: AssetId) {
        let mut dependents = self.dependents.write().unwrap();
        if let Some(dependents) = dependents.get_mut(&id) {
            dependents.insert(dependent);
        } else {
            let mut set = DenseSet::new();
            set.insert(dependent);
            dependents.insert(id, set);
        }
    }

    pub fn remove(&self, id: &AssetId) -> Option<DenseSet<AssetId>> {
        self.dependents.write().unwrap().remove(id)
    }

    pub fn get(&self, id: &AssetId) -> Option<DenseSet<AssetId>> {
        self.dependents.read().unwrap().get(id).cloned()
    }
}

#[derive(Debug, Clone)]
pub struct AssetTrackers {
    trackers: Trackers,
    dependents: AssetDependents,
    load_queue: Arc<RwLock<DenseMap<AssetId, AssetType>>>,
}

impl AssetTrackers {
    pub fn new() -> Self {
        Self {
            trackers: Arc::new(RwLock::new(DenseMap::new())),
            dependents: AssetDependents::new(),
            load_queue: Arc::new(RwLock::new(DenseMap::new())),
        }
    }

    pub fn status(&self, id: &AssetId) -> AssetStatus {
        self.trackers
            .read()
            .unwrap()
            .get(id)
            .map(|tracker| tracker.status())
            .unwrap_or(AssetStatus::Unloaded)
    }

    pub(crate) fn drain_queue(&self) -> Vec<(AssetId, AssetType)> {
        self.load_queue.write().unwrap().drain().collect()
    }

    pub fn enqueue<A: Asset>(&self, id: AssetId) {
        self.load_queue
            .write()
            .unwrap()
            .insert(id, AssetType::of::<A>());
    }

    pub fn is_queued(&self, id: &AssetId) -> bool {
        self.load_queue.read().unwrap().contains(id)
    }

    pub fn dequeue(&self, id: &AssetId) {
        self.load_queue.write().unwrap().remove(id);
    }

    pub fn import<A: Asset>(&self, id: AssetId) -> Option<AssetTracker> {
        let tracker = AssetTracker::new::<A>(id, AssetStatus::Importing);
        self.trackers.write().unwrap().insert(id, tracker)
    }

    pub fn add<A: Asset>(&self, id: AssetId) -> Option<AssetTracker> {
        let tracker = AssetTracker::new::<A>(id, AssetStatus::Loading);
        self.trackers.write().unwrap().insert(id, tracker)
    }

    pub fn load(&self, id: AssetId, dependencies: &Vec<AssetDependency>) -> Option<LoadResult> {
        if self.set_status(&id, AssetStatus::Loaded) {
            let mut result = LoadResult::new();
            for dependency in dependencies {
                self.dependents.add(dependency.id(), id);
                let status = self.status(&dependency.id());
                if status == AssetStatus::Unloaded {
                    let mut trackers = self.trackers.write().unwrap();
                    trackers
                        .get_mut(&id)
                        .unwrap()
                        .add_dependency(dependency.id());
                    result.add_unloaded(*dependency);
                } else if status == AssetStatus::Loading {
                    let mut trackers = self.trackers.write().unwrap();
                    trackers
                        .get_mut(&id)
                        .unwrap()
                        .add_dependency(dependency.id());
                    result.add_loading(*dependency);
                }
            }

            if result.is_finished() {
                let mut trackers = self.trackers.write().unwrap();
                result.add_finished(trackers.get_mut(&id).unwrap().to_dependency());
                if let Some(dependents) = self.dependents.remove(&id) {
                    for dependent in dependents.iter() {
                        if let Some(dependent) = trackers.get_mut(dependent) {
                            dependent.remove_dependency(id);
                            if dependent.ready() {
                                result.add_finished(dependent.to_dependency());
                            }
                        }
                    }
                }
            }

            self.dequeue(&id);
            Some(result)
        } else {
            None
        }
    }

    pub fn fail(&self, id: &AssetId) {
        if let Some(tracker) = self.trackers.write().unwrap().get_mut(id) {
            tracker.set_status(AssetStatus::Failed);
        }
    }

    pub fn remove(&self, id: &AssetId) -> Option<AssetTracker> {
        self.dependents.remove(id);
        self.load_queue.write().unwrap().remove(id);
        self.trackers.write().unwrap().remove(id)
    }

    pub fn set_path_id(&self, path: PathBuf, id: AssetId) {
        self.dependents.id_map.write().unwrap().insert(path, id);
    }

    pub fn get_path_id(&self, path: &PathBuf) -> Option<AssetId> {
        self.dependents.id_map.read().unwrap().get(path).cloned()
    }

    fn set_status(&self, id: &AssetId, status: AssetStatus) -> bool {
        if let Some(tracker) = self.trackers.write().unwrap().get_mut(id) {
            tracker.set_status(status);
            true
        } else {
            false
        }
    }
}

impl Resource for AssetTrackers {}
