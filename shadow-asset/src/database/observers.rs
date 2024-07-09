use super::{
    library::AssetLibraryError,
    registry::AssetRegistry,
    task::{AssetTaskComplete, AssetTaskExecutorState},
    AssetDatabase,
};
use shadow_ecs::ecs::{
    event::{Event, EventStorage, Events},
    task::TaskPool,
};

pub fn on_task_event(
    _: &[()],
    database: &mut AssetDatabase,
    registry: &AssetRegistry,
    pool: &TaskPool,
    events: &Events,
) {
    let mut tasks = database.tasks();
    match tasks.state() {
        AssetTaskExecutorState::Running => (),
        AssetTaskExecutorState::Idle => {
            if !tasks.is_empty() {
                tasks.set_state(AssetTaskExecutorState::Running);
                drop(tasks);
                let database = database.clone();
                let registry = registry.clone();
                let events = events.clone();
                pool.spawn(move || {
                    while let Some(task) = database.pop_task() {
                        let mut task_events = EventStorage::new();
                        task.execute(&database, &registry, &mut task_events);
                        events.append(task_events);
                    }

                    database.tasks().set_state(AssetTaskExecutorState::Idle);
                    events.add(AssetTaskComplete);
                });
            }
        }
    }
}

pub fn on_asset_library_error(errors: &[<AssetLibraryError as Event>::Output]) {
    for error in errors {
        println!("Asset library error: {:?}", error);
    }
}
