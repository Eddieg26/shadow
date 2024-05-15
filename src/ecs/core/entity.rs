use crate::ecs::storage::sparse::SparseMap;

use super::{GenId, IdAllocator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    id: usize,
    generation: u32,
}

impl Entity {
    pub fn new(id: usize, generation: u32) -> Self {
        Self { id, generation }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }
}

impl Into<GenId> for Entity {
    fn into(self) -> GenId {
        GenId::new(self.id, self.generation)
    }
}

pub struct Entities {
    allocator: IdAllocator,
    nodes: SparseMap<Entity, EntityNode>,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            allocator: IdAllocator::new(),
            nodes: SparseMap::new(),
        }
    }

    pub fn create(&mut self) -> Entity {
        let id = self.allocator.allocate();
        let node = EntityNode::new(None);
        let entity = Entity::new(id.id(), id.generation());

        self.nodes.insert(entity, node);

        entity
    }

    pub fn delete(&mut self, entity: Entity, recursive: bool) -> Vec<Entity> {
        let mut deleted = Vec::new();
        if let Some(node) = self.nodes.remove(&entity) {
            if recursive {
                for child in node.children {
                    deleted.extend(self.delete(child, true));
                }
            }
            self.allocator
                .free(GenId::new(entity.id(), entity.generation()));
            deleted.push(entity);
        }
        deleted
    }

    pub fn reserve(&mut self, amount: usize) {
        self.allocator.reserve(amount);
    }

    pub fn len(&self) -> usize {
        self.allocator.len()
    }

    pub fn is_empty(&self) -> bool {
        self.allocator.is_empty()
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.allocator
            .is_alive(GenId::new(entity.id(), entity.generation()))
    }

    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.allocator
            .iter()
            .map(|id| Entity::new(id.id(), id.generation()))
    }
}

pub struct EntityNode {
    parent: Option<Entity>,
    children: Vec<Entity>,
}

impl EntityNode {
    pub fn new(parent: Option<Entity>) -> Self {
        Self {
            parent,
            children: Vec::new(),
        }
    }

    pub fn parent(&self) -> Option<Entity> {
        self.parent
    }

    pub fn children(&self) -> &[Entity] {
        &self.children
    }

    pub fn children_mut(&mut self) -> &mut [Entity] {
        &mut self.children
    }

    pub fn add_child(&mut self, entity: Entity) {
        self.children.push(entity);
    }

    pub fn remove_child(&mut self, entity: Entity) {
        self.children.retain(|e| *e != entity);
    }

    pub fn set_parent(&mut self, parent: Option<Entity>) {
        self.parent = parent;
    }
}

impl Entities {
    pub fn add_entity(&mut self, entity: Entity) {
        self.nodes.insert(
            entity,
            EntityNode {
                parent: None,
                children: Vec::new(),
            },
        );
    }

    pub fn set_parent(&mut self, entity: Entity, parent: Option<Entity>) {
        if let Some(old_parent) = self
            .nodes
            .get_mut(&entity)
            .and_then(|e| {
                let old = e.parent;
                e.parent = parent;
                old
            })
            .and_then(|old_parent| self.nodes.get_mut(&old_parent))
        {
            old_parent.children.retain(|e| *e != entity);
        }
        if let Some(parent) = parent {
            if let Some(parent_node) = self.nodes.get_mut(&parent) {
                parent_node.children.push(entity);
            }
        }
    }

    pub fn add_child(&mut self, entity: Entity, child: Entity) {
        if !self.contains(entity) || !self.contains(child) {
            return;
        }

        {
            let parent = self.nodes.get_mut(&entity).unwrap();
            parent.children.push(child);
        }

        let old_parent = self.nodes.get_mut(&child).and_then(|e| {
            let old = e.parent;
            e.parent = Some(entity);
            old
        });

        if let Some(old_parent) = old_parent {
            if let Some(old_parent) = self.nodes.get_mut(&old_parent) {
                old_parent.children.retain(|e| *e != child);
            }
        }
    }

    pub fn remove_child(&mut self, entity: Entity, child: Entity) {
        if !self.contains(entity) || !self.contains(child) {
            return;
        }

        if let Some(parent) = self.nodes.get_mut(&entity) {
            parent.children.retain(|e| *e != child);
        }

        if let Some(child) = self.nodes.get_mut(&child) {
            child.parent = None;
        }
    }

    pub fn parent(&self, entity: Entity) -> Option<Entity> {
        self.nodes.get(&entity).and_then(|e| e.parent)
    }

    pub fn children(&self, entity: Entity, recursive: bool) -> Vec<Entity> {
        let mut children = Vec::new();
        if let Some(node) = self.nodes.get(&entity) {
            children.extend(node.children.iter().cloned());
            if recursive {
                for child in node.children.iter() {
                    children.extend(self.children(*child, true));
                }
            }
        }
        children
    }
}
