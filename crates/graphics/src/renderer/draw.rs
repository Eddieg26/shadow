use ecs::{
    core::Resource,
    system::{ArgItem, SystemArg},
};
use spatial::Partition;

pub trait Draw: Send + Sync + 'static {}

pub struct DrawCalls<D: Draw> {
    calls: Vec<D>,
}

impl<D: Draw> DrawCalls<D> {
    pub fn new() -> Self {
        Self { calls: vec![] }
    }

    pub fn add(&mut self, draw: D) {
        self.calls.push(draw);
    }

    pub fn iter(&self) -> impl Iterator<Item = &D> {
        self.calls.items()
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl<D: Draw> Resource for DrawCalls<D> {}

pub trait DrawCallExtractor: 'static {
    type Draw: Draw;
    type Arg: SystemArg;

    fn extract(draw: &mut DrawCalls<Self::Draw>, arg: ArgItem<Self::Arg>);
}
