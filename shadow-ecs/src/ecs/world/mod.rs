use self::events::{
    AddChildren, AddComponent, AddComponents, ComponentEvents, Despawn, RemoveChildren,
    RemoveComponent, RemoveComponents, SetParent, Spawn,
};
use super::{
    archetype::{ArchetypeId, ArchetypeMove, Archetypes},
    core::{
        Component, ComponentId, Components, Entities, Entity, LocalResource, LocalResources,
        Resource, Resources,
    },
    event::{Event, Events},
    storage::{
        dense::{DenseMap, DenseSet},
        table::ComponentSet,
    },
    system::observer::{EventObservers, IntoObserver},
    task::TaskManager,
};
use std::{any::TypeId, collections::HashSet};

pub mod events;
pub mod query;

pub struct World {
    resources: Resources,
    local_resources: LocalResources,
    components: Components,
    entities: Entities,
    archetypes: Archetypes,
    events: Events,
    observers: EventObservers,
    tasks: TaskManager,
}

impl World {
    pub fn new() -> Self {
        let mut resources = Resources::new();
        let mut events = Events::new();
        resources.add(events.register::<Spawn>());
        resources.add(events.register::<Despawn>());
        resources.add(events.register::<SetParent>());
        resources.add(events.register::<AddChildren>());
        resources.add(events.register::<RemoveChildren>());
        resources.add(events.register::<AddComponents>());
        resources.add(events.register::<RemoveComponents>());

        Self {
            resources,
            events,
            local_resources: LocalResources::new(),
            components: Components::new(),
            entities: Entities::new(),
            archetypes: Archetypes::new(),
            observers: EventObservers::new(),
            tasks: TaskManager::new(),
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

    pub fn tasks(&self) -> &TaskManager {
        &self.tasks
    }
}

impl World {
    pub fn register<C: Component>(&mut self) -> &mut Self {
        let id = self.components.register::<C>();
        self.components
            .add_extension(&id, ComponentEvents::new::<C>());
        self.register_event::<AddComponent<C>>()
            .register_event::<RemoveComponent<C>>()
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        let outputs = self.events.register::<E>();
        self.add_resource(outputs);
        self
    }

    pub fn add_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.resources.add(resource);
        self
    }

    pub fn add_local_resource<R: LocalResource>(&mut self, resource: R) -> &mut Self {
        self.local_resources.register(resource);
        self
    }

    pub fn observe<E: Event, M>(&mut self, observer: impl IntoObserver<E, M>) -> &mut Self {
        self.observers.add_observer(observer);
        self
    }
}

impl World {
    pub fn spawn(&mut self, parent: Option<Entity>) -> Entity {
        let entity = self.entities.spawn(parent.as_ref());
        self.archetypes.add_entity(&entity);
        entity
    }

    pub fn despawn(&mut self, entity: &Entity) -> DenseMap<Entity, ComponentSet> {
        let mut despawned = DenseMap::new();
        for entity in self.entities.despawn(entity) {
            if let Some((_, set)) = self.archetypes.remove_entity(&entity) {
                despawned.insert(entity, set);
            }
        }

        despawned
    }

    pub fn query(
        &self,
        components: &[ComponentId],
        exclude: &HashSet<ComponentId>,
    ) -> Vec<ArchetypeId> {
        self.archetypes.query(components, exclude)
    }

    pub fn has_component<C: Component>(&self, entity: &Entity) -> bool {
        let id = ComponentId::new::<C>();
        self.archetypes.has_component(entity, &id)
    }

    pub fn has_components(&self, entity: &Entity, components: &[ComponentId]) -> bool {
        let ids = components.into();
        self.archetypes.has_components(entity, ids)
    }

    pub fn add_component<C: Component>(
        &mut self,
        entity: &Entity,
        component: C,
    ) -> Option<ArchetypeMove> {
        let id = ComponentId::new::<C>();
        self.archetypes.add_component(entity, &id, component)
    }

    pub fn add_components(
        &mut self,
        entity: &Entity,
        components: ComponentSet,
    ) -> Option<ArchetypeMove> {
        self.archetypes.add_components(entity, components)
    }

    pub fn remove_component(
        &mut self,
        entity: &Entity,
        component: &ComponentId,
    ) -> Option<ArchetypeMove> {
        self.archetypes.remove_component(entity, component)
    }

    pub fn remove_components(
        &mut self,
        entity: &Entity,
        components: impl Into<DenseSet<ComponentId>>,
    ) -> Option<ArchetypeMove> {
        self.archetypes.remove_components(entity, components.into())
    }

    pub fn set_parent(&mut self, entity: &Entity, parent: Option<&Entity>) -> Option<Entity> {
        self.entities.set_parent(entity, parent)
    }

    pub fn flush(&mut self) {
        let mut events = self.events.drain();

        while !events.is_empty() {
            for mut event in events {
                let meta = self.events.meta_dynamic(event.ty());
                meta.invoke(&mut event, self);
            }

            self.observers.run(self);
            events = self.events.drain();
        }
    }

    pub fn flush_events<E: Event>(&mut self) {
        let mut events = self.events.remove::<E>();
        let ty = TypeId::of::<E>();
        let meta = self.events.meta_dynamic(&ty);
        while !events.is_empty() {
            for mut event in events {
                meta.invoke(&mut event, self);
            }

            self.observers.run_type::<E>(self);
            events = self.events.remove::<E>();
        }
    }
}

impl World {
    pub fn try_resource<R: Resource>(&self) -> Option<&R> {
        self.resources.try_get::<R>()
    }

    pub fn try_resource_mut<R: Resource>(&self) -> Option<&mut R> {
        self.resources.try_get_mut::<R>()
    }

    pub fn try_local_resource<R: LocalResource>(&self) -> Option<&R> {
        self.local_resources.try_get::<R>()
    }

    pub fn try_local_resource_mut<R: LocalResource>(&self) -> Option<&mut R> {
        self.local_resources.try_get_mut::<R>()
    }
}
