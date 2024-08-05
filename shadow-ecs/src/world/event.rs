use super::World;
use crate::core::{internal::blob::BlobCell, DenseSet, Resource};
use std::{
    any::TypeId,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex, RwLock},
};

pub use internal::*;

pub trait Event: Send + Sync + 'static {
    type Output: Send + Sync + 'static;
    const PRIORITY: i32 = 0;

    fn priority(&self) -> i32 {
        Self::PRIORITY
    }

    fn invoke(self, world: &mut World) -> Option<Self::Output>;
}

pub type EventType = TypeId;

pub struct ErasedEvent {
    ty: EventType,
    event: BlobCell,
}

impl ErasedEvent {
    pub fn new<E: Event>(event: E) -> Self {
        Self {
            ty: TypeId::of::<E>(),
            event: BlobCell::new(event),
        }
    }

    pub fn ty(&self) -> &EventType {
        &self.ty
    }

    pub fn cast<E: Event>(&self) -> Option<&E> {
        (self.ty == TypeId::of::<E>()).then_some(self.event.value::<E>())
    }

    pub fn cast_mut<E: Event>(&mut self) -> Option<&mut E> {
        (self.ty == TypeId::of::<E>()).then_some(self.event.value_mut())
    }

    pub fn take<E: Event>(self) -> E {
        self.event.take::<E>()
    }
}

impl<E: Event> From<E> for ErasedEvent {
    fn from(event: E) -> Self {
        Self::new(event)
    }
}

pub struct EventMeta {
    priority: i32,
    invoke: Box<dyn Fn(ErasedEvent, &mut World) + Send + Sync>,
    clear: Box<dyn Fn(&World) + Send + Sync>,
}

impl EventMeta {
    pub fn new<E: Event>() -> Self {
        Self {
            priority: E::PRIORITY,
            invoke: Box::new(|event, world| {
                let event = event.take::<E>();
                if let Some(output) = event.invoke(world) {
                    world.events().invoked::<E>();
                    world.resource_mut::<EventOutputs<E>>().add(output);
                }
            }),
            clear: Box::new(|world| {
                world.resource_mut::<EventOutputs<E>>().clear();
            }),
        }
    }

    pub fn priority(&self) -> i32 {
        self.priority
    }

    pub fn invoke(&self, event: ErasedEvent, world: &mut World) {
        (self.invoke)(event, world)
    }

    pub fn clear(&self, world: &World) {
        (self.clear)(world)
    }
}

#[derive(Clone)]
pub struct Events {
    events: Arc<Mutex<Vec<ErasedEvent>>>,
    metas: HashMap<EventType, Arc<EventMeta>>,
    invocations: Arc<RwLock<DenseSet<EventInvocation>>>,
}

impl Events {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            metas: HashMap::new(),
            invocations: Arc::new(RwLock::new(DenseSet::new())),
        }
    }

    pub fn register<E: Event>(&mut self) -> EventOutputs<E> {
        let meta = Arc::new(EventMeta::new::<E>());
        self.metas.insert(TypeId::of::<E>(), meta);
        EventOutputs::<E>::new()
    }

    pub fn meta<E: Event>(&self) -> Arc<EventMeta> {
        let ty = TypeId::of::<E>();
        let meta = self.metas.get(&ty).expect("Event not registered");
        meta.clone()
    }

    pub fn meta_dynamic(&self, ty: &EventType) -> Arc<EventMeta> {
        let meta = self.metas.get(ty).expect("Event not registered");
        meta.clone()
    }

    pub fn add(&self, event: impl Into<ErasedEvent>) {
        let mut events = self.events.lock().unwrap();
        events.push(event.into());
    }

    pub fn extend(&self, events: Vec<impl Into<ErasedEvent>>) {
        let mut _events = self.events.lock().unwrap();
        _events.extend(events.into_iter().map(|e| e.into()));
    }

    pub fn remove<E: Event>(&self) -> Vec<ErasedEvent> {
        let mut events = self.events.lock().unwrap();
        let mut drained = Vec::new();
        let mut index = 0;
        while index < events.len() {
            if events[index].ty == TypeId::of::<E>() {
                drained.push(events.remove(index));
            } else {
                index += 1;
            }
        }

        drained
    }

    pub fn drain(&self) -> Vec<ErasedEvent> {
        let mut events = self.events.lock().unwrap();
        events.drain(..).collect::<Vec<_>>()
    }

    pub(crate) fn invoked<E: Event>(&self) {
        let mut invocations = self.invocations.write().unwrap();
        invocations.insert(EventInvocation::new::<E>());
    }

    pub(crate) fn invocations(&self) -> Vec<EventInvocation> {
        let mut invocations = self.invocations.write().unwrap();
        invocations.sort();
        invocations.drain().collect::<Vec<_>>()
    }

    pub(crate) fn invocation_type<E: Event>(&self) -> Option<EventInvocation> {
        let mut invocations = self.invocations.write().unwrap();
        let invocation = EventInvocation::new::<E>();
        invocations.remove(&invocation).map(|_| invocation)
    }

    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        events.clear();
    }

    pub fn is_empty(&self) -> bool {
        let events = self.events.lock().unwrap();
        events.is_empty()
    }

    pub fn len(&self) -> usize {
        let events = self.events.lock().unwrap();
        events.len()
    }
}

pub struct EventOutputs<E: Event> {
    outputs: Vec<E::Output>,
}

impl<E: Event> EventOutputs<E> {
    pub fn new() -> Self {
        Self {
            outputs: Vec::new(),
        }
    }

    pub fn add(&mut self, output: E::Output) {
        self.outputs.push(output);
    }

    pub fn extend(&mut self, outputs: Vec<E::Output>) {
        self.outputs.extend(outputs);
    }

    pub fn drain(&mut self) -> Vec<E::Output> {
        self.outputs.drain(..).collect::<Vec<_>>()
    }

    pub fn slice(&self) -> &[E::Output] {
        &self.outputs
    }

    pub fn swap(&mut self, world: &mut World) {
        let outputs = world.resource_mut::<EventOutputs<E>>();
        std::mem::swap(&mut outputs.outputs, &mut self.outputs);
    }

    pub fn clear(&mut self) {
        self.outputs.clear();
    }

    pub fn len(&self) -> usize {
        self.outputs.len()
    }
}

impl<E: Event> Resource for EventOutputs<E> {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord)]
pub struct EventInvocation {
    event: EventType,
    priority: i32,
}

impl EventInvocation {
    pub fn new<E: Event>() -> Self {
        Self {
            event: TypeId::of::<E>(),
            priority: E::PRIORITY,
        }
    }

    pub fn priority(&self) -> i32 {
        self.priority
    }

    pub fn event(&self) -> EventType {
        self.event
    }
}

impl PartialOrd for EventInvocation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}

impl Hash for EventInvocation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.event.hash(state);
    }
}

pub mod internal {
    use super::{Event, EventOutputs, World};
    use crate::{
        archetype::table::EntityRow,
        core::{ColumnCell, Component, ComponentId, DenseSet, Entity},
        system::schedule::SystemTag,
    };
    pub struct Spawn {
        parent: Option<Entity>,
        components: EntityRow,
    }

    impl Spawn {
        pub fn new() -> Self {
            Self {
                parent: None,
                components: EntityRow::new(),
            }
        }

        pub fn set_parent(mut self, parent: Entity) -> Self {
            self.parent = Some(parent);
            self
        }

        pub fn with<C: Component>(mut self, component: C) -> Self {
            self.components.add_component(component);
            self
        }
    }

    impl Event for Spawn {
        type Output = Entity;
        const PRIORITY: i32 = i32::MAX - 1000;

        fn invoke(self, world: &mut super::World) -> Option<Self::Output> {
            let entity = world.spawn(self.parent);
            if matches!(self.parent, Some(_)) {
                world.events().add(SetParent::new(entity, self.parent));
            }

            if let Some(result) = world.add_components(&entity, self.components) {
                for added in result.added().iter() {
                    let meta = world.components().extension::<ComponentEvents>(&added);
                    meta.add(world, &entity);
                }
            }

            Some(entity)
        }
    }

    pub struct Despawn {
        entity: Entity,
    }

    impl Despawn {
        pub fn new(entity: Entity) -> Self {
            Self { entity }
        }
    }

    impl Event for Despawn {
        type Output = Vec<Entity>;
        const PRIORITY: i32 = i32::MIN + 1000;

        fn invoke(self, world: &mut super::World) -> Option<Self::Output> {
            let mut entities = vec![];
            for (entity, mut components) in world.despawn(&self.entity).drain() {
                entities.push(entity);
                for (id, cell) in components.drain() {
                    let meta = world.components().extension::<ComponentEvents>(&id);
                    meta.remove(world, &entity, cell);
                }
            }

            (!entities.is_empty()).then_some(entities)
        }
    }

    pub struct ParentUpdate {
        entity: Entity,
        parent: Option<Entity>,
        old_parent: Option<Entity>,
    }

    impl ParentUpdate {
        fn new(entity: Entity, parent: Option<Entity>, old_parent: Option<Entity>) -> Self {
            Self {
                entity,
                parent,
                old_parent,
            }
        }

        pub fn entity(&self) -> Entity {
            self.entity
        }

        pub fn parent(&self) -> Option<Entity> {
            self.parent
        }

        pub fn old_parent(&self) -> Option<Entity> {
            self.old_parent
        }
    }

    pub struct SetParent {
        entity: Entity,
        parent: Option<Entity>,
    }

    impl SetParent {
        pub fn new(entity: Entity, parent: Option<Entity>) -> Self {
            Self { entity, parent }
        }

        pub fn none(entity: Entity) -> Self {
            Self {
                entity,
                parent: None,
            }
        }
    }

    impl Event for SetParent {
        type Output = ParentUpdate;
        const PRIORITY: i32 = Spawn::PRIORITY - 1000;

        fn invoke(self, world: &mut super::World) -> Option<Self::Output> {
            let old_parent = world.set_parent(&self.entity, self.parent.as_ref());
            Some(ParentUpdate::new(self.entity, self.parent, old_parent))
        }
    }

    pub struct AddChildren {
        parent: Entity,
        children: Vec<Entity>,
    }

    impl AddChildren {
        pub fn new(parent: Entity, children: Vec<Entity>) -> Self {
            Self { parent, children }
        }
    }

    impl Event for AddChildren {
        type Output = Vec<ParentUpdate>;
        const PRIORITY: i32 = SetParent::PRIORITY - 1000;

        fn invoke(self, world: &mut super::World) -> Option<Self::Output> {
            let updates = self
                .children
                .iter()
                .map(|child| {
                    let old_parent = world.set_parent(child, Some(&self.parent));
                    ParentUpdate::new(*child, Some(self.parent), old_parent)
                })
                .collect::<Vec<_>>();

            Some(updates)
        }
    }

    pub struct RemoveChildren {
        parent: Entity,
        children: Vec<Entity>,
    }

    impl RemoveChildren {
        pub fn new(parent: Entity, children: Vec<Entity>) -> Self {
            Self { parent, children }
        }

        pub fn parent(&self) -> Entity {
            self.parent
        }

        pub fn children(&self) -> &[Entity] {
            &self.children
        }
    }

    impl Event for RemoveChildren {
        type Output = Vec<ParentUpdate>;
        const PRIORITY: i32 = AddChildren::PRIORITY - 1000;

        fn invoke(self, world: &mut super::World) -> Option<Self::Output> {
            let updates = self
                .children
                .iter()
                .map(|child| {
                    let old_parent = world.set_parent(child, None);
                    ParentUpdate::new(*child, None, old_parent)
                })
                .collect::<Vec<_>>();

            Some(updates)
        }
    }

    pub struct AddComponent<C: Component> {
        entity: Entity,
        component: Option<C>,
    }

    impl<C: Component> AddComponent<C> {
        pub fn new(entity: Entity, component: C) -> Self {
            Self {
                entity,
                component: Some(component),
            }
        }
    }

    impl<C: Component> Event for AddComponent<C> {
        type Output = Entity;
        const PRIORITY: i32 = Spawn::PRIORITY - 1000;

        fn invoke(mut self, world: &mut super::World) -> Option<Self::Output> {
            let component = self.component.take()?;
            world.add_component(&self.entity, component)?;

            let id = ComponentId::new::<C>();
            let meta = world.components().extension::<ComponentEvents>(&id);
            meta.add(world, &self.entity);

            Some(self.entity)
        }
    }

    pub struct AddComponents {
        entity: Entity,
        components: EntityRow,
    }

    impl AddComponents {
        pub fn new(entity: Entity) -> Self {
            Self {
                entity,
                components: EntityRow::new(),
            }
        }

        pub fn with<C: Component>(mut self, component: C) -> Self {
            self.components.add_component(component);

            self
        }
    }

    impl Event for AddComponents {
        type Output = Entity;
        const PRIORITY: i32 = AddComponent::<()>::PRIORITY;

        fn invoke(self, world: &mut super::World) -> Option<Self::Output> {
            let result = world.add_components(&self.entity, self.components)?;

            for added in result.added().iter() {
                let meta = world.components().extension::<ComponentEvents>(&added);
                meta.add(world, &self.entity);
            }

            Some(self.entity)
        }
    }

    pub struct RemovedComponent<C: Component> {
        pub entity: Entity,
        pub component: C,
    }

    impl<C: Component> RemovedComponent<C> {
        pub fn new(entity: Entity, component: C) -> Self {
            Self { entity, component }
        }
    }

    pub struct RemoveComponent<C: Component> {
        entity: Entity,
        _marker: std::marker::PhantomData<C>,
    }

    impl<C: Component> RemoveComponent<C> {
        pub fn new(entity: Entity) -> Self {
            Self {
                entity,
                _marker: std::marker::PhantomData,
            }
        }
    }

    impl<C: Component> Event for RemoveComponent<C> {
        type Output = RemovedComponent<C>;
        const PRIORITY: i32 = AddComponent::<C>::PRIORITY - 1000;

        fn invoke(self, world: &mut super::World) -> Option<Self::Output> {
            let id = ComponentId::new::<C>();
            let mut result = world.remove_component(&self.entity, &id)?;
            let component = result.removed_mut().remove_component::<C>()?;
            Some(RemovedComponent::new(self.entity, component))
        }
    }

    pub struct RemoveComponents {
        entity: Entity,
        components: DenseSet<ComponentId>,
    }

    impl RemoveComponents {
        pub fn new(entity: Entity) -> Self {
            Self {
                entity,
                components: DenseSet::new(),
            }
        }

        pub fn with<C: Component>(mut self) -> Self {
            self.components.insert(ComponentId::new::<C>());
            self
        }
    }

    impl Event for RemoveComponents {
        type Output = Entity;
        const PRIORITY: i32 = RemoveComponent::<()>::PRIORITY;

        fn invoke(mut self, world: &mut super::World) -> Option<Self::Output> {
            let components = std::mem::take(&mut self.components);
            let mut result = world.remove_components(&self.entity, components)?;
            for (id, component) in result.removed.drain() {
                let meta = world.components().extension::<ComponentEvents>(&id);
                meta.remove(world, &self.entity, component);
            }
            Some(self.entity)
        }
    }

    pub struct ComponentEvents {
        add: Box<dyn Fn(&World, &Entity) + Send + Sync + 'static>,
        remove: Box<dyn Fn(&World, &Entity, ColumnCell) + Send + Sync + 'static>,
    }

    impl ComponentEvents {
        pub fn new<C: Component>() -> Self {
            Self {
                add: Box::new(|world, entity| {
                    let outputs = world.resource_mut::<EventOutputs<AddComponent<C>>>();
                    world.events().invoked::<AddComponent<C>>();
                    outputs.add(*entity);
                }),
                remove: Box::new(|world, entity, cell| {
                    let outputs = world.resource_mut::<EventOutputs<RemoveComponent<C>>>();
                    let component = cell.take::<C>();
                    world.events().invoked::<RemoveComponent<C>>();
                    outputs.add(RemovedComponent::new(*entity, component));
                }),
            }
        }

        pub fn add(&self, world: &World, entity: &Entity) {
            (self.add)(world, entity);
        }

        pub fn remove(&self, world: &World, entity: &Entity, cell: ColumnCell) {
            (self.remove)(world, entity, cell);
        }
    }

    pub struct ActivateSystemGroup {
        tag: SystemTag,
    }

    impl ActivateSystemGroup {
        pub fn new(tag: impl Into<SystemTag>) -> Self {
            Self { tag: tag.into() }
        }
    }

    impl Event for ActivateSystemGroup {
        type Output = ();

        fn invoke(self, world: &mut super::World) -> Option<Self::Output> {
            world.activate_system_group(self.tag);
            None
        }
    }

    pub struct DeactivateSystemGroup {
        tag: SystemTag,
    }

    impl DeactivateSystemGroup {
        pub fn new(tag: impl Into<SystemTag>) -> Self {
            Self { tag: tag.into() }
        }
    }

    impl Event for DeactivateSystemGroup {
        type Output = ();

        fn invoke(self, world: &mut super::World) -> Option<Self::Output> {
            world.deactivate_system_group(self.tag);
            None
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::{
            core::{Component, Entity, Resource},
            system::schedule::Root,
            world::{
                event::{
                    AddComponent, Despawn, Events, ParentUpdate, RemoveChildren, RemoveComponent,
                    RemoveComponents, RemovedComponent, SetParent,
                },
                World,
            },
        };

        use super::Spawn;

        #[test]
        fn spawn() {
            let mut world = World::new();
            let entity = world.spawn(None);
            assert_eq!(entity.id(), 0);
        }

        #[test]
        fn on_spawn() {
            let mut world = World::new();
            struct Spawned(bool);
            impl Resource for Spawned {}

            world.add_resource(Spawned(false));

            world.observe::<Spawn, _>(|entities: &[Entity], spawned: &mut Spawned| {
                spawned.0 = entities.len() == 1;
            });

            world.events().add(Spawn::new());

            world.run(Root);

            assert!(world.resource::<Spawned>().0);
        }

        #[test]
        fn on_add_component() {
            struct Player;
            impl Component for Player {}
            struct Added(bool);
            impl Resource for Added {}

            let mut world = World::new();
            world.register::<Player>();
            world.add_resource(Added(false));

            world.observe::<AddComponent<Player>, _>(|entities: &[Entity], added: &mut Added| {
                added.0 = entities.len() == 1;
            });

            world.events().add(Spawn::new().with(Player));

            world.run(Root);

            assert!(world.resource::<Added>().0);
        }

        #[test]
        fn on_add_components() {
            struct Player;
            impl Component for Player {}
            struct Health;
            impl Component for Health {}
            struct Added {
                player: bool,
                health: bool,
            }
            impl Resource for Added {}

            let mut world = World::new();
            world.register::<Player>();
            world.register::<Health>();
            world.add_resource(Added {
                player: false,
                health: false,
            });

            world.observe::<AddComponent<Player>, _>(|entities: &[Entity], added: &mut Added| {
                added.player = entities.len() == 1;
            });

            world.observe::<AddComponent<Health>, _>(|entities: &[Entity], added: &mut Added| {
                added.health = entities.len() == 1;
            });

            world.events().add(Spawn::new().with(Player).with(Health));

            world.run(Root);

            assert!(world.resource::<Added>().player);
            assert!(world.resource::<Added>().health);
        }

        #[test]
        fn on_remove_components() {
            struct Player;
            impl Component for Player {}
            struct Health;
            impl Component for Health {}
            struct Removed {
                player: bool,
                health: bool,
            }
            impl Resource for Removed {}

            let mut world = World::new();
            world.register::<Player>();
            world.register::<Health>();
            world.add_resource(Removed {
                player: false,
                health: false,
            });

            world.observe::<RemoveComponent<Player>, _>(
                |components: &[RemovedComponent<Player>], removed: &mut Removed| {
                    removed.player = components.len() == 1;
                },
            );

            world.observe::<RemoveComponent<Health>, _>(
                |components: &[RemovedComponent<Health>], removed: &mut Removed| {
                    removed.health = components.len() == 1;
                },
            );

            world.events().add(Spawn::new().with(Player).with(Health));
            world.run(Root);

            world.events().add(
                RemoveComponents::new(Entity::new(0, 0))
                    .with::<Player>()
                    .with::<Health>(),
            );
            world.run(Root);

            assert!(world.resource::<Removed>().player);
            assert!(world.resource::<Removed>().health);
        }

        #[test]
        fn on_remove_component() {
            struct Player;
            impl Component for Player {}
            struct Removed(bool);
            impl Resource for Removed {}

            let mut world = World::new();
            world.register::<Player>();
            world.add_resource(Removed(false));

            world.observe::<AddComponent<Player>, _>(|entities: &[Entity], events: &Events| {
                events.add(RemoveComponent::<Player>::new(*entities.first().unwrap()));
            });

            world.observe::<RemoveComponent<Player>, _>(
                |components: &[RemovedComponent<Player>], removed: &mut Removed| {
                    removed.0 = components.len() == 1;
                },
            );

            world.events().add(Spawn::new().with(Player));

            world.run(Root);

            assert!(world.resource::<Removed>().0);
        }

        #[test]
        fn on_despawn() {
            struct Despawned(bool);
            impl Resource for Despawned {}

            let mut world = World::new();
            world.add_resource(Despawned(false));

            world.observe::<Spawn, _>(|entities: &[Entity], events: &Events| {
                events.add(Despawn::new(*entities.first().unwrap()));
            });

            world.observe::<Despawn, _>(|entities: &[Vec<Entity>], despawned: &mut Despawned| {
                despawned.0 = entities.len() == 1;
            });

            world.events().add(Spawn::new());

            world.run(Root);

            assert!(world.resource::<Despawned>().0);
        }

        #[test]
        fn set_parent() {
            let mut world = World::new();
            let parent = world.spawn(None);
            let child = world.spawn(Some(parent));

            let children = world.entities().children(&parent);
            let has_child = children
                .map(|children| children.contains(&child))
                .unwrap_or_default();
            assert!(has_child);

            let child_parent = world.entities().parent(&child);
            assert_eq!(child_parent, Some(&parent));

            world.set_parent(&child, None);

            let children = world.entities().children(&parent);
            let has_child = children
                .map(|children| children.contains(&child))
                .unwrap_or_default();
            assert!(!has_child);

            let child_parent = world.entities().parent(&child);
            assert_eq!(child_parent, None);
        }

        #[test]
        fn on_set_parent() {
            struct Parented(bool);
            impl Resource for Parented {}

            let mut world = World::new();
            world.add_resource(Parented(false));

            world.observe::<SetParent, _>(|updates: &[ParentUpdate], parented: &mut Parented| {
                parented.0 = updates.len() == 1;
            });

            let parent = world.spawn(None);
            let child = world.spawn(None);

            world.events().add(SetParent::new(child, Some(parent)));

            world.run(Root);

            assert!(world.resource::<Parented>().0);
        }

        #[test]
        fn on_remove_children() {
            struct RemovedChildren(usize);
            impl Resource for RemovedChildren {}

            let mut world = World::new();
            world.add_resource(RemovedChildren(0));

            let parent = world.spawn(None);
            let children = (0..10)
                .map(|_| world.spawn(Some(parent)))
                .collect::<Vec<_>>();

            let child_count = children.len();

            world.observe::<RemoveChildren, _>(
                move |updates: &[Vec<ParentUpdate>], removed: &mut RemovedChildren| {
                    removed.0 = updates
                        .first()
                        .map(|updates| updates.len())
                        .unwrap_or_default();
                },
            );

            world.events().add(RemoveChildren::new(parent, children));

            world.run(Root);

            assert_eq!(world.resource::<RemovedChildren>().0, child_count);
        }
    }
}
