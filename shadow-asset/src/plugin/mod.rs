use shadow_game::{
    game::Game,
    plugin::{PhaseExt, Plugin, PluginContext},
    schedule::{DefaultPhaseRunner, Init, Phase},
};

use crate::{
    asset::{Asset, AssetSettings, Assets},
    database::{
        config::AssetConfig,
        events::{ImportAsset, ImportFailed, ImportFolder, RemoveAsset},
        observers::{import_assets, import_folders, Folder},
        pipeline::{AssetLoader, AssetPipeline, AssetRegistry},
        AssetDatabase,
    },
};
use std::path::{Path, PathBuf};

pub struct AssetPlugin {
    root: PathBuf,
}

impl AssetPlugin {
    pub fn new(root: impl AsRef<Path>) -> Self {
        AssetPlugin {
            root: root.as_ref().to_path_buf(),
        }
    }
}

impl Plugin for AssetPlugin {
    fn start(&mut self, ctx: &mut PluginContext) {
        let config = AssetConfig::new(self.root.clone());

        ctx.add_resource(AssetDatabase::new(config, ctx.events()))
            .observe::<ImportAsset, _>(import_assets)
            .observe::<ImportFolder, _>(import_folders)
            .register_event::<ImportFolder>()
            .register_event::<ImportAsset>()
            .register_event::<RemoveAsset>()
            .register_asset::<Folder>()
            .add_sub_phase::<Init, AssetInit>()
            .add_system(AssetInit, init);

        if let Some(registry) = ctx.try_resource_mut::<AssetRegistry>() {
            registry.register::<Folder>();
        } else {
            let mut registry = AssetRegistry::new();
            registry.register::<Folder>();
            ctx.add_resource(registry);
        }
    }
}

pub struct AssetInit;

impl Phase for AssetInit {
    type Runner = DefaultPhaseRunner;

    fn runner() -> Self::Runner {
        DefaultPhaseRunner
    }
}

fn init(database: &AssetDatabase) {
    let config = database.config();
    if !config.root().exists() {
        std::fs::create_dir_all(config.root()).unwrap();
    }

    if !config.assets().exists() {
        std::fs::create_dir(config.assets()).unwrap();
    }

    if !config.cache().exists() {
        std::fs::create_dir(config.cache()).unwrap();
    }

    if !config.artifacts().exists() {
        std::fs::create_dir(config.artifacts()).unwrap();
    }

    database.import_folder("");
}

pub trait AssetExt {
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn register_pipeline<P: AssetPipeline>(&mut self) -> &mut Self;
}

impl AssetExt for Game {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        self.add_resource(Assets::<A>::new())
            .register_event::<ImportFailed<A>>()
    }

    fn register_pipeline<P: AssetPipeline>(&mut self) -> &mut Self {
        self.register_asset::<<P::Loader as AssetLoader>::Asset>()
            .add_resource(AssetSettings::<<P::Loader as AssetLoader>::Settings>::new());
        if let Some(registry) = self.try_resource_mut::<AssetRegistry>() {
            registry.register::<P>();
        } else {
            let mut registry = AssetRegistry::new();
            registry.register::<P>();
            self.add_resource(registry);
        }

        self
    }
}

impl AssetExt for PluginContext<'_> {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        self.add_resource(Assets::<A>::new())
            .register_event::<ImportFailed<A>>()
    }

    fn register_pipeline<P: AssetPipeline>(&mut self) -> &mut Self {
        self.register_asset::<<P::Loader as AssetLoader>::Asset>()
            .add_resource(AssetSettings::<<P::Loader as AssetLoader>::Settings>::new());
        if let Some(registry) = self.try_resource_mut::<AssetRegistry>() {
            registry.register::<P>();
        } else {
            let mut registry = AssetRegistry::new();
            registry.register::<P>();
            self.add_resource(registry);
        }

        self
    }
}
