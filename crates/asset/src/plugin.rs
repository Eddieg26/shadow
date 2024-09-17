use crate::{
    asset::{Asset, Assets},
    database::{
        events::{
            AssetImported, AssetLoaded, AssetUnloaded, AssetUpdated, DeleteAsset, DeleteAssets,
            ImportAsset, ImportAssets, ImportFolder, LoadAsset, LoadAssets, StartAssetEvent,
            UnloadAsset,
        },
        AssetConfig, AssetDatabase,
    },
    importer::{AssetError, AssetImporter},
    io::{embedded::EmbeddedAssets, PathExt},
    AssetActions, AssetId,
};
use game::{phases::Init, plugin::Plugin, Game, PostExecute};

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn start(&self, game: &mut Game) {
        game.init_resource::<AssetConfig>()
            .init_resource::<EmbeddedAssets>()
            .add_system(Init, asset_config_init);
    }

    fn run(&mut self, game: &mut Game) {
        game.register_event::<ImportFolder>()
            .register_event::<ImportAsset>()
            .register_event::<ImportAssets>()
            .register_event::<AssetImported>()
            .register_event::<DeleteAsset>()
            .register_event::<DeleteAssets>()
            .register_event::<LoadAsset>()
            .register_event::<LoadAssets>()
            .register_event::<UnloadAsset>()
            .register_event::<AssetError>()
            .register_event::<StartAssetEvent>()
            .observe::<ImportAsset, _>(ImportAsset::observer)
            .observe::<LoadAsset, _>(LoadAsset::observer)
            .observe::<DeleteAsset, _>(DeleteAsset::observer)
            .observe::<AssetError, _>(AssetError::observer)
            .observe::<StartAssetEvent, _>(StartAssetEvent::on_start);
    }

    fn finish(&mut self, game: &mut Game) {
        let config = match game.remove_resource::<AssetConfig>() {
            Some(config) => config,
            None => AssetConfig::default(),
        };

        let database = AssetDatabase::new(config);
        if let Some(mut embedded) = game.remove_resource::<EmbeddedAssets>() {
            let mut library = database.library_mut();
            for (id, (path, kind)) in embedded.drain() {
                library.add_asset(id, path, kind);
            }
        };

        game.add_resource(database);
    }
}

pub trait AssetExt: Sized {
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn add_importer<I: AssetImporter>(&mut self) -> &mut Self;
    fn embed(&mut self, id: AssetId, path: &'static str, data: &'static [u8]) -> &mut Self;
}

impl AssetExt for Game {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        let config = self.resource_mut::<AssetConfig>();
        if !config.registry().has::<A>() {
            config.register::<A>();
            self.register_event::<AssetLoaded<A>>()
                .register_event::<AssetUnloaded<A>>()
                .register_event::<AssetUpdated<A>>()
                .observe::<AssetLoaded<A>, _>(AssetLoaded::<A>::observer)
                .observe::<AssetUnloaded<A>, _>(AssetUnloaded::<A>::observer)
                .observe::<AssetUpdated<A>, _>(AssetUpdated::<A>::observer)
                .add_system(PostExecute, |actions: &mut AssetActions<A>| actions.clear())
                .init_resource::<Assets<A>>()
                .init_resource::<AssetActions<A>>();
        }

        self
    }

    fn add_importer<I: AssetImporter>(&mut self) -> &mut Self {
        self.register_asset::<I::Asset>();
        self.resource_mut::<AssetConfig>().add_importer::<I>();

        self
    }

    fn embed(&mut self, id: AssetId, path: &'static str, bytes: &'static [u8]) -> &mut Self {
        let config = self.resource::<AssetConfig>();
        let metadata = path
            .ext()
            .and_then(|ext| config.registry().get_metadata_by_ext(ext));

        if let Some(metadata) = metadata {
            let _ = metadata.embed(id, path, bytes, self.world());
        }

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
