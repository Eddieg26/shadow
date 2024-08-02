use crate::core::{ComponentId, ResourceType};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum WorldAccessType {
    None,
    World,
    Component(ComponentId),
    Resource(ResourceType),
    LocalResource(ResourceType),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Access {
    Read,
    Write,
}

pub struct WorldAccess {
    ty: WorldAccessType,
    access: Access,
}

impl WorldAccess {
    pub fn new(ty: WorldAccessType, access: Access) -> Self {
        Self { ty, access }
    }

    pub fn read(ty: WorldAccessType) -> Self {
        Self::new(ty, Access::Read)
    }

    pub fn write(ty: WorldAccessType) -> Self {
        Self::new(ty, Access::Write)
    }

    pub fn from_type(ty: WorldAccessType, access: Access) -> Self {
        Self { ty, access }
    }

    pub fn ty(&self) -> WorldAccessType {
        self.ty
    }

    pub fn access(&self) -> Access {
        self.access
    }

    pub fn pick(
        reads: &mut Vec<WorldAccessType>,
        writes: &mut Vec<WorldAccessType>,
        access: &[WorldAccess],
    ) {
        for access in access.iter() {
            match access.access {
                Access::Read => reads.push(access.ty),
                Access::Write => writes.push(access.ty),
            }
        }
    }
}
