use self::{
    events::{
        AssetLoaded, AssetMetas, ImportAsset, ImportFolder, LoadAsset, ProcessAsset,
        SettingsLoaded, UnloadAsset,
    },
    observers::{on_import_assets, on_import_folder, on_load_assets},
};
use crate::{
    asset::{Asset, AssetSettings, Assets},
    config::AssetConfig,
    database::AssetDatabase,
    loader::AssetLoader,
};
use shadow_game::{
    game::Game,
    plugin::{Plugin, PluginContext},
};

pub mod events;
pub mod observers;

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn start(&mut self, ctx: &mut PluginContext) {
        ctx.add_resource(AssetConfig::new("assets", "cache"));
        ctx.add_resource(AssetMetas::new());
        ctx.add_resource(AssetDatabase::new());
        ctx.register_event::<ImportFolder>();
        ctx.observe::<ImportFolder, _>(on_import_folder);
    }
}

pub trait AssetPluginExt {
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn register_loader<L: AssetLoader>(&mut self) -> &mut Self;
}

impl AssetPluginExt for Game {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        self.add_resource(Assets::<A>::new())
            .register_event::<ImportAsset<A>>()
            .register_event::<LoadAsset<A>>()
            .register_event::<AssetLoaded<A>>()
            .register_event::<ProcessAsset<A>>()
            .register_event::<UnloadAsset<A>>()
    }

    fn register_loader<L: AssetLoader>(&mut self) -> &mut Self {
        self.resource_mut::<AssetMetas>().add::<L>();
        self.register_asset::<L::Asset>()
            .register_event::<SettingsLoaded<L::Settings>>()
            .add_resource(AssetSettings::<L::Settings>::new())
            .observe::<ImportAsset<L::Asset>, _>(on_import_assets::<L>())
            .observe::<LoadAsset<L::Asset>, _>(on_load_assets::<L>())
    }
}

//AssetDatabase
//ImportAssets
//LoadAssets
