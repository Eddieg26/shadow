use crate::{
    asset::{Asset, Assets},
    database::{
        events::{
            AssetImported, AssetLoaded, AssetUnloaded, ImportAsset, ImportAssets, ImportFolder,
            LoadAsset, LoadAssets, RemoveAsset, RemoveAssets, StartAssetEvent, UnloadAsset,
        },
        AssetConfig, AssetDatabase,
    },
    loader::{AssetError, AssetLoader, AssetProcessor},
};
use shadow_ecs::world::{event::Events, World};
use shadow_game::{game::Game, phases::Init, plugin::Plugin};

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn run(&mut self, game: &mut Game) {
        let config = match game.remove_resource::<AssetConfig>() {
            Some(config) => config,
            None => AssetConfig::default(),
        };

        game.add_resource(AssetDatabase::new(config))
            .add_system(Init, asset_config_init)
            .register_event::<ImportFolder>()
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
    fn config(&mut self) -> &mut AssetConfig;
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn register_loader<L: AssetLoader>(&mut self) -> &mut Self;
    fn register_processor<P: AssetProcessor>(&mut self) -> &mut Self;
}

impl AssetExt for Game {
    fn config(&mut self) -> &mut AssetConfig {
        self.try_init_resource::<AssetConfig>()
    }

    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        if !self.config().registry().has::<A>() {
            self.config().register::<A>();
            self.register_event::<AssetLoaded<A>>()
                .register_event::<AssetUnloaded<A>>()
                .observe::<AssetLoaded<A>, _>(AssetLoaded::<A>::observer)
                .observe::<AssetUnloaded<A>, _>(AssetUnloaded::<A>::observer)
                .init_resource::<Assets<A>>();
        }

        self
    }

    fn register_loader<L: AssetLoader>(&mut self) -> &mut Self {
        self.register_asset::<L::Asset>();
        self.config().add_loader::<L>();

        self
    }

    fn register_processor<P: AssetProcessor>(&mut self) -> &mut Self {
        self.register_loader::<P::Loader>();
        self.config().set_processor::<P>();

        self
    }
}

impl AssetExt for World {
    fn config(&mut self) -> &mut AssetConfig {
        self.try_init_resource::<AssetConfig>()
    }

    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        if !self.config().registry().has::<A>() {
            self.config().register::<A>();
            self.register_event::<AssetLoaded<A>>()
                .register_event::<AssetUnloaded<A>>()
                .observe::<AssetLoaded<A>, _>(AssetLoaded::<A>::observer)
                .observe::<AssetUnloaded<A>, _>(AssetUnloaded::<A>::observer)
                .init_resource::<Assets<A>>();
        }

        self
    }

    fn register_loader<L: AssetLoader>(&mut self) -> &mut Self {
        self.register_asset::<L::Asset>();
        self.config().add_loader::<L>();

        self
    }

    fn register_processor<P: AssetProcessor>(&mut self) -> &mut Self {
        self.register_loader::<P::Loader>();
        self.config().set_processor::<P>();

        self
    }
}

fn asset_config_init(database: &AssetDatabase, events: &Events) {
    let config = database.config();

    if let Err(error) = config.init() {
        println!("Failed to initialize asset database: {}", error);
        return;
    }

    events.add(ImportFolder::new(""));
}
