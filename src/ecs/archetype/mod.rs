use super::{
    core::{ComponentId, Entity},
    storage::{
        dense::{DenseMap, DenseSet},
        table::TableId,
    },
};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub struct ArchetypeId(u64);

impl ArchetypeId {
    pub fn new(ids: &[ComponentId]) -> Self {
        let mut hasher = DefaultHasher::new();
        ids.hash(&mut hasher);
        ArchetypeId(hasher.finish())
    }

    pub fn add(ids: &[ComponentId], added: &[ComponentId]) -> (ArchetypeId, Vec<ComponentId>) {
        let mut ids = ids.iter().cloned().collect::<Vec<_>>();
        ids.extend(added.iter().cloned());
        ids.sort_unstable();
        (ArchetypeId::new(&ids), ids)
    }

    pub fn remove(ids: &[ComponentId], removed: &[ComponentId]) -> (ArchetypeId, Vec<ComponentId>) {
        let mut ids = ids.iter().cloned().collect::<Vec<_>>();
        ids.retain(|c| !removed.contains(c));
        (ArchetypeId::new(&ids), ids)
    }

    pub fn moved(
        ids: &[ComponentId],
        other: &[ComponentId],
        edge: ArchetypeEdge,
    ) -> (ArchetypeId, Vec<ComponentId>) {
        match edge {
            ArchetypeEdge::Add => ArchetypeId::add(ids, other),
            ArchetypeEdge::Remove => ArchetypeId::remove(ids, other),
        }
    }
}

impl Into<TableId> for ArchetypeId {
    fn into(self) -> TableId {
        TableId::raw(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArchetypeEdge {
    Add,
    Remove,
}

impl ArchetypeEdge {
    pub fn reverse(&self) -> Self {
        match self {
            ArchetypeEdge::Add => ArchetypeEdge::Remove,
            ArchetypeEdge::Remove => ArchetypeEdge::Add,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EdgeId {
    Component(ComponentId),
    Archetype(ArchetypeId),
}

impl EdgeId {
    pub fn new(ids: &[ComponentId]) -> Self {
        if ids.len() == 1 {
            EdgeId::Component(ids[0])
        } else {
            EdgeId::Archetype(ArchetypeId::new(ids))
        }
    }
}

pub struct Archetype {
    id: ArchetypeId,
    components: DenseSet<ComponentId>,
    entities: DenseSet<Entity>,
    add_edges: HashMap<EdgeId, ArchetypeId>,
    remove_edges: HashMap<EdgeId, ArchetypeId>,
}

impl Archetype {
    pub fn new(id: ArchetypeId, components: Vec<ComponentId>) -> Self {
        Archetype {
            id,
            components: components.as_slice().into(),
            entities: DenseSet::new(),
            add_edges: HashMap::new(),
            remove_edges: HashMap::new(),
        }
    }

    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    pub fn components(&self) -> &[ComponentId] {
        self.components.values()
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }

    pub fn edge(&self, id: &EdgeId, edge: ArchetypeEdge) -> Option<&ArchetypeId> {
        match edge {
            ArchetypeEdge::Add => self.add_edges.get(id),
            ArchetypeEdge::Remove => self.remove_edges.get(id),
        }
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
    root_id: ArchetypeId,
    entities: DenseMap<Entity, ArchetypeId>,
    archetypes: DenseMap<ArchetypeId, Archetype>,
    components: DenseMap<ComponentId, DenseSet<ArchetypeId>>,
}

impl Archetypes {
    pub fn new() -> Self {
        let root_id = ArchetypeId::new(&[]);
        let root_archetype = Archetype::new(root_id, vec![]);
        let mut archetypes = DenseMap::new();
        archetypes.insert(root_id, root_archetype);

        Archetypes {
            root_id,
            archetypes,
            entities: DenseMap::new(),
            components: DenseMap::new(),
        }
    }

    pub fn root_id(&self) -> ArchetypeId {
        self.root_id
    }

    pub fn get(&self, id: &ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(id)
    }

    pub fn query(&self, ids: &[ComponentId]) -> Vec<ArchetypeId> {
        let mut archetypes = DenseMap::new();
        for id in ids {
            if let Some(archetype_ids) = self.components.get(id) {
                for id in archetype_ids.iter() {
                    if let Some(count) = archetypes.get_mut(id) {
                        (*count) += 1;
                    } else {
                        archetypes.insert(*id, 1usize);
                    }
                }
            }
        }

        archetypes.retain(|_, count| *count >= ids.len());
        archetypes.destruct().0
    }

    pub fn entity_archetype(&self, entity: &Entity) -> Option<&Archetype> {
        let id = self.entities.get(entity).cloned()?;
        self.archetypes.get(&id)
    }

    pub fn add_entity(&mut self, entity: &Entity) {
        self.remove_entity(entity);
        let archetype = self.archetypes.get_mut(&self.root_id).unwrap();
        archetype.insert(entity.clone());
        self.entities.insert(entity.clone(), archetype.id);
    }

    pub fn remove_entity(&mut self, entity: &Entity) -> Option<&mut Archetype> {
        self.entities
            .remove(entity)
            .and_then(|id| self.archetypes.get_mut(&id))
            .and_then(|archetype| {
                archetype.remove(entity);
                Some(archetype)
            })
    }

    pub fn add_component(
        &mut self,
        entity: &Entity,
        component: ComponentId,
    ) -> Option<ArchetypeId> {
        self.move_entity(entity, &[component], ArchetypeEdge::Add)
    }

    pub fn add_components(
        &mut self,
        entity: &Entity,
        components: &[ComponentId],
    ) -> Option<ArchetypeId> {
        self.move_entity(entity, components, ArchetypeEdge::Add)
    }

    pub fn remove_component(
        &mut self,
        entity: &Entity,
        component: ComponentId,
    ) -> Option<ArchetypeId> {
        self.move_entity(entity, &[component], ArchetypeEdge::Remove)
    }

    pub fn remove_components(
        &mut self,
        entity: &Entity,
        components: &[ComponentId],
    ) -> Option<ArchetypeId> {
        self.move_entity(entity, components, ArchetypeEdge::Remove)
    }
}

impl Archetypes {
    fn add_component_archetypes(&mut self, components: &[ComponentId], archetype_id: ArchetypeId) {
        for component in components.iter() {
            if let Some(types) = self.components.get_mut(&component) {
                types.insert(archetype_id);
            } else {
                let mut types = DenseSet::new();
                types.insert(archetype_id);
                self.components.insert(*component, types);
            }
        }
    }

    fn add_archetype_edge(
        &mut self,
        entity: &Entity,
        edge_id: EdgeId,
        archetype_id: ArchetypeId,
        components: Vec<ComponentId>,
        edge: ArchetypeEdge,
    ) {
        if let Some(archetype) = self.archetypes.get_mut(&archetype_id) {
            if archetype.edge(&edge_id, edge).is_none() {
                match edge {
                    ArchetypeEdge::Add => archetype.add_edges.insert(edge_id, archetype_id),
                    ArchetypeEdge::Remove => archetype.remove_edges.insert(edge_id, archetype_id),
                };
            }
            archetype.insert(entity.clone());
        } else {
            let mut archetype = Archetype::new(archetype_id, components);
            archetype.insert(entity.clone());
            self.archetypes.insert(archetype_id, archetype);
        };
    }

    fn move_entity(
        &mut self,
        entity: &Entity,
        components: &[ComponentId],
        edge: ArchetypeEdge,
    ) -> Option<ArchetypeId> {
        let id = EdgeId::new(components);
        let (from_id, to_id) = {
            let archetype = self.remove_entity(entity)?;
            let to_id = archetype.edge(&id, edge).cloned();
            (archetype.id, to_id)
        };

        if let Some(archetype) = to_id.and_then(|id| self.archetypes.get_mut(&id)) {
            archetype.insert(entity.clone());
            self.entities.insert(entity.clone(), archetype.id);
            Some(archetype.id)
        } else {
            let arch_components = self.archetypes.get(&from_id)?.components();
            let (arch_id, components) = ArchetypeId::moved(arch_components, components, edge);
            self.add_component_archetypes(components.as_slice(), arch_id);

            let other = edge.reverse();
            self.add_archetype_edge(entity, id, arch_id, components, other);

            self.entities.insert(entity.clone(), arch_id);
            Some(arch_id)
        }
    }
}
