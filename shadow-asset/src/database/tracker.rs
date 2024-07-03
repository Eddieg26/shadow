use crate::asset::AssetId;
use shadow_ecs::ecs::storage::dense::DenseMap;
use std::{
    hash::Hash,
    path::PathBuf,
    sync::{Arc, RwLock},
};

pub trait AssetStatus: Sized + 'static {
    type Id: Clone + PartialEq + Hash;

    fn get(id: &Self::Id, tracker: &AssetTracker<Self>) -> Self;
    fn set(&self, id: Self::Id, tracker: &AssetTracker<Self>) -> Option<Self>;
    fn clear(tracker: &AssetTracker<Self>);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoadStatus {
    None,
    Loading,
    Done,
    Failed,
}

impl AssetStatus for LoadStatus {
    type Id = AssetId;

    fn get(id: &Self::Id, tracker: &AssetTracker<Self>) -> Self {
        if let Some(status) = tracker.status.read().unwrap().get(id) {
            *status
        } else {
            LoadStatus::None
        }
    }

    fn set(&self, id: Self::Id, tracker: &AssetTracker<Self>) -> Option<Self> {
        tracker.status.write().unwrap().insert(id, *self)
    }

    fn clear(tracker: &AssetTracker<Self>) {
        tracker.status.write().unwrap().clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImportStatus {
    None,
    Importing,
    Done,
    Failed,
}

impl AssetStatus for ImportStatus {
    type Id = PathBuf;

    fn get(id: &Self::Id, tracker: &AssetTracker<Self>) -> Self {
        if let Some(status) = tracker.status.read().unwrap().get(id) {
            *status
        } else {
            ImportStatus::None
        }
    }

    fn set(&self, id: Self::Id, tracker: &AssetTracker<Self>) -> Option<Self> {
        tracker.status.write().unwrap().insert(id, *self)
    }

    fn clear(tracker: &AssetTracker<Self>) {
        tracker.status.write().unwrap().clear();
    }
}

#[derive(Clone, Debug)]
pub struct AssetTracker<S: AssetStatus> {
    status: Arc<RwLock<DenseMap<S::Id, S>>>,
}

impl<S: AssetStatus> AssetTracker<S> {
    pub fn new() -> Self {
        AssetTracker {
            status: Arc::default(),
        }
    }

    pub fn status(&self, id: &S::Id) -> S {
        S::get(id, self)
    }

    pub fn set_status(&self, id: S::Id, status: S) -> Option<S> {
        status.set(id, self)
    }

    pub fn clear(&self) {
        S::clear(self)
    }
}
