use crate::{core::Color, resources::ResourceId};
use ecs::core::Resource;
use spatial::partition::Partition;
use std::hash::Hash;

pub trait Draw: 'static {
    type Partition: Partition<Item = Self>;
}

impl Draw for () {
    type Partition = Vec<()>;
}

pub struct DrawCalls<D: Draw> {
    calls: D::Partition,
}

impl<D: Draw> DrawCalls<D> {
    pub fn new(partition: D::Partition) -> Self {
        Self { calls: partition }
    }

    pub fn add(&mut self, draw: D) {
        self.calls.insert(draw);
    }

    pub fn iter(&self) -> impl Iterator<Item = &D> {
        self.calls.items()
    }

    pub fn query(&self, query: &<D::Partition as Partition>::Query) -> Vec<&D> {
        self.calls.query(query)
    }

    pub fn extract(&mut self, other: &mut Self) {
        for call in other.calls.drain() {
            self.calls.insert(call);
        }
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl<D: Draw> Resource for DrawCalls<D> {}

pub trait Render: downcast_rs::Downcast + 'static {
    fn texture(&self) -> Option<ResourceId>;
    fn clear_color(&self) -> Option<Color>;
}

downcast_rs::impl_downcast!(Render);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderType(u32);

impl RenderType {
    pub fn from<T: Render>() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        std::any::TypeId::of::<T>().hash(&mut hasher);
        Self(hasher.finalize())
    }
}

pub struct RenderCall {
    render: Box<dyn Render>,
    ty: RenderType,
}

impl RenderCall {
    pub fn new<T: Render>(render: T) -> Self {
        Self {
            render: Box::new(render),
            ty: RenderType::from::<T>(),
        }
    }

    pub fn render_dyn(&self) -> &dyn Render {
        self.render.as_ref()
    }

    pub fn render<R: Render>(&self) -> Option<&R> {
        self.render.downcast_ref::<R>()
    }

    pub fn ty(&self) -> RenderType {
        self.ty
    }
}

pub struct RenderCalls {
    calls: Vec<RenderCall>,
}

impl RenderCalls {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }

    pub fn add<T: Render>(&mut self, render: T) {
        self.calls.push(RenderCall::new(render));
    }

    pub fn iter(&self) -> impl Iterator<Item = &RenderCall> {
        self.calls.items()
    }

    pub fn extract(&mut self, other: &mut Self) {
        self.calls.append(&mut other.calls);
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl Resource for RenderCalls {}
