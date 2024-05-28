use super::internal::blob::Blob;
use crate::ecs::storage::dense::DenseMap;
use std::hash::{Hash, Hasher};
pub trait Resource: 'static {}
pub trait LocalResource: 'static {}

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
    pub fn new<R: 'static>() -> Self {
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
    pub fn new<R: 'static>(resource: R) -> Self {
        let mut data = Blob::new::<R>();
        data.push(resource);
        Self { data }
    }

    pub fn get<R: 'static>(&self) -> &R {
        self.data.get::<R>(0).unwrap()
    }

    pub fn get_mut<R: 'static>(&self) -> &mut R {
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

    pub fn add<R: 'static>(&mut self, resource: R) {
        let resource = ResourceData::new(resource);
        self.resources.insert(ResourceType::new::<R>(), resource);
    }

    pub fn cast<R: 'static>(&self) -> &R {
        let ty = ResourceType::new::<R>();
        let res = self
            .resources
            .get(&ty)
            .expect(format!("Resource doesn't exist. {}", std::any::type_name::<R>()).as_str());
        res.get::<R>()
    }

    pub fn cast_mut<R: 'static>(&self) -> &mut R {
        let ty = ResourceType::new::<R>();
        let res = self
            .resources
            .get(&ty)
            .expect(format!("Resource doesn't exist. {}", std::any::type_name::<R>()).as_str());

        res.get_mut::<R>()
    }
}

pub struct Resources(BaseResouces);

impl Resources {
    pub fn new() -> Self {
        Self(BaseResouces::new())
    }

    pub fn add<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.0.add(resource);
        self
    }

    pub fn get<R: Resource>(&self) -> &R {
        self.0.cast::<R>()
    }

    pub fn get_mut<R: Resource>(&self) -> &mut R {
        self.0.cast_mut::<R>()
    }

    pub fn try_get<R: Resource>(&self) -> Option<&R> {
        self.0
            .resources
            .get(&ResourceType::new::<R>())
            .map(|data| data.get::<R>())
    }

    pub fn try_get_mut<R: Resource>(&self) -> Option<&mut R> {
        self.0
            .resources
            .get(&ResourceType::new::<R>())
            .map(|data| data.get_mut::<R>())
    }
}

pub struct LocalResources(BaseResouces);

impl LocalResources {
    pub fn new() -> Self {
        Self(BaseResouces::new())
    }

    pub fn register<R: LocalResource>(&mut self, resource: R) {
        self.0.add(resource);
    }

    pub fn get<R: LocalResource>(&self) -> &R {
        self.0.cast::<R>()
    }

    pub fn get_mut<R: LocalResource>(&self) -> &mut R {
        self.0.cast_mut::<R>()
    }

    pub fn try_get<R: LocalResource>(&self) -> Option<&R> {
        self.0
            .resources
            .get(&ResourceType::new::<R>())
            .map(|data| data.get::<R>())
    }

    pub fn try_get_mut<R: LocalResource>(&self) -> Option<&mut R> {
        self.0
            .resources
            .get(&ResourceType::new::<R>())
            .map(|data| data.get_mut::<R>())
    }
}
