use ecs::core::Resource;
use spatial::partition::Partition;

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
