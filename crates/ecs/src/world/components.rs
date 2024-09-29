use crate::core::{Component, Entity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Parent(Entity);

impl From<Entity> for Parent {
    fn from(entity: Entity) -> Self {
        Parent(entity)
    }
}

impl From<&Entity> for Parent {
    fn from(entity: &Entity) -> Self {
        Parent(*entity)
    }
}

impl std::ops::Deref for Parent {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Component for Parent {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Children(Vec<Entity>);

impl Children {
    pub fn new() -> Self {
        Children(Vec::new())
    }

    pub fn add(&mut self, entity: Entity) {
        self.0.push(entity);
    }

    pub fn remove(&mut self, entity: &Entity) -> bool {
        if let Some(index) = self.0.iter().position(|e| e == entity) {
            self.0.remove(index);
            true
        } else {
            false
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::ops::Deref for Children {
    type Target = [Entity];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Component for Children {}

impl From<Entity> for Children {
    fn from(entity: Entity) -> Self {
        let mut children = Children::new();
        children.add(entity);
        children
    }
}
