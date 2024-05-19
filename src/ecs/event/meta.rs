use std::{any::TypeId, sync::Arc};

use super::{ErasedEvent, Event, EventOutputs, EventType};
use crate::ecs::{core::Resource, storage::dense::DenseMap, world::World};

pub struct EventMeta {
    priority: i32,
    invoke: Box<dyn Fn(&mut [ErasedEvent], &mut World) + Send + Sync>,
    clear: Box<dyn Fn(&mut World) + Send + Sync>,
    clear_outputs: Box<dyn Fn(&World) + Send + Sync>,
}

impl EventMeta {
    pub fn new<E: Event>() -> Self {
        Self {
            priority: E::PRIORITY,
            invoke: Box::new(|events, world| {
                for event in events.iter_mut() {
                    let mut outputs = EventOutputs::<E>::new();
                    let event = event.cast_mut::<E>().expect("invalid event type");
                    if !event.skip(world) {
                        outputs.add(event.invoke(world));
                    }

                    outputs.swap(world);
                }
            }),
            clear: Box::new(|world| {
                world.events_mut().clear();
            }),
            clear_outputs: Box::new(|world| {
                world.resource_mut::<EventOutputs<E>>().clear();
            }),
        }
    }

    pub fn priority(&self) -> i32 {
        self.priority
    }

    pub fn invoke(&self, events: &mut [ErasedEvent], world: &mut World) {
        (self.invoke)(events, world)
    }

    pub fn clear(&self, world: &mut World) {
        (self.clear)(world)
    }

    pub fn clear_outputs(&self, world: &World) {
        (self.clear_outputs)(world)
    }
}

pub struct EventMetas {
    metas: DenseMap<EventType, Arc<EventMeta>>,
}

impl EventMetas {
    pub fn new() -> Self {
        Self {
            metas: DenseMap::new(),
        }
    }

    pub fn register<E: Event>(&mut self) {
        let meta = Arc::new(EventMeta::new::<E>());
        self.metas.insert(TypeId::of::<E>(), meta);
    }

    pub fn get(&self, ty: EventType) -> Option<&EventMeta> {
        self.metas.get(&ty).map(|meta| &**meta)
    }
}

impl Resource for EventMetas {}
