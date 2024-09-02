use crate::{
    asset::{Asset, Assets},
    database::{
        events::{
            AssetImported, AssetLoaded, AssetUnloaded, ImportAsset, ImportAssets, ImportFolder,
            LoadAsset, LoadAssets, RemoveAsset, RemoveAssets, StartAssetEvent, UnloadAsset,
        },
        AssetConfig, AssetDatabase,
    },
    importer::{AssetError, AssetImporter},
};
use game::{phases::Init, plugin::Plugin, Game};

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn start(&self, game: &mut Game) {
        game.init_resource::<AssetConfig>()
            .add_system(Init, asset_config_init);
    }

    fn run(&mut self, game: &mut Game) {
        game.register_event::<ImportFolder>()
            .register_event::<ImportAsset>()
            .register_event::<ImportAssets>()
            .register_event::<AssetImported>()
            .register_event::<RemoveAsset>()
            .register_event::<RemoveAssets>()
            .register_event::<LoadAsset>()
            .register_event::<LoadAssets>()
            .register_event::<UnloadAsset>()
            .register_event::<AssetError>()
            .register_event::<StartAssetEvent>()
            .observe::<ImportAsset, _>(ImportAsset::observer)
            .observe::<LoadAsset, _>(LoadAsset::observer)
            .observe::<RemoveAsset, _>(RemoveAsset::observer)
            .observe::<AssetError, _>(AssetError::observer)
            .observe::<StartAssetEvent, _>(StartAssetEvent::on_start);
    }
}

pub trait AssetExt: Sized {
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn add_importer<I: AssetImporter>(&mut self) -> &mut Self;
}

impl AssetExt for Game {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        let config = self.resource_mut::<AssetConfig>();
        if !config.registry().has::<A>() {
            config.register::<A>();
            self.register_event::<AssetLoaded<A>>()
                .register_event::<AssetUnloaded<A>>()
                .observe::<AssetLoaded<A>, _>(AssetLoaded::<A>::observer)
                .observe::<AssetUnloaded<A>, _>(AssetUnloaded::<A>::observer)
                .init_resource::<Assets<A>>();
        }

        self
    }

    fn add_importer<I: AssetImporter>(&mut self) -> &mut Self {
        self.register_asset::<I::Asset>();
        self.resource_mut::<AssetConfig>().add_importer::<I>();

        self
    }
}

fn asset_config_init(database: &AssetDatabase) {
    let config = database.config();

    if let Err(error) = config.init() {
        println!("Failed to initialize asset database: {}", error);
        return;
    }
}
