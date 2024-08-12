use crate::loader::{AssetError, AssetErrorKind};

use super::AssetDatabase;
use shadow_ecs::{
    system::RunMode,
    task::TaskPool,
    world::event::{Event, Events},
};
use std::collections::VecDeque;

pub mod import;
pub mod load;

pub use import::*;
pub use load::*;

pub trait AssetEvent: Send + Sync + 'static {
    fn execute(&mut self, database: &AssetDatabase, events: &Events);
}

impl<A: AssetEvent> From<A> for Box<dyn AssetEvent> {
    fn from(event: A) -> Self {
        Box::new(event)
    }
}

pub struct AssetEvents {
    events: VecDeque<Box<dyn AssetEvent>>,
    running: bool,
}

impl AssetEvents {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            running: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn push(&mut self, event: impl Into<Box<dyn AssetEvent>>) {
        self.events.push_back(event.into());
    }

    pub fn push_front(&mut self, event: impl Into<Box<dyn AssetEvent>>) {
        self.events.push_front(event.into());
    }

    pub fn pop(&mut self) -> Option<Box<dyn AssetEvent>> {
        self.events.pop_front()
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}

pub struct StartAssetEvent {
    event: Box<dyn AssetEvent>,
}

impl StartAssetEvent {
    pub fn new(event: impl AssetEvent) -> Self {
        Self {
            event: event.into(),
        }
    }

    pub fn boxed(event: Box<dyn AssetEvent>) -> Self {
        Self { event }
    }

    pub fn event(&self) -> &dyn AssetEvent {
        &*self.event
    }

    pub fn on_start(_: &[()], database: &AssetDatabase, events: &Events, tasks: &TaskPool) {
        let mut db_events = database.events();
        if !db_events.is_running() {
            db_events.start();
            std::mem::drop(db_events);

            let database = database.clone();
            let events = events.clone();

            match database.config().mode() {
                RunMode::Sequential => {
                    AssetEventExecutor::execute(&database, &events);

                    database.events().stop();
                }
                RunMode::Parallel => tasks.spawn(move || {
                    AssetEventExecutor::execute(&database, &events);

                    database.events().stop();
                }),
            }
        }
    }
}

impl Event for StartAssetEvent {
    type Output = ();

    fn invoke(self, world: &mut shadow_ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();
        database.events().push(self.event);
        Some(())
    }
}

pub struct AssetEventExecutor;

impl AssetEventExecutor {
    pub fn execute(database: &AssetDatabase, events: &Events) {
        while let Some(mut event) = database.pop_event() {
            event.execute(database, events);
        }
    }
}

impl AssetError {
    pub fn observer(errors: &[AssetError], events: &Events) {
        let mut remove = Vec::new();
        let mut unloads = Vec::new();

        for error in errors {
            match error.kind() {
                AssetErrorKind::Import(path) => remove.push(path.clone()),
                AssetErrorKind::Load(path) => unloads.push(UnloadAsset::new(path.clone())),
            }
        }

        events.add(RemoveAssets::new(remove));
        events.extend(unloads);
    }
}
