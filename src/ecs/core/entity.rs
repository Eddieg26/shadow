use super::allocator::{Allocator, GenId};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub struct Entity {
    id: usize,
    gen: usize,
}

impl Entity {
    pub fn new(id: usize, gen: usize) -> Entity {
        Entity { id, gen }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn gen(&self) -> usize {
        self.gen
    }
}

impl From<GenId> for Entity {
    fn from(value: GenId) -> Self {
        Entity::new(value.id(), value.gen())
    }
}

impl Into<GenId> for Entity {
    fn into(self) -> GenId {
        GenId::new(self.id, self.gen)
    }
}

pub struct EntityNode {
    parent: Option<Entity>,
    children: Vec<Entity>,
}

impl EntityNode {
    pub fn new(parent: Option<Entity>) -> Self {
        EntityNode {
            parent,
            children: vec![],
        }
    }

    pub fn parent(&self) -> Option<&Entity> {
        self.parent.as_ref()
    }

    pub fn children(&self) -> &[Entity] {
        &self.children
    }

    pub fn add_child(&mut self, child: Entity) {
        self.children.push(child);
    }

    pub fn remove_child(&mut self, child: Entity) -> Option<Entity> {
        self.children.retain(|e| *e != child);
        Some(child)
    }

    pub fn set_parent(&mut self, parent: Option<Entity>) {
        self.parent = parent;
    }
}

pub struct Entities {
    allocator: Allocator,
    nodes: HashMap<Entity, EntityNode>,
}

impl Entities {
    pub fn new() -> Entities {
        Entities {
            allocator: Allocator::new(),
            nodes: HashMap::new(),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        let entity: Entity = self.allocator.allocate().into();
        let node = EntityNode::new(None);
        self.nodes.insert(entity, node);

        entity
    }

    pub fn spawn_with_parent(&mut self, parent: &Entity) -> Entity {
        if self.nodes.contains_key(parent) {
            let entity: Entity = self.allocator.allocate().into();
            let node = EntityNode::new(Some(*parent));
            self.nodes.insert(entity, node);
            self.nodes.get_mut(parent).unwrap().add_child(entity);

            entity
        } else {
            self.spawn()
        }
    }

    pub fn kill(&mut self, entity: &Entity) -> Vec<Entity> {
        let mut dead = vec![];
        if let Some(node) = self.nodes.remove(entity) {
            dead.push(*entity);
            for child in node.children() {
                dead.append(&mut self.kill(child));
            }
        }

        dead
    }

    pub fn set_parent(&mut self, child: &Entity, parent: Option<&Entity>) {
        if !self.nodes.contains_key(child) {
            return;
        }

        if let Some(old_parent) = self.nodes.get(child).unwrap().parent().copied() {
            let old = self.nodes.get_mut(&old_parent).unwrap();
            old.remove_child(*child);
        }

        if let Some(parent) = parent {
            if self.nodes.contains_key(parent) {
                self.nodes.get_mut(parent).unwrap().add_child(*child);
                self.nodes.get_mut(child).unwrap().set_parent(Some(*parent))
            }
        } else {
            self.nodes.get_mut(child).unwrap().set_parent(None);
        }
    }

    pub fn add_child(&mut self, parent: &Entity, child: &Entity) {
        if !self.nodes.contains_key(child) || !self.nodes.contains_key(parent) {
            return;
        }

        if let Some(old_parent) = self.nodes.get(child).unwrap().parent().copied() {
            let old = self.nodes.get_mut(&old_parent).unwrap();
            old.remove_child(*child);
        }

        self.nodes.get_mut(parent).unwrap().add_child(*child);
        self.nodes.get_mut(child).unwrap().set_parent(Some(*parent))
    }

    pub fn remove_child(&mut self, parent: &Entity, child: &Entity) -> bool {
        if !self.nodes.contains_key(child) || !self.nodes.contains_key(parent) {
            return false;
        }

        let old_parent = self.nodes.get(child).unwrap().parent().copied();
        if Some(*parent) != old_parent {
            return false;
        }

        self.nodes.get_mut(parent).unwrap().remove_child(*child);
        self.nodes.get_mut(child).unwrap().set_parent(None);

        true
    }

    pub fn children(&self, entity: &Entity) -> Option<&[Entity]> {
        self.nodes.get(entity).and_then(|n| Some(n.children()))
    }

    pub fn parent(&self, entity: &Entity) -> Option<&Entity> {
        self.nodes.get(entity).and_then(|n| n.parent())
    }
}
