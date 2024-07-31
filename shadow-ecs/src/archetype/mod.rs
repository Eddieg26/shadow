use super::{
    core::{Component, ComponentId, Entity},
    storage::{
        dense::{DenseMap, DenseSet},
        table::{ComponentSet, Row, SelectedRow, Table},
    },
};
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EdgeType {
    Add,
    Remove,
}

impl EdgeType {
    pub fn reverse(&self) -> Self {
        match self {
            EdgeType::Add => EdgeType::Remove,
            EdgeType::Remove => EdgeType::Add,
        }
    }
}

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
        ty: EdgeType,
    ) -> (ArchetypeId, Vec<ComponentId>) {
        match ty {
            EdgeType::Add => ArchetypeId::add(ids, other),
            EdgeType::Remove => ArchetypeId::remove(ids, other),
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

impl From<ComponentId> for EdgeId {
    fn from(id: ComponentId) -> Self {
        EdgeId::Component(id)
    }
}

impl From<ArchetypeId> for EdgeId {
    fn from(id: ArchetypeId) -> Self {
        EdgeId::Archetype(id)
    }
}

impl From<&ComponentId> for EdgeId {
    fn from(id: &ComponentId) -> Self {
        EdgeId::Component(*id)
    }
}

impl From<&ArchetypeId> for EdgeId {
    fn from(id: &ArchetypeId) -> Self {
        EdgeId::Archetype(*id)
    }
}

impl From<&[ComponentId]> for EdgeId {
    fn from(ids: &[ComponentId]) -> Self {
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
    add_edges: HashMap<EdgeId, ArchetypeId>,
    remove_edges: HashMap<EdgeId, ArchetypeId>,
    table: Table,
}

impl Archetype {
    pub fn new(id: ArchetypeId, table: Table) -> Self {
        Archetype {
            id,
            components: table.components().iter().copied().collect::<DenseSet<_>>(),
            add_edges: HashMap::new(),
            remove_edges: HashMap::new(),
            table,
        }
    }

    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    pub fn components(&self) -> &[ComponentId] {
        self.components.values()
    }

    pub fn component<C: Component>(&self, entity: &Entity) -> Option<&C> {
        self.table.component(entity)
    }

    pub fn component_mut<C: Component>(&self, entity: &Entity) -> Option<&mut C> {
        self.table.component_mut(entity)
    }

    pub fn entities(&self) -> &[Entity] {
        self.table.entities()
    }

    pub fn edge(&self, id: impl Into<EdgeId>, ty: EdgeType) -> Option<&ArchetypeId> {
        match ty {
            EdgeType::Add => self.add_edges.get(&id.into()),
            EdgeType::Remove => self.remove_edges.get(&id.into()),
        }
    }

    pub fn insert_edge(&mut self, id: impl Into<EdgeId>, target: ArchetypeId, ty: EdgeType) {
        match ty {
            EdgeType::Add => {
                self.add_edges.insert(id.into(), target);
            }
            EdgeType::Remove => {
                self.remove_edges.insert(id.into(), target);
            }
        }
    }

    pub fn select(&self, entity: &Entity) -> Option<SelectedRow> {
        self.table.select(entity)
    }

    pub fn insert(&mut self, entity: &Entity, row: Row) {
        self.table.insert(entity, row)
    }

    pub fn remove(&mut self, entity: &Entity) -> Option<ComponentSet> {
        self.table.remove(entity)
    }

    pub fn contains(&self, entity: &Entity) -> bool {
        self.table.contains(entity)
    }

    pub fn has_component(&self, id: &ComponentId) -> bool {
        self.components.contains(id)
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
        let table = Table::new().build();
        let root_archetype = Archetype::new(root_id, table);
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

    pub fn query(&self, ids: &[ComponentId], exclude: &HashSet<ComponentId>) -> Vec<ArchetypeId> {
        let mut archetypes = DenseMap::new();
        for id in ids {
            if let Some(archetype_ids) = self.components.get(id) {
                for id in archetype_ids.iter() {
                    let archetype = self.archetypes.get(id).unwrap();
                    if archetype.entities().is_empty()
                        || exclude.iter().any(|id| archetype.has_component(id))
                    {
                        continue;
                    }

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

    pub fn add_entity(&mut self, entity: &Entity) {
        let root_id = self.root_id;
        self.entities.insert(*entity, root_id);
        self.archetypes
            .get_mut(&root_id)
            .unwrap()
            .insert(entity, Row::new().build());
    }

    pub fn remove_entity(&mut self, entity: &Entity) -> Option<(ArchetypeId, ComponentSet)> {
        self.entities.remove(entity).map(|id| {
            let archetype = self.archetypes.get_mut(&id).unwrap();
            let components = archetype.remove(entity).unwrap();
            (archetype.id(), components)
        })
    }

    pub fn has_component(&self, entity: &Entity, component: &ComponentId) -> bool {
        self.entities.get(entity).map_or(false, |id| {
            self.archetypes.get(id).unwrap().has_component(component)
        })
    }

    pub fn has_components(&self, entity: &Entity, ids: DenseSet<ComponentId>) -> bool {
        self.entities.get(entity).map_or(false, |id| {
            let archetype = self.archetypes.get(id).unwrap();
            ids.iter().all(|id| archetype.has_component(id))
        })
    }

    pub fn add_component<C: Component>(
        &mut self,
        entity: &Entity,
        id: &ComponentId,
        component: C,
    ) -> Option<ArchetypeMove> {
        let (archetype, mut components) = self.remove_entity(entity)?;
        let mut added = DenseSet::new();
        let mut removed = ComponentSet::new();
        components.insert(*id, component).map(|c| {
            removed.add_column(*id, c);
        });
        components.sort();
        added.insert(*id);

        let edge = EdgeId::from(id);
        let ty = MoveType::Add(added, removed);
        self.move_entity(entity, &archetype, &edge, components, ty)
    }

    pub fn add_components(
        &mut self,
        entity: &Entity,
        mut set: ComponentSet,
    ) -> Option<ArchetypeMove> {
        let (archetype, mut components) = self.remove_entity(entity)?;
        let mut added = DenseSet::new();
        let mut removed = ComponentSet::new();
        let mut unique = DenseSet::new();
        set.drain().for_each(|(id, column)| {
            added.insert(id);
            if components.has(&id) {
                unique.insert(id);
            }
            components.add_column(id, column).map(|c| {
                removed.add_column(id, c);
            });
        });
        components.sort();

        let edge = EdgeId::from(unique.values());
        let ty = MoveType::Add(added, removed);
        self.move_entity(entity, &archetype, &edge, components, ty)
    }

    pub fn remove_component(&mut self, entity: &Entity, id: &ComponentId) -> Option<ArchetypeMove> {
        let (archetype, mut components) = self.remove_entity(entity)?;
        let mut removed = ComponentSet::new();
        components.remove(id).map(|c| {
            removed.add_column(*id, c);
        });

        let edge = EdgeId::from(id);
        let ty = MoveType::Remove(removed);
        self.move_entity(entity, &archetype, &edge, components, ty)
    }

    pub fn remove_components(
        &mut self,
        entity: &Entity,
        ids: DenseSet<ComponentId>,
    ) -> Option<ArchetypeMove> {
        let (archetype, mut components) = self.remove_entity(entity)?;
        let mut removed = ComponentSet::new();
        ids.iter().for_each(|id| {
            components.remove(id).map(|c| {
                removed.add_column(*id, c);
            });
        });

        let edge = EdgeId::from(removed.components().keys());
        let ty = MoveType::Remove(removed);
        self.move_entity(entity, &archetype, &edge, components, ty)
    }
}

impl Archetypes {
    fn add_archetypes(&mut self, components: &[ComponentId], archetype_id: ArchetypeId) {
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

    fn next_archetype(
        &mut self,
        id: &ArchetypeId,
        edge: &EdgeId,
        ty: EdgeType,
    ) -> Option<&mut Archetype> {
        let id = self.get(id)?.edge(*edge, ty).copied()?;
        self.archetypes.get_mut(&id)
    }

    fn new_edge(
        &mut self,
        entity: &Entity,
        from: &ArchetypeId,
        edge: &EdgeId,
        components: ComponentSet,
        ty: EdgeType,
    ) -> ArchetypeId {
        let id = ArchetypeId::new(components.ids());
        let mut next = Archetype::new(id, components.layout().build());

        self.add_archetypes(components.ids(), id);
        next.insert(entity, components.into());

        let reverse = ty.reverse();
        next.insert_edge(*edge, *from, reverse);

        self.archetypes.insert(id, next);
        id
    }

    fn move_entity(
        &mut self,
        entity: &Entity,
        from: &ArchetypeId,
        edge: &EdgeId,
        components: ComponentSet,
        ty: MoveType,
    ) -> Option<ArchetypeMove> {
        let (added, removed, ty) = match ty {
            MoveType::Add(added, removed) => (added, removed, EdgeType::Add),
            MoveType::Remove(removed) => (DenseSet::new(), removed, EdgeType::Remove),
        };

        let next = if let Some(next) = self.next_archetype(from, edge, ty) {
            next.insert(entity, components.into());
            next.id()
        } else {
            let next_id = self.new_edge(entity, from, &edge, components, ty);
            self.archetypes
                .get_mut(from)?
                .insert_edge(*edge, next_id, ty);
            next_id
        };

        let _move = ArchetypeMove::new(*from, next)
            .with_removed(removed)
            .with_added(added);

        self.entities.insert(*entity, next);

        Some(_move)
    }
}

pub enum MoveType {
    Add(DenseSet<ComponentId>, ComponentSet),
    Remove(ComponentSet),
}

impl MoveType {
    pub fn edge(&self) -> EdgeType {
        match self {
            MoveType::Add(_, _) => EdgeType::Add,
            MoveType::Remove(_) => EdgeType::Remove,
        }
    }
}

pub struct ArchetypeMove {
    from: ArchetypeId,
    to: ArchetypeId,
    added: DenseSet<ComponentId>,
    removed: ComponentSet,
}

impl ArchetypeMove {
    pub fn new(from: ArchetypeId, to: ArchetypeId) -> Self {
        ArchetypeMove {
            from,
            to,
            removed: ComponentSet::new(),
            added: DenseSet::new(),
        }
    }

    pub fn with_added(mut self, added: DenseSet<ComponentId>) -> Self {
        self.added = added;
        self
    }

    pub fn with_removed(mut self, removed: ComponentSet) -> Self {
        self.removed = removed;
        self
    }

    pub fn from(&self) -> ArchetypeId {
        self.from
    }

    pub fn to(&self) -> ArchetypeId {
        self.to
    }

    pub fn removed(&self) -> &ComponentSet {
        &self.removed
    }

    pub fn removed_mut(&mut self) -> &mut ComponentSet {
        &mut self.removed
    }

    pub fn added(&self) -> &DenseSet<ComponentId> {
        &self.added
    }

    pub fn added_mut(&mut self) -> &mut DenseSet<ComponentId> {
        &mut self.added
    }
}
