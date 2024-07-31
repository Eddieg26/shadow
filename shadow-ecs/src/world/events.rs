use super::World;
use crate::{
    core::{Component, ComponentId, Entity},
    event::{Event, EventOutputs},
    storage::{
        dense::{DenseMap, DenseSet},
        table::{Column, ComponentSet},
    },
};
pub struct Spawn {
    parent: Option<Entity>,
    components: DenseMap<ComponentId, Column>,
}

impl Spawn {
    pub fn new() -> Self {
        Self {
            parent: None,
            components: DenseMap::new(),
        }
    }

    pub fn set_parent(mut self, parent: Entity) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with<C: Component>(mut self, component: C) -> Self {
        let mut column = Column::new::<C>();
        column.insert(component);
        self.components.insert(ComponentId::new::<C>(), column);
        self
    }
}

impl Event for Spawn {
    type Output = Entity;
    const PRIORITY: i32 = i32::MAX - 1000;

    fn invoke(&mut self, world: &mut super::World) -> Option<Self::Output> {
        let entity = world.spawn(self.parent);
        if matches!(self.parent, Some(_)) {
            world.events().add(SetParent::new(entity, self.parent));
        }
        let mut components = ComponentSet::new();
        for (id, column) in self.components.drain() {
            components.add_column(id, column);
        }
        world.add_components(&entity, components);
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

    fn invoke(&mut self, world: &mut super::World) -> Option<Self::Output> {
        let mut entities = vec![];
        for (entity, mut set) in world.despawn(&self.entity).drain() {
            entities.push(entity);
            while let Some((id, mut column)) = set.remove_at(0) {
                let meta = world.components().extension::<ComponentEvents>(&id);
                meta.remove(world, &entity, &mut column);
            }
        }
        if entities.is_empty() {
            None
        } else {
            Some(entities)
        }
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

    fn invoke(&mut self, world: &mut super::World) -> Option<Self::Output> {
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

    fn invoke(&mut self, world: &mut super::World) -> Option<Self::Output> {
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

    fn invoke(&mut self, world: &mut super::World) -> Option<Self::Output> {
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

    fn invoke(&mut self, world: &mut super::World) -> Option<Self::Output> {
        let component = self.component.take()?;
        let _ = world.add_component(&self.entity, component)?;

        Some(self.entity)
    }
}

pub struct AddComponents {
    entity: Entity,
    components: DenseMap<ComponentId, Column>,
}

impl AddComponents {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            components: DenseMap::new(),
        }
    }

    pub fn with<C: Component>(mut self, component: C) -> Self {
        let mut column = Column::new::<C>();
        column.insert(component);
        self.components.insert(ComponentId::new::<C>(), column);

        self
    }
}

impl Event for AddComponents {
    type Output = Entity;
    const PRIORITY: i32 = AddComponent::<()>::PRIORITY;

    fn invoke(&mut self, world: &mut super::World) -> Option<Self::Output> {
        let mut components = ComponentSet::new();
        for (id, column) in self.components.drain() {
            components.add_column(id, column);

            let metas = world.components().extension::<ComponentEvents>(&id);
            metas.add(world, &self.entity);
        }
        let _ = world.add_components(&self.entity, components)?;
        Some(self.entity)
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
    type Output = (Entity, C);
    const PRIORITY: i32 = AddComponent::<C>::PRIORITY - 1000;

    fn invoke(&mut self, world: &mut super::World) -> Option<Self::Output> {
        let id = ComponentId::new::<C>();
        let mut _move = world.remove_component(&self.entity, &id)?;
        let component = _move.removed_mut().remove_component::<C>(&id)?;
        Some((self.entity, component))
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

    fn invoke(&mut self, world: &mut super::World) -> Option<Self::Output> {
        let components = std::mem::take(&mut self.components);
        let _move = world.remove_components(&self.entity, components)?;
        Some(self.entity)
    }
}

pub struct ComponentEvents {
    add: Box<dyn Fn(&World, &Entity) + Send + Sync + 'static>,
    remove: Box<dyn Fn(&World, &Entity, &mut Column) + Send + Sync + 'static>,
}

impl ComponentEvents {
    pub fn new<C: Component>() -> Self {
        Self {
            add: Box::new(|world, entity| {
                let outputs = world.resource_mut::<EventOutputs<AddComponent<C>>>();
                outputs.add(*entity);
            }),
            remove: Box::new(|world, entity, column| {
                let outputs = world.resource_mut::<EventOutputs<RemoveComponent<C>>>();
                let component = column.swap_remove(0).remove::<C>(0);
                outputs.add((*entity, component));
            }),
        }
    }

    pub fn add(&self, world: &World, entity: &Entity) {
        (self.add)(world, entity);
    }

    pub fn remove(&self, world: &World, entity: &Entity, column: &mut Column) {
        (self.remove)(world, entity, column);
    }
}
