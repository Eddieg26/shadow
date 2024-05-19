use super::access::WorldAccessType;
use crate::ecs::{
    core::internal::blob::Blob,
    event::{meta::EventMetas, Event, EventInvocations, EventOutputs, EventType},
    storage::dense::DenseMap,
    world::World,
};
use std::any::TypeId;

pub struct Observer<E: Event> {
    function: Box<dyn Fn(&[E::Output], &World)>,
    reads: Vec<WorldAccessType>,
    writes: Vec<WorldAccessType>,
}

impl<E: Event> Observer<E> {
    pub fn new(function: impl Fn(&[E::Output], &World) + 'static) -> Self {
        Self {
            function: Box::new(function),
            reads: Vec::new(),
            writes: Vec::new(),
        }
    }

    pub fn reads(&self) -> &[WorldAccessType] {
        &self.reads
    }

    pub fn writes(&self) -> &[WorldAccessType] {
        &self.writes
    }

    pub fn run(&self, output: &[E::Output], world: &World) {
        (self.function)(output, world);
    }
}

pub struct Observers {
    ty: EventType,
    observers: Blob,
    observe: fn(&Blob, &World),
}

impl Observers {
    pub fn new<E: Event>() -> Self {
        Self {
            ty: TypeId::of::<E>(),
            observers: Blob::new::<Observer<E>>(),
            observe: |observers, world| {
                let outputs = world.resource::<EventOutputs<E>>();
                for observer in observers.iter::<Observer<E>>() {
                    observer.run(outputs.slice(), world);
                }
            },
        }
    }

    pub fn ty(&self) -> EventType {
        self.ty
    }

    pub fn observe(&self, world: &World) {
        (self.observe)(&self.observers, world);
    }

    pub fn add<E: Event>(&mut self, observer: Observer<E>) {
        if self.ty != TypeId::of::<E>() {
            panic!("Event type mismatch!");
        }
        self.observers.push(observer);
    }
}

pub struct EventObservers {
    observers: DenseMap<EventType, Observers>,
}

impl EventObservers {
    pub fn new() -> Self {
        Self {
            observers: DenseMap::new(),
        }
    }

    pub fn add_observer<E: Event>(&mut self, observer: Observer<E>) {
        let ty = TypeId::of::<E>();
        if let Some(observers) = self.observers.get_mut(&ty) {
            observers.add(observer);
        } else {
            let mut observers = Observers::new::<E>();
            observers.add(observer);
            self.observers.insert(ty, observers);
        }
    }

    pub fn add_observers(&mut self, observers: Vec<Observers>) {
        for observers in observers {
            self.observers.insert(observers.ty(), observers);
        }
    }

    pub fn run(&self, world: &World) {
        for invocation in world.resource_mut::<EventInvocations>().drain() {
            if let Some(observers) = self.observers.get(&invocation.event()) {
                observers.observe(world);
                let meta = world.resource::<EventMetas>().get(&invocation.event());
                meta.clear(world);
            }
        }
    }
}
