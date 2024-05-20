use crate::ecs::storage::dense::DenseMap;
use std::{
    alloc::Layout,
    any::{Any, TypeId},
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::Arc,
};

pub trait Component: Send + Sync + 'static {}

impl Component for () {}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentId(usize);

impl ComponentId {
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl std::ops::Deref for ComponentId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for ComponentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentType(u64);

impl ComponentType {
    pub fn new<C: Component>() -> Self {
        Self::dynamic(std::any::TypeId::of::<C>())
    }

    pub fn dynamic(type_id: std::any::TypeId) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        type_id.hash(&mut hasher);
        Self(hasher.finish())
    }

    pub fn raw(type_id: u64) -> Self {
        Self(type_id)
    }

    pub fn is<C: Component>(&self) -> bool {
        self.0 == Self::new::<C>().0
    }
}

#[derive(Clone)]
pub struct ComponentMeta {
    name: &'static str,
    layout: Layout,
    type_id: TypeId,
    extensions: HashMap<TypeId, Arc<Box<dyn Any + Send + Sync + 'static>>>,
}

impl ComponentMeta {
    pub fn new<C: Component>() -> ComponentMeta {
        let name: &str = std::any::type_name::<C>();
        let layout: Layout = Layout::new::<C>();
        let type_id: TypeId = TypeId::of::<C>();

        ComponentMeta {
            name,
            layout,
            type_id,
            extensions: HashMap::new(),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn type_id(&self) -> &TypeId {
        &self.type_id
    }

    pub fn add_extension<T: Any + Send + Sync + 'static>(&mut self, extension: T) {
        self.extensions
            .insert(TypeId::of::<T>(), Arc::new(Box::new(extension)));
    }

    pub fn extension<T: Any>(&self) -> Option<&T> {
        self.extensions
            .get(&TypeId::of::<T>())
            .and_then(|ext| ext.downcast_ref::<T>())
    }
}

pub struct Components {
    metas: DenseMap<ComponentType, ComponentMeta>,
}

impl Components {
    pub fn new() -> Self {
        Components {
            metas: DenseMap::new(),
        }
    }

    pub fn register<C: Component>(&mut self) -> ComponentId {
        let meta = ComponentMeta::new::<C>();
        self.metas
            .insert(ComponentType::dynamic(*meta.type_id()), meta);

        ComponentId::new(self.metas.len() - 1)
    }

    pub fn id(&self, type_id: &ComponentType) -> ComponentId {
        self.metas
            .index(type_id)
            .map(|index| ComponentId::new(index))
            .expect("Component not found")
    }

    pub fn meta(&self, ty: &ComponentType) -> &ComponentMeta {
        self.metas.get(ty).expect("Component not found")
    }

    pub fn meta_at(&self, id: &ComponentId) -> &ComponentMeta {
        self.metas.get_at(**id).expect("Component not found")
    }

    pub fn extension<T: Any>(&self, id: ComponentId) -> &T {
        let meta = self.metas.get_at(*id).expect("Component not found");
        meta.extension().expect("Extension not found")
    }

    pub fn add_extension<T: Any + Send + Sync + 'static>(&mut self, id: ComponentId, extension: T) {
        let meta = self.metas.get_at_mut(*id).expect("Component not found");
        meta.add_extension(extension);
    }
}
