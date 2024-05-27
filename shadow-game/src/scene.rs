use super::schedule::PhaseSystems;
use shadow_ecs::ecs::{core::Resource, storage::dense::DenseMap};
use std::hash::{DefaultHasher, Hash, Hasher};

pub trait Scene: 'static {
    fn systems(&self) -> PhaseSystems;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SceneId(u64);

impl SceneId {
    pub fn new<S: Scene>() -> Self {
        let type_id = std::any::TypeId::of::<S>();
        let mut hasher = DefaultHasher::new();
        type_id.hash(&mut hasher);
        Self(hasher.finish())
    }
}

pub struct Scenes {
    scenes: DenseMap<SceneId, Box<dyn Scene>>,
}

impl Scenes {
    pub fn new() -> Self {
        Self {
            scenes: DenseMap::new(),
        }
    }

    pub fn add<S: Scene>(&mut self, scene: S) {
        self.scenes.insert(SceneId::new::<S>(), Box::new(scene));
    }

    pub fn get(&self, id: SceneId) -> Option<&dyn Scene> {
        self.scenes.get(&id).map(|scene| &**scene)
    }
}

pub struct SceneTracker {
    current: Option<SceneId>,
    next: Option<SceneId>,
}

impl SceneTracker {
    pub fn new() -> Self {
        Self {
            current: None,
            next: None,
        }
    }

    pub fn set_next<S: Scene>(&mut self) {
        self.next = Some(SceneId::new::<S>());
    }

    pub fn next(&self) -> Option<SceneId> {
        self.next
    }

    pub fn swap(&mut self) {
        self.current = self.next;
        self.next = None;
    }

    pub fn current(&self) -> Option<SceneId> {
        self.current
    }
}

impl Resource for SceneTracker {}
