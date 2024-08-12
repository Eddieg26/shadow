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

#[cfg(test)]
mod tests {
    use shadow_ecs::{
        core::Resource,
        system::{schedule::Root, RunMode},
        world::World,
    };
    use std::path::PathBuf;

    use crate::{
        asset::{Asset, AssetId, Assets, DefaultSettings},
        database::{
            events::{
                AssetLoaded, AssetUnloaded, ImportFolder, LoadAssets, StartAssetEvent, UnloadAsset,
            },
            AssetConfig, AssetDatabase,
        },
        io::{vfs::VirtualFileSystem, AssetIoError, AssetReader},
        loader::{AssetCacher, AssetError, AssetLoader, LoadContext},
    };

    use super::{AssetImported, ImportAssets, RemoveAssets};

    struct PlainText(String);
    impl Asset for PlainText {}

    impl AssetCacher for PlainText {
        type Asset = Self;
        type Error = AssetIoError;

        fn cache(asset: &Self::Asset) -> Result<Vec<u8>, Self::Error> {
            Ok(asset.0.as_bytes().to_vec())
        }

        fn load(data: &[u8]) -> Result<Self::Asset, Self::Error> {
            let content = String::from_utf8(data.to_vec())
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;

            Ok(Self(content))
        }
    }

    impl AssetLoader for PlainText {
        type Asset = Self;
        type Settings = DefaultSettings;
        type Error = AssetIoError;
        type Cacher = Self;

        fn load(
            _: &mut LoadContext<Self::Settings>,
            reader: &mut dyn AssetReader,
        ) -> Result<Self::Asset, Self::Error> {
            reader.read_to_end()?;
            <Self::Cacher as AssetCacher>::load(&reader.flush()?)
        }

        fn extensions() -> &'static [&'static str] {
            &["txt"]
        }
    }

    #[derive(Default)]
    pub struct Tracker {
        pub imported: bool,
        pub loaded: bool,
        pub unloaded: bool,
    }

    impl Resource for Tracker {}

    fn create_world() -> World {
        let mut config = AssetConfig::new(VirtualFileSystem::new(""));
        config.register::<PlainText>();
        config.add_loader::<PlainText>();
        config.set_run_mode(RunMode::Sequential);
        config.init().unwrap();

        let mut writer = config.writer(config.assets().join("test.txt"));
        writer.write("Hello, world!".as_bytes()).unwrap();
        writer.flush().unwrap();

        let mut world = World::new();
        world
            .add_resource(AssetDatabase::new(config))
            .init_resource::<Assets<PlainText>>()
            .init_resource::<Tracker>()
            .register_event::<AssetLoaded<PlainText>>()
            .register_event::<AssetUnloaded<PlainText>>()
            .register_event::<ImportFolder>()
            .register_event::<ImportAssets>()
            .register_event::<AssetImported>()
            .register_event::<RemoveAssets>()
            .register_event::<LoadAssets>()
            .register_event::<UnloadAsset>()
            .register_event::<AssetError>()
            .register_event::<StartAssetEvent>()
            .observe::<StartAssetEvent, _>(StartAssetEvent::on_start);

        world
    }

    #[test]
    fn import() {
        let mut world = create_world();
        world.build();

        world.events().add(ImportFolder::new(""));
        world.run(Root);

        let database = world.resource::<AssetDatabase>();

        let id = database.library().id(&PathBuf::from("test.txt")).cloned();
        assert!(id.is_some());
        assert!(database
            .config()
            .filesystem()
            .exists(&database.config().assets().join("test.txt.meta")));
        assert!(database
            .config()
            .filesystem()
            .exists(&database.config().artifact(id.unwrap())))
    }

    #[test]
    fn load() {
        let mut world = create_world();
        world.observe::<AssetLoaded<PlainText>, _>(
            |ids: &[AssetId], assets: &Assets<PlainText>, tracker: &mut Tracker| match assets
                .get(&ids[0])
            {
                Some(asset) => tracker.loaded = asset.0 == "Hello, world!",
                None => panic!("Asset not found"),
            },
        );
        world.build();

        world.events().add(ImportFolder::new(""));
        world.events().add(LoadAssets::hard(vec!["test.txt"]));
        world.run(Root);

        assert!(world.resource::<Tracker>().loaded);
    }

    #[test]
    fn unload() {
        let mut world = create_world();
        world.observe::<AssetUnloaded<PlainText>, _>(
            |unloads: &[AssetUnloaded<PlainText>], tracker: &mut Tracker| {
                tracker.unloaded = unloads[0].asset().0 == "Hello, world!";
            },
        );
        world.build();

        world.events().add(ImportFolder::new(""));
        world.events().add(LoadAssets::hard(vec!["test.txt"]));
        world.run(Root);

        world.events().add(UnloadAsset::new("test.txt"));
        world.run(Root);

        assert!(world.resource::<Tracker>().unloaded);
    }

    #[test]
    fn remove() {
        let mut world = create_world();
        world.build();

        world.events().add(ImportFolder::new(""));
        world.run(Root);

        let id = {
            let database = world.resource::<AssetDatabase>();
            database
                .library()
                .id(&PathBuf::from("test.txt"))
                .cloned()
                .unwrap()
        };

        world.events().add(RemoveAssets::new(vec!["test.txt"]));
        world.run(Root);

        let database = world.resource::<AssetDatabase>();
        let removed = database.library().id(&PathBuf::from("test.txt")).cloned();

        assert!(removed.is_none());
        assert!(!database
            .config()
            .filesystem()
            .exists(&database.config().artifact(id)))
    }
}
