use super::internal::blob::Blob;
use crate::ecs::storage::dense::DenseMap;
use std::hash::{Hash, Hasher};

pub trait BaseResource: 'static {}
pub trait Resource: BaseResource + 'static {}
pub trait LocalResource: BaseResource + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(u32);

impl ResourceId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceType(u64);

impl ResourceType {
    pub fn new<R: BaseResource>() -> Self {
        Self::dynamic(std::any::TypeId::of::<R>())
    }

    pub fn dynamic(type_id: std::any::TypeId) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        type_id.hash(&mut hasher);
        Self(hasher.finish())
    }

    pub fn raw(type_id: u64) -> Self {
        Self(type_id)
    }

    pub fn is<R: Resource>(&self) -> bool {
        self.0 == Self::new::<R>().0
    }
}

pub(crate) struct ResourceData {
    data: Blob,
}

impl ResourceData {
    pub fn new<R: BaseResource>(resource: R) -> Self {
        let mut data = Blob::new::<R>();
        data.push(resource);
        Self { data }
    }

    pub fn get<R: BaseResource>(&self) -> &R {
        self.data.get::<R>(0).unwrap()
    }

    pub fn get_mut<R: BaseResource>(&self) -> &mut R {
        self.data.get_mut::<R>(0).unwrap()
    }
}

pub struct BaseResouces {
    resources: DenseMap<ResourceType, ResourceData>,
}

impl BaseResouces {
    pub fn new() -> Self {
        Self {
            resources: DenseMap::new(),
        }
    }

    pub fn register<R: BaseResource>(&mut self, resource: R) {
        let resource = ResourceData::new(resource);
        self.resources.insert(ResourceType::new::<R>(), resource);
    }

    pub fn get<R: BaseResource>(&self) -> &R {
        let ty = ResourceType::new::<R>();
        let res = self.resources.get(&ty).expect("Resource doesn't exist.");
        res.get::<R>()
    }

    pub fn get_mut<R: BaseResource>(&self) -> &mut R {
        let ty = ResourceType::new::<R>();
        let res = self.resources.get(&ty).expect("Resource doesn't exist.");

        res.get_mut::<R>()
    }
}

pub struct Resources(BaseResouces);

impl Resources {
    pub fn new() -> Self {
        Self(BaseResouces::new())
    }
}

impl std::ops::Deref for Resources {
    type Target = BaseResouces;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Resources {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct LocalResources(BaseResouces);

impl LocalResources {
    pub fn new() -> Self {
        Self(BaseResouces::new())
    }
}

impl std::ops::Deref for LocalResources {
    type Target = BaseResouces;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for LocalResources {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
