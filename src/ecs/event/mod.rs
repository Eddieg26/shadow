use super::{
    core::{internal::blob::Blob, Resource},
    storage::dense::DenseSet,
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

    pub fn ty(&self) -> &EventType {
        &self.ty
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

    pub fn drain_by_type<E: Event>(&self) -> Vec<ErasedEvent> {
        let mut events = self.events.lock().unwrap();
        let mut drained = Vec::new();
        let mut index = 0;
        while index < events.len() {
            if events[index].ty == TypeId::of::<E>() {
                drained.push(events.remove(index));
            } else {
                index += 1;
            }
        }

        drained
    }

    pub fn drain(&self) -> Vec<ErasedEvent> {
        let mut events = self.events.lock().unwrap();
        std::mem::take(&mut *events)
    }

    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        events.clear();
    }

    pub fn is_empty(&self) -> bool {
        let events = self.events.lock().unwrap();
        events.is_empty()
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

    pub fn slice(&self) -> &[E::Output] {
        &self.outputs
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EventInvocation {
    event: EventType,
    priority: i32,
}

impl EventInvocation {
    pub fn new<E: Event>() -> Self {
        Self {
            event: TypeId::of::<E>(),
            priority: E::PRIORITY,
        }
    }

    pub fn priority(&self) -> i32 {
        self.priority
    }

    pub fn event(&self) -> EventType {
        self.event
    }
}

impl PartialOrd for EventInvocation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}

pub struct EventInvocations {
    invocations: DenseSet<EventInvocation>,
}

impl EventInvocations {
    pub fn new() -> Self {
        Self {
            invocations: DenseSet::new(),
        }
    }

    pub fn add<E: Event>(&mut self) {
        self.invocations.insert(EventInvocation::new::<E>());
    }

    pub fn contains<E: Event>(&self) -> bool {
        self.invocations.contains(&EventInvocation::new::<E>())
    }

    fn sort(&mut self) {
        self.invocations.sort()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = EventInvocation> + '_ {
        self.invocations.sort();
        self.invocations.drain()
    }
}

impl Resource for EventInvocations {}
