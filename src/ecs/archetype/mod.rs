use super::{
    core::{ComponentId, Entity},
    storage::{
        dense::{DenseMap, DenseSet},
        table::{Column, Row, Table},
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
    add_edges: HashMap<EdgeId, ArchetypeId>,
    remove_edges: HashMap<EdgeId, ArchetypeId>,
    table: Table,
}

impl Archetype {
    pub fn new(id: ArchetypeId, table: Table) -> Self {
        Archetype {
            id,
            components: table
                .component_ids()
                .iter()
                .copied()
                .collect::<DenseSet<_>>(),
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

    pub fn entities(&self) -> &[Entity] {
        self.table.entities()
    }

    pub fn edge(&self, id: &EdgeId, edge: ArchetypeEdge) -> Option<&ArchetypeId> {
        match edge {
            ArchetypeEdge::Add => self.add_edges.get(id),
            ArchetypeEdge::Remove => self.remove_edges.get(id),
        }
    }

    pub fn insert(&mut self, entity: Entity, row: Row) {
        self.table.insert(entity, row)
    }

    pub fn remove(&mut self, entity: &Entity) -> Option<Row> {
        self.table.remove(entity)
    }

    pub fn contains(&self, entity: &Entity) -> bool {
        self.table.contains(entity)
    }

    pub fn clear(&mut self) {
        self.table.clear();
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
        let row: Row = Row::new();
        archetype.insert(entity.clone(), row);
        self.entities.insert(entity.clone(), archetype.id);
    }

    pub fn remove_entity(&mut self, entity: &Entity) -> Option<(&mut Archetype, Row)> {
        self.entities
            .remove(entity)
            .and_then(|id| self.archetypes.get_mut(&id))
            .and_then(|archetype| {
                archetype
                    .remove(entity)
                    .and_then(|row| Some((archetype, row)))
            })
    }

    pub fn add_components(&mut self, entity: &Entity, components: Row) -> Option<RowMoveResult> {
        self.move_entity(entity, &mut RowMove::Add(components))
    }

    pub fn remove_components(
        &mut self,
        entity: &Entity,
        components: impl Into<DenseSet<ComponentId>>,
    ) -> Option<RowMoveResult> {
        self.move_entity(entity, &mut RowMove::Remove(components.into()))
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
        components: Row,
        edge: ArchetypeEdge,
    ) {
        if let Some(archetype) = self.archetypes.get_mut(&archetype_id) {
            if archetype.edge(&edge_id, edge).is_none() {
                match edge {
                    ArchetypeEdge::Add => archetype.add_edges.insert(edge_id, archetype_id),
                    ArchetypeEdge::Remove => archetype.remove_edges.insert(edge_id, archetype_id),
                };
            }
            archetype.insert(entity.clone(), components);
        } else {
            let mut table = Table::new();
            for (id, column) in components.iter() {
                table = table.with_column(*id, Column::from_column(column));
            }
            let mut archetype = Archetype::new(archetype_id, table.build());
            archetype.insert(entity.clone(), components);
            self.archetypes.insert(archetype_id, archetype);
        };
    }

    fn filter_components(archetype: &Archetype, row_move: &mut RowMove) {
        match row_move {
            RowMove::Add(_) => {}
            RowMove::Remove(components) => {
                components.retain(|id| archetype.components().contains(id));
            }
        }
    }

    fn move_entity(&mut self, entity: &Entity, row_move: &mut RowMove) -> Option<RowMoveResult> {
        let edge = row_move.edge();
        let (from_id, to_id, ids, edge_id, mut row) = {
            let (archetype, old_row) = self.remove_entity(entity)?;
            Self::filter_components(archetype, row_move);
            let ids = row_move.ids();
            if ids.len() == 0 {
                return None;
            }
            let id = EdgeId::new(ids.values());
            let to_id = archetype.edge(&id, edge).cloned();
            (archetype.id, to_id, ids, id, old_row)
        };

        if let Some(archetype) = to_id.and_then(|id| self.archetypes.get_mut(&id)) {
            let removed = match &row_move {
                RowMove::Add(_) => None,
                RowMove::Remove(components) => Some(row.take(components)),
            };
            let result = RowMoveResult::new(from_id, archetype.id(), removed);
            archetype.insert(entity.clone(), row);
            self.entities.insert(entity.clone(), archetype.id);
            Some(result)
        } else {
            let arch_components = self.archetypes.get(&from_id)?.components();
            let (arch_id, ids) = ArchetypeId::moved(arch_components, ids.values(), edge);
            self.add_component_archetypes(ids.as_slice(), arch_id);

            let removed = match &row_move {
                RowMove::Add(_) => None,
                RowMove::Remove(components) => Some(row.take(components)),
            };
            let result = RowMoveResult::new(from_id, arch_id, removed);

            let other = edge.reverse();
            self.add_archetype_edge(entity, edge_id, arch_id, row, other);

            self.entities.insert(entity.clone(), arch_id);
            Some(result)
        }
    }
}

pub enum RowMove {
    Add(Row),
    Remove(DenseSet<ComponentId>),
}

impl RowMove {
    pub fn edge(&self) -> ArchetypeEdge {
        match self {
            RowMove::Add(_) => ArchetypeEdge::Add,
            RowMove::Remove(_) => ArchetypeEdge::Remove,
        }
    }

    pub fn ids(&self) -> DenseSet<ComponentId> {
        match self {
            RowMove::Add(row) => row.keys().iter().copied().collect(),
            RowMove::Remove(ids) => ids.clone(),
        }
    }
}

pub struct RowMoveResult {
    from: ArchetypeId,
    to: ArchetypeId,
    removed_components: Option<Row>,
}

impl RowMoveResult {
    pub fn new(from: ArchetypeId, to: ArchetypeId, removed: Option<Row>) -> Self {
        RowMoveResult {
            from,
            to,
            removed_components: removed,
        }
    }

    pub fn from(&self) -> ArchetypeId {
        self.from
    }

    pub fn to(&self) -> ArchetypeId {
        self.to
    }

    pub fn removed_components(&mut self) -> Option<&mut Row> {
        self.removed_components.as_mut()
    }
}
