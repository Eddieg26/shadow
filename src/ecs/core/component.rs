use crate::ecs::storage::dense::DenseMap;
use std::{alloc::Layout, any::TypeId};

pub trait Component: 'static {}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentId(usize);

impl ComponentId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl std::fmt::Display for ComponentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone)]
pub struct ComponentMeta {
    name: &'static str,
    layout: Layout,
    type_id: TypeId,
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
}

pub struct Components {
    metas: DenseMap<TypeId, ComponentMeta>,
}

impl Components {
    pub fn new() -> Self {
        Components {
            metas: DenseMap::new(),
        }
    }

    pub fn register<C: Component>(&mut self) {
        let meta = ComponentMeta::new::<C>();
        self.metas.insert(*meta.type_id(), meta);
    }

    pub fn id(&self, type_id: &TypeId) -> Option<ComponentId> {
        self.metas
            .index(type_id)
            .map(|index| ComponentId::new(index))
    }

    pub fn meta(&self, type_id: &TypeId) -> &ComponentMeta {
        self.metas.get(type_id).unwrap()
    }
}
