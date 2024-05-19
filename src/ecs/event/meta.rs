use std::{any::TypeId, sync::Arc};

use super::{ErasedEvent, Event, EventInvocations, EventOutputs, EventType};
use crate::ecs::{core::Resource, storage::dense::DenseMap, world::World};

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
                let mut outputs = EventOutputs::<E>::new();
                let event = event.cast_mut::<E>().expect("invalid event type");
                let mut invoked = false;
                if !event.skip(world) {
                    outputs.add(event.invoke(world));
                    invoked = true;
                }

                if invoked {
                    world.resource_mut::<EventInvocations>().add::<E>();
                }
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

#[derive(Clone)]
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

    pub fn get(&self, ty: &EventType) -> &EventMeta {
        self.metas
            .get(ty)
            .map(|meta| &**meta)
            .expect("Event not registered")
    }
}

impl Resource for EventMetas {}
