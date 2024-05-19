use self::events::{
    AddChildren, AddComponent, AddComponents, ComponentEvents, Despawn, RemoveChildren,
    RemoveComponent, RemoveComponents, SetParent, Spawn,
};
use super::{
    archetype::Archetypes,
    core::{
        Component, ComponentId, Components, Entities, Entity, LocalResource, LocalResources,
        Resource, Resources,
    },
    event::{meta::EventMetas, Event, EventInvocations, Events},
    storage::{
        dense::{DenseMap, DenseSet},
        table::{Column, Row, Table, TableId, Tables},
    },
};

pub mod events;

pub struct World {
    resources: Resources,
    local_resources: LocalResources,
    components: Components,
    entities: Entities,
    archetypes: Archetypes,
    tables: Tables,
    events: Events,
}

impl World {
    pub fn new() -> Self {
        let mut resources = Resources::new();
        let mut event_metas = EventMetas::new();
        event_metas.register::<Spawn>();
        event_metas.register::<Despawn>();
        event_metas.register::<SetParent>();
        event_metas.register::<AddChildren>();
        event_metas.register::<RemoveChildren>();
        event_metas.register::<AddComponents>();
        event_metas.register::<RemoveComponents>();

        resources.register(event_metas);
        resources.register(EventInvocations::new());

        Self {
            resources,
            local_resources: LocalResources::new(),
            components: Components::new(),
            entities: Entities::new(),
            archetypes: Archetypes::new(),
            tables: Tables::new(),
            events: Events::new(),
        }
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.resources.get::<R>()
    }

    pub fn resource_mut<R: Resource>(&self) -> &mut R {
        self.resources.get_mut::<R>()
    }

    pub fn local_resource<R: LocalResource>(&self) -> &R {
        self.local_resources.get::<R>()
    }

    pub fn local_resource_mut<R: LocalResource>(&self) -> &mut R {
        self.local_resources.get_mut::<R>()
    }

    pub fn components(&self) -> &Components {
        &self.components
    }

    pub fn entities(&self) -> &Entities {
        &self.entities
    }

    pub fn archetypes(&self) -> &Archetypes {
        &self.archetypes
    }

    pub fn tables(&self) -> &Tables {
        &self.tables
    }

    pub fn events(&self) -> &Events {
        &self.events
    }

    pub fn events_mut(&mut self) -> &mut Events {
        &mut self.events
    }
}

impl World {
    pub fn register<C: Component>(&mut self) -> &mut Self {
        let id = self.components.register::<C>();
        self.components.add_extension(id, ComponentEvents::new::<C>());
        self.register_event::<AddComponent<C>>()
            .register_event::<RemoveComponent<C>>()
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.resource_mut::<EventMetas>().register::<E>();
        self
    }

    pub fn add_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.resources.register(resource);
        self
    }

    pub fn add_local_resource<R: LocalResource>(&mut self, resource: R) -> &mut Self {
        self.local_resources.register(resource);
        self
    }
}

impl World {
    pub fn spawn(&mut self, parent: Option<Entity>) -> Entity {
        let entity = self.entities.spawn(parent.as_ref());
        self.archetypes.add_entity(&entity);
        let table_id = self.archetypes.root_id().into();
        if let Some(table) = self.tables.get_mut(&table_id) {
            let row = Row::new();
            table.insert(entity, row);
        } else {
            let mut table = Table::new().build();
            let row = Row::new();
            table.insert(entity, row);
            self.tables.insert(table_id, table)
        }
        entity
    }

    pub fn despawn(&mut self, entity: &Entity) -> DenseMap<Entity, Row> {
        let mut despawned = DenseMap::new();
        for entity in self.entities.kill(entity) {
            if let Some(archetype) = self
                .archetypes
                .remove_entity(&entity)
                .and_then(|a| Some(a.id()))
            {
                let table_id = archetype.into();
                let table = self.tables.get_mut(&table_id).expect("Table not found");
                let row = table.remove(&entity).unwrap();
                despawned.insert(entity, row);
            }
        }

        despawned
    }

    pub fn add_components(
        &mut self,
        entity: &Entity,
        mut components: DenseMap<ComponentId, Column>,
    ) -> Option<TableId> {
        components.sort(|a, b| a.0.cmp(&b.0));
        let old = self.archetypes.entity_archetype(entity)?.id();
        let new = self.archetypes.add_components(entity, components.keys())?;
        if old != new {
            let old = old.into();
            let table_id = new.into();
            let mut row = self
                .table_mut(&old)
                .remove(entity)
                .expect("Entity not found");

            for (id, column) in components.drain() {
                row.add_column(id, column);
            }

            row.sort();
            self.table_mut(&table_id).insert(*entity, row);
            Some(table_id)
        } else {
            Some(new.into())
        }
    }

    pub fn remove_components(
        &mut self,
        entity: &Entity,
        components: &mut DenseSet<ComponentId>,
    ) -> Option<DenseMap<ComponentId, Column>> {
        components.sort();
        let old = self.archetypes.entity_archetype(entity)?.id();
        let new = self
            .archetypes
            .remove_components(entity, components.values())?;
        let mut removed = DenseMap::new();
        if old != new {
            let old = old.into();
            let table_id = new.into();
            let mut row = self
                .table_mut(&old)
                .remove(entity)
                .expect("Entity not found");

            for id in components.values() {
                if let Some(column) = row.remove_column(*id) {
                    removed.insert(*id, column);
                }
            }
            row.sort();
            self.table_mut(&table_id).insert(*entity, row);
            Some(removed)
        } else {
            None
        }
    }

    pub fn set_parent(&mut self, entity: &Entity, parent: Option<&Entity>) -> Option<Entity> {
        self.entities.set_parent(entity, parent)
    }

    fn table_mut(&mut self, id: &TableId) -> &mut Table {
        self.tables.get_mut(id).expect("Table not found")
    }
}
