use super::{
    core::{internal::blob::Blob, Resource},
    storage::dense::DenseSet,
    world::World,
};
use std::{
    any::TypeId,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex, RwLock},
};

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

pub struct EventMeta {
    priority: i32,
    invoke: Box<dyn Fn(&mut ErasedEvent, &mut World) + Send + Sync>,
    clear: Box<dyn Fn(&World) + Send + Sync>,
}

impl EventMeta {
    pub fn new<E: Event>() -> Self {
        Self {
            priority: E::PRIORITY,
            invoke: Box::new(|event, world| {
                let mut outputs = Vec::<E::Output>::new();
                let event = event.cast_mut::<E>().expect("invalid event type");
                if !event.skip(world) {
                    outputs.push(event.invoke(world));
                }
                world.resource_mut::<EventOutputs<E>>().extend(outputs);
            }),
            clear: Box::new(|world| {
                world.resource_mut::<EventOutputs<E>>().clear();
            }),
        }
    }

    pub fn priority(&self) -> i32 {
        self.priority
    }

    pub fn invoke(&self, event: &mut ErasedEvent, world: &mut World) {
        (self.invoke)(event, world)
    }

    pub fn clear(&self, world: &World) {
        (self.clear)(world)
    }
}

pub struct Events {
    events: Arc<Mutex<Vec<ErasedEvent>>>,
    metas: HashMap<EventType, Arc<EventMeta>>,
    invocations: Arc<RwLock<DenseSet<EventInvocation>>>,
}

impl Events {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            metas: HashMap::new(),
            invocations: Arc::new(RwLock::new(DenseSet::new())),
        }
    }

    pub fn register<E: Event>(&mut self) -> EventOutputs<E> {
        let meta = Arc::new(EventMeta::new::<E>());
        self.metas.insert(TypeId::of::<E>(), meta);
        EventOutputs::<E>::new(self.invocations.clone())
    }

    pub fn meta<E: Event>(&self) -> Arc<EventMeta> {
        let ty = TypeId::of::<E>();
        let meta = self.metas.get(&ty).expect("Event not registered");
        meta.clone()
    }

    pub fn meta_dynamic(&self, ty: &EventType) -> Arc<EventMeta> {
        let meta = self.metas.get(ty).expect("Event not registered");
        meta.clone()
    }

    pub fn add<E: Event>(&self, event: E) {
        let mut events = self.events.lock().unwrap();
        events.push(ErasedEvent::new(event));
    }

    pub fn remove<E: Event>(&self) -> Vec<ErasedEvent> {
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
        events.drain(..).collect::<Vec<_>>()
    }

    pub(crate) fn invocations(&self) -> Vec<EventInvocation> {
        let mut invocations = self.invocations.write().unwrap();
        invocations.sort();
        invocations.drain().collect::<Vec<_>>()
    }

    pub(crate) fn invocation_type<E: Event>(&self) -> Option<EventInvocation> {
        let mut invocations = self.invocations.write().unwrap();
        invocations.remove(&EventInvocation::new::<E>())
    }

    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        events.clear();
    }

    pub fn is_empty(&self) -> bool {
        let events = self.events.lock().unwrap();
        events.is_empty()
    }

    pub fn len(&self) -> usize {
        let events = self.events.lock().unwrap();
        events.len()
    }
}

pub struct EventOutputs<E: Event> {
    outputs: Vec<E::Output>,
    invocations: Arc<RwLock<DenseSet<EventInvocation>>>,
}

impl<E: Event> EventOutputs<E> {
    pub fn new(invocations: Arc<RwLock<DenseSet<EventInvocation>>>) -> Self {
        Self {
            outputs: Vec::new(),
            invocations,
        }
    }

    pub fn add(&mut self, output: E::Output) {
        self.outputs.push(output);
        self.invocations
            .write()
            .unwrap()
            .insert(EventInvocation::new::<E>());
    }

    pub fn extend(&mut self, outputs: Vec<E::Output>) {
        self.outputs.extend(outputs);
        self.invocations
            .write()
            .unwrap()
            .insert(EventInvocation::new::<E>());
    }

    pub fn drain(&mut self) -> Vec<E::Output> {
        self.outputs.drain(..).collect::<Vec<_>>()
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

    pub fn len(&self) -> usize {
        self.outputs.len()
    }
}

impl<E: Event> Resource for EventOutputs<E> {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord)]
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

impl Hash for EventInvocation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.event.hash(state);
    }
}
