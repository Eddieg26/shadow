use crate::{
    asset::{Asset, Assets},
    database::{
        config::AssetConfig,
        library::AssetLibraryError,
        observers::{on_asset_library_error, on_task_event},
        registry::AssetRegistry,
        task::{
            AssetTaskComplete, ImportAsset, ImportAssets, ImportFolder, LoadLibrary, RemoveAsset,
            RemoveAssets, SaveLibrary, StartAssetTask,
        },
        AssetDatabase,
    },
    importer::{AssetImporter, Folder, ImportFailed, LoadFailed},
};
use shadow_ecs::ecs::event::Events;
use shadow_game::{
    game::Game,
    plugin::{PhaseExt, Plugin, PluginContext},
    schedule::{DefaultPhaseRunner, Init, Phase},
};
use std::path::Path;

pub struct AssetPlugin {
    config: AssetConfig,
}

impl AssetPlugin {
    pub fn new(root: impl AsRef<Path>) -> AssetPlugin {
        AssetPlugin {
            config: AssetConfig::new(root),
        }
    }
}

impl Plugin for AssetPlugin {
    fn start(&mut self, ctx: &mut PluginContext) {
        ctx.add_sub_phase::<Init, AssetInit>()
            .add_system(AssetInit, init)
            .register_event::<StartAssetTask>()
            .register_event::<AssetTaskComplete>()
            .register_event::<ImportFolder>()
            .register_event::<ImportAsset>()
            .register_event::<ImportAssets>()
            .register_event::<RemoveAsset>()
            .register_event::<RemoveAssets>()
            .register_event::<SaveLibrary>()
            .register_event::<LoadLibrary>()
            .register_event::<AssetLibraryError>()
            .register_event::<ImportFailed>()
            .observe::<StartAssetTask, _>(on_task_event)
            .observe::<AssetTaskComplete, _>(on_task_event)
            .observe::<AssetLibraryError, _>(on_asset_library_error);
    }

    fn finish(&mut self, ctx: &mut PluginContext) {
        ctx.add_resource(AssetDatabase::new(self.config.clone()));
        ctx.register_importer::<Folder>();
    }
}

pub struct AssetInit;

impl Phase for AssetInit {
    type Runner = DefaultPhaseRunner;

    fn runner() -> Self::Runner {
        DefaultPhaseRunner
    }
}

fn init(database: &AssetDatabase, events: &Events) {
    let config = database.config();

    if !config.assets().exists() {
        std::fs::create_dir_all(config.assets()).unwrap();
    }

    if !config.cache().exists() {
        std::fs::create_dir_all(config.cache()).unwrap();
    }

    if !config.artifacts().exists() {
        std::fs::create_dir_all(config.artifacts()).unwrap();
    }

    events.add(LoadLibrary);
    events.add(ImportFolder::new(""));
}

pub trait AssetExt {
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn register_importer<A: AssetImporter>(&mut self) -> &mut Self;
}

impl AssetExt for Game {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        self.add_resource(Assets::<A>::new())
            .register_event::<LoadFailed<A>>()
    }

    fn register_importer<A: AssetImporter>(&mut self) -> &mut Self {
        self.register_asset::<A::Asset>();
        if let Some(registry) = self.try_resource_mut::<AssetRegistry>() {
            registry.register::<A>();
        } else {
            let mut registry = AssetRegistry::new();
            registry.register::<A>();
            self.add_resource(registry);
        }

        self
    }
}

impl AssetExt for PluginContext<'_> {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        self.add_resource(Assets::<A>::new())
            .register_event::<LoadFailed<A>>()
    }

    fn register_importer<A: AssetImporter>(&mut self) -> &mut Self {
        self.register_asset::<A::Asset>();
        if let Some(registry) = self.try_resource_mut::<AssetRegistry>() {
            registry.register::<A>();
        } else {
            let mut registry = AssetRegistry::new();
            registry.register::<A>();
            self.add_resource(registry);
        }

        self
    }
}
