use super::allocator::{Allocator, GenId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub struct Entity {
    id: u32,
    gen: u32,
}

impl Entity {
    pub fn new(id: u32, gen: u32) -> Entity {
        Entity { id, gen }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn gen(&self) -> u32 {
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

impl Into<GenId> for &Entity {
    fn into(self) -> GenId {
        GenId::new(self.id, self.gen)
    }
}

pub struct Entities {
    allocator: Allocator,
}

impl Entities {
    pub fn new() -> Entities {
        Entities {
            allocator: Allocator::new(),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        self.allocator.allocate().into()
    }

    pub fn despawn(&mut self, entity: &Entity) -> bool {
        self.allocator.free(&entity.into())
    }

    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.allocator.iter().map(|(id, gen)| Entity::new(id, gen))
    }
}
