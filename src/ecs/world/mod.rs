use super::{
    archetype::Archetypes,
    core::{Component, Components, Entities, LocalResource, LocalResources, Resource, Resources},
    event::{meta::EventMetas, Event, EventInvocations, Events},
    storage::table::Tables,
};

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
    pub fn empty() -> Self {
        let mut resources = Resources::new();
        resources.register(EventMetas::new());
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
        self.components.register::<C>();
        self
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
