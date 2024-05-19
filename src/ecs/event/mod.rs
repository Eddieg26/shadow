use super::{
    core::{internal::blob::Blob, Resource},
    world::World,
};
use std::{
    any::TypeId,
    sync::{Arc, Mutex},
};

pub mod meta;

pub trait Event: Send + Sync + 'static {
    type Output: Send + Sync + 'static;
    const PRIORITY: i32 = 0;

    fn priority(&self) -> i32 {
        Self::PRIORITY
    }

    fn skip(&self, _: &World) -> bool {
        false
    }

    fn invoke(&mut self, world: &mut World) -> Self::Output;
}

pub type EventType = TypeId;

pub struct ErasedEvent {
    ty: EventType,
    event: Blob,
}

impl ErasedEvent {
    pub fn new<E: Event>(event: E) -> Self {
        let mut data = Blob::new::<E>();
        data.push(event);
        Self {
            ty: TypeId::of::<E>(),
            event: data,
        }
    }

    pub fn ty(&self) -> EventType {
        self.ty
    }

    pub fn data(&self) -> &Blob {
        &self.event
    }

    pub fn cast<E: Event>(&self) -> Option<&E> {
        if self.ty == TypeId::of::<E>() {
            self.event.get::<E>(0)
        } else {
            None
        }
    }

    pub fn cast_mut<E: Event>(&mut self) -> Option<&mut E> {
        if self.ty == TypeId::of::<E>() {
            self.event.get_mut::<E>(0)
        } else {
            None
        }
    }
}

pub struct Events {
    events: Arc<Mutex<Vec<ErasedEvent>>>,
}

impl Events {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add<E: Event>(&self, event: E) {
        let mut events = self.events.lock().unwrap();
        events.push(ErasedEvent::new(event));
    }

    pub fn drain(&self) -> Vec<ErasedEvent> {
        let mut events = self.events.lock().unwrap();
        std::mem::take(&mut *events)
    }

    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        events.clear();
    }
}

pub struct EventOutputs<E: Event> {
    outputs: Vec<E::Output>,
}

impl<E: Event> EventOutputs<E> {
    pub fn new() -> Self {
        Self {
            outputs: Vec::new(),
        }
    }

    pub fn add(&mut self, output: E::Output) {
        self.outputs.push(output);
    }

    pub fn drain(&mut self) -> Vec<E::Output> {
        std::mem::take(&mut self.outputs)
    }

    pub fn swap(&mut self, world: &mut World) {
        let outputs = world.resource_mut::<EventOutputs<E>>();
        std::mem::swap(&mut outputs.outputs, &mut self.outputs);
    }

    pub fn clear(&mut self) {
        self.outputs.clear();
    }
}

impl<E: Event> Resource for EventOutputs<E> {}
