use super::{
    core::{ComponentId, Entity},
    storage::dense::DenseSet,
};
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub struct ArchetypeId(u64);

impl ArchetypeId {
    pub fn new(ids: &[ComponentId]) -> Self {
        let mut hasher = DefaultHasher::new();
        ids.hash(&mut hasher);
        ArchetypeId(hasher.finish())
    }
}

pub struct Archetype {
    id: ArchetypeId,
    components: Vec<ComponentId>,
    entities: DenseSet<Entity>,
}

impl Archetype {
    pub fn new() -> Self {
        Archetype {
            id: ArchetypeId(0),
            components: vec![],
            entities: DenseSet::new(),
        }
    }

    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    pub fn components(&self) -> &[ComponentId] {
        &self.components
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }

    pub fn insert(&mut self, entity: Entity) -> bool {
        self.entities.insert(entity)
    }

    pub fn remove(&mut self, entity: &Entity) -> bool {
        self.entities.swap_remove(entity)
    }

    pub fn contains(&self, entity: &Entity) -> bool {
        self.entities.contains(entity)
    }

    pub fn clear(&mut self) {
        self.entities.clear();
    }
}

pub struct Archetypes {
    entities: HashMap<Entity, ArchetypeId>,
    archetypes: HashMap<ArchetypeId, Archetype>,
    components: HashMap<ComponentId, DenseSet<ArchetypeId>>,
}
