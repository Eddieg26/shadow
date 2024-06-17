use crate::{
    asset::{Asset, Assets},
    database::{
        config::{AssetConfig, AssetDatabaseConfig},
        events::{AssetFailed, ImportAsset, ImportFolder, LoadAsset, LoadLibrary, SaveLibrary},
        observers::{import_assets, import_folders},
        AssetDatabase,
    },
    loader::AssetPipeline,
    registry::AssetPipelineRegistry,
};
use shadow_game::{
    game::Game,
    plugin::{Plugin, PluginContext},
};

pub struct AssetPlugin {
    config: AssetDatabaseConfig,
}

impl AssetPlugin {
    pub fn new(config: AssetDatabaseConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &AssetDatabaseConfig {
        &self.config
    }
}

impl Plugin for AssetPlugin {
    fn start(&mut self, ctx: &mut shadow_game::plugin::PluginContext) {
        ctx.add_resource(AssetConfig::new());
        if ctx.try_resource_mut::<AssetPipelineRegistry>().is_none() {
            ctx.add_resource(AssetPipelineRegistry::new());
        }

        ctx.register_event::<ImportFolder>();
        ctx.register_event::<SaveLibrary>();
        ctx.register_event::<LoadLibrary>();
        ctx.observe::<ImportFolder, _>(import_folders);
    }

    fn finish(&mut self, ctx: &mut PluginContext) {
        let config = {
            let config = ctx.resource_mut::<AssetConfig>();
            AssetDatabaseConfig::new(config.assets(), config.cache())
        };

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
