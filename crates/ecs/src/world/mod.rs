use self::event::{
    AddChildren, AddComponent, AddComponents, ComponentEvents, Despawn, RemoveChildren,
    RemoveComponent, RemoveComponents, SetParent, Spawn,
};
use super::{
    archetype::{ArchetypeId, ArchetypeMove, Archetypes},
    core::{
        Component, ComponentId, Components, DenseMap, DenseSet, Entities, Entity, LocalResource,
        LocalResources, Resource, Resources,
    },
    system::{
        observer::{EventObservers, IntoObserver},
        schedule::{Phase, PhaseRunner, SystemGroup, SystemTag, Systems, SystemsInfo},
        IntoSystem, RunMode,
    },
    task::{max_thread_count, TaskPool},
};
use crate::{archetype::table::EntityRow, system::schedule::Schedule};
use event::{Event, Events};
use std::collections::HashSet;

pub mod event;
pub mod query;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldKind {
    Main,
    Sub,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorldId(ulid::Ulid);

impl WorldId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }
}

pub struct World {
    id: WorldId,
    kind: WorldKind,
    systems: Option<Systems>,
    infos: SystemsInfo,
    resources: Resources,
    local_resources: LocalResources,
    components: Components,
    entities: Entities,
    archetypes: Archetypes,
    events: Events,
    observers: EventObservers,
    tasks: TaskPool,
}

impl World {
    pub fn new() -> Self {
        let (resources, events) = Self::init();

        Self {
            id: WorldId::new(),
            kind: WorldKind::Main,
            resources,
            events,
            systems: Some(Systems::new(RunMode::Parallel)),
            infos: SystemsInfo::new(),
            local_resources: LocalResources::new(),
            components: Components::new(),
            entities: Entities::new(),
            archetypes: Archetypes::new(),
            observers: EventObservers::new(),
            tasks: TaskPool::new(max_thread_count().min(3)),
        }
    }

    pub fn sub(&self) -> World {
        let (resources, events) = Self::init();

        World {
            id: WorldId::new(),
            kind: WorldKind::Sub,
            resources,
            events,
            systems: Some(Systems::new(RunMode::Sequential)),
            infos: SystemsInfo::new(),
            local_resources: LocalResources::new(),
            components: Components::new(),
            entities: Entities::new(),
            archetypes: Archetypes::new(),
            observers: EventObservers::new(),
            tasks: self.tasks.clone(),
        }
    }

    pub fn id(&self) -> WorldId {
        self.id
    }

    pub fn kind(&self) -> WorldKind {
        self.kind
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

    pub fn tasks(&self) -> &TaskPool {
        &self.tasks
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
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

    pub fn add_system<M>(&mut self, phase: impl Phase, system: impl IntoSystem<M>) -> &mut Self {
        self.systems.as_mut().unwrap().add_system(phase, system);
        self
    }

    pub fn add_phase<P: Phase>(&mut self) -> &mut Self {
        self.systems.as_mut().unwrap().add_phase::<P>();
        self
    }

    pub fn add_sub_phase<Main: Phase, Sub: Phase>(&mut self) -> &mut Self {
        self.systems.as_mut().unwrap().add_sub_phase::<Main, Sub>();
        self
    }

    pub fn insert_phase_before<Main: Phase, Before: Phase>(&mut self) -> &mut Self {
        self.systems
            .as_mut()
            .unwrap()
            .insert_phase_before::<Main, Before>();
        self
    }

    pub fn insert_phase_after<Main: Phase, After: Phase>(&mut self) -> &mut Self {
        self.systems
            .as_mut()
            .unwrap()
            .insert_phase_after::<Main, After>();
        self
    }

    pub fn add_phase_runner<P: Phase>(&mut self, runner: impl PhaseRunner) -> &mut Self {
        self.systems.as_mut().unwrap().add_phase_runner::<P>(runner);
        self
    }

    pub fn add_system_group<G: SystemGroup>(&mut self) -> &mut Self {
        self.infos.add_system_group::<G>();
        self
    }

    pub fn init_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.resources.add(R::default());
        self
    }

    pub fn add_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.resources.add(resource);
        self
    }

    pub fn init_local_resource<R: LocalResource + Default>(&mut self) -> &mut Self {
        self.local_resources.register(R::default());
        self
    }

    pub fn add_local_resource<R: LocalResource>(&mut self, resource: R) -> &mut Self {
        self.local_resources.register(resource);
        self
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.resources.remove::<R>()
    }

    pub fn remove_local_resource<R: LocalResource>(&mut self) -> Option<R> {
        self.local_resources.remove::<R>()
    }

    pub fn observe<E: Event, M>(&mut self, observer: impl IntoObserver<E, M>) -> &mut Self {
        self.observers.add_observer(observer);
        self
    }

    pub fn schedule(&self) -> Option<&Schedule> {
        self.systems.as_ref().map(|systems| systems.schedule())
    }

    pub fn build(&mut self) -> &mut Self {
        self.systems.as_mut().unwrap().build();
        self
    }
}

impl World {
    pub fn spawn(&mut self, parent: Option<Entity>) -> Entity {
        let entity = self.entities.spawn(parent.as_ref());
        self.archetypes.add_entity(&entity);
        entity
    }

    pub fn despawn(&mut self, entity: &Entity) -> DenseMap<Entity, EntityRow> {
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
        let ids = components.iter().copied().collect::<DenseSet<_>>();
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
        components: EntityRow,
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

    pub fn activate_system_group(&mut self, tag: impl Into<SystemTag>) {
        self.infos.activate(tag.into());
    }

    pub fn deactivate_system_group(&mut self, tag: impl Into<SystemTag>) {
        self.infos.deactivate(tag.into());
    }

    pub fn flush(&mut self) {
        let mut events = self.events.drain();

        while !events.is_empty() {
            for event in events {
                let meta = self.events.meta_dynamic(event.ty());
                meta.invoke(event, self);
            }

            self.observers.run(self);
            events = self.events.drain();
        }
    }

    pub fn flush_events<E: Event>(&mut self) {
        let mut events = self.events.remove::<E>();
        let meta = self.events.meta::<E>();
        while !events.is_empty() {
            for event in events {
                meta.invoke(event, self);
            }

            self.observers.run_type::<E>(self);
            events = self.events.remove::<E>();
        }
    }

    pub fn run(&mut self, phase: impl Phase) {
        let mut systems = self.systems.take().unwrap();
        let id = phase.id();

        self.infos.update(&mut systems);

        systems.run(id, self);

        self.systems = Some(systems);
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

    pub fn has_resource<R: Resource>(&self) -> bool {
        self.resources.has::<R>()
    }

    pub fn has_local_resource<R: LocalResource>(&self) -> bool {
        self.local_resources.has::<R>()
    }

    fn init() -> (Resources, Events) {
        let mut resources = Resources::new();
        let mut events = Events::new();
        resources.add(events.register::<Spawn>());
        resources.add(events.register::<Despawn>());
        resources.add(events.register::<SetParent>());
        resources.add(events.register::<AddChildren>());
        resources.add(events.register::<RemoveChildren>());
        resources.add(events.register::<AddComponents>());
        resources.add(events.register::<RemoveComponents>());

        (resources, events)
    }
}
