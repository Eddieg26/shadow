use self::events::{
    AddChildren, AddComponent, AddComponents, ComponentEvents, Despawn, RemoveChildren,
    RemoveComponent, RemoveComponents, SetParent, Spawn,
};
use super::{
    archetype::{Archetypes, RowMoveResult},
    core::{
        Component, ComponentId, Components, Entities, Entity, LocalResource, LocalResources,
        Resource, Resources,
    },
    event::{meta::EventMetas, Event, EventInvocations, Events},
    storage::{
        dense::{DenseMap, DenseSet},
        table::{Column, Row},
    },
};

pub mod events;

pub struct World {
    resources: Resources,
    local_resources: LocalResources,
    components: Components,
    entities: Entities,
    archetypes: Archetypes,
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
        self.components
            .add_extension(id, ComponentEvents::new::<C>());
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
        entity
    }

    pub fn despawn(&mut self, entity: &Entity) -> DenseMap<Entity, Row> {
        let mut despawned = DenseMap::new();
        for entity in self.entities.kill(entity) {
            if let Some((_, row)) = self.archetypes.remove_entity(&entity) {
                despawned.insert(entity, row);
            }
        }

        despawned
    }

    pub fn add_components(
        &mut self,
        entity: &Entity,
        mut components: DenseMap<ComponentId, Column>,
    ) -> Option<RowMoveResult> {
        components.sort(|a, b| a.0.cmp(&b.0));
        self.archetypes
            .add_components(entity, Row::with_columns(components))
    }

    pub fn remove_components(
        &mut self,
        entity: &Entity,
        components: impl Into<DenseSet<ComponentId>>,
    ) -> Option<RowMoveResult> {
        let mut components = components.into();
        components.sort();
        self.archetypes
            .remove_components(entity, components.clone())
    }

    pub fn set_parent(&mut self, entity: &Entity, parent: Option<&Entity>) -> Option<Entity> {
        self.entities.set_parent(entity, parent)
    }
}
