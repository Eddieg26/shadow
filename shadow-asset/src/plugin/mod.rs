use crate::{
    asset::{Asset, AssetSettings, Assets},
    database::{
        config::{AssetConfig, AssetDatabaseConfig},
        events::{
            AssetFailed, AssetImported, FolderImported, ImportAsset, ImportFolder, LoadAsset,
            LoadLibrary, SaveLibrary,
        },
        observers::{import_assets, import_folders},
        AssetDatabase,
    },
    loader::AssetPipeline,
    registry::AssetPipelineRegistry,
};
use shadow_game::{
    game::Game,
    plugin::{PhaseExt, Plugin, PluginContext},
    schedule::{DefaultPhaseRunner, Init, Phase},
};

pub struct AssetPlugin {
    config: AssetConfig,
}

impl AssetPlugin {
    pub fn new(config: AssetConfig) -> Self {
        AssetPlugin { config }
    }
}

impl Plugin for AssetPlugin {
    fn start(&mut self, ctx: &mut shadow_game::plugin::PluginContext) {
        ctx.add_sub_phase::<Init, AssetInitPhase>();
        ctx.add_system(AssetInitPhase, initialize_assets);

        ctx.register_event::<ImportFolder>();
        ctx.register_event::<FolderImported>();
        ctx.register_event::<SaveLibrary>();
        ctx.register_event::<LoadLibrary>();
        ctx.observe::<ImportFolder, _>(import_folders);

        if ctx.try_resource_mut::<AssetPipelineRegistry>().is_none() {
            ctx.add_resource(AssetPipelineRegistry::new());
        }
    }

    fn finish(&mut self, ctx: &mut PluginContext) {
        let config = AssetDatabaseConfig::new(self.config.assets(), self.config.cache());

        ctx.add_resource(AssetDatabase::new(config, ctx.events()));
    }
}

pub trait GameAssetExt {
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn register_pipeline<P: AssetPipeline>(&mut self) -> &mut Self;
}

impl GameAssetExt for Game {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        self.add_resource(Assets::<A>::new())
            .register_event::<LoadAsset<A>>()
            .register_event::<ImportAsset<A>>()
            .register_event::<AssetImported<A>>()
            .register_event::<AssetFailed<A>>()
    }

    fn register_pipeline<P: AssetPipeline>(&mut self) -> &mut Self {
        self.register_asset::<P::Asset>();
        self.add_resource(AssetSettings::<P::Settings>::new());
        if let Some(registry) = self.try_resource_mut::<AssetPipelineRegistry>() {
            registry.register::<P>();
        } else {
            let mut registry = AssetPipelineRegistry::new();
            registry.register::<P>();
            self.add_resource(registry);
        }

        self.observe::<ImportAsset<P::Asset>, _>(import_assets::<P>)
    }
}

impl GameAssetExt for PluginContext<'_> {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        self.add_resource(Assets::<A>::new())
            .register_event::<LoadAsset<A>>()
            .register_event::<ImportAsset<A>>()
            .register_event::<AssetFailed<A>>()
    }

    fn register_pipeline<P: AssetPipeline>(&mut self) -> &mut Self {
        self.register_asset::<P::Asset>();
        if let Some(registry) = self.try_resource_mut::<AssetPipelineRegistry>() {
            registry.register::<P>();
        } else {
            let mut registry = AssetPipelineRegistry::new();
            registry.register::<P>();
            self.add_resource(registry);
        }

        self.observe::<ImportAsset<P::Asset>, _>(import_assets::<P>)
    }
}

pub struct AssetInitPhase;

impl Phase for AssetInitPhase {
    type Runner = DefaultPhaseRunner;

    fn runner() -> Self::Runner {
        DefaultPhaseRunner
    }
}

fn initialize_assets(database: &AssetDatabase) {
    let inner = || -> std::io::Result<()> {
        std::fs::create_dir_all(database.config().assets())?;
        std::fs::create_dir_all(database.config().cache())?;
        std::fs::create_dir_all(database.config().blocks())
    };

    match inner() {
        Ok(_) => database.import_folder(""),
        Err(e) => println!("Failed to initialize assets: {:?}", e),
    }
}
