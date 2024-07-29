use crate::{
    events::{
        AssetEventExt, AssetLoaded, ImportAsset, ImportFolder, LoadAsset, StartAssetEvent,
        UnloadAsset,
    },
    importer::{AssetImporter, AssetProcessor, ImportError, LoadError},
    observers::{
        on_asset_loaded, on_import_asset, on_import_error, on_load_asset, on_load_error,
        on_start_asset_event, on_unload_asset,
    },
    Asset, AssetDatabase, Assets,
};
use shadow_ecs::{event::Events, world::events::Spawn};
use shadow_game::{
    game::Game,
    plugin::{PhaseExt, Plugin, PluginContext},
    schedule::{DefaultPhaseRunner, Init, Phase},
};

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn start(&mut self, ctx: &mut shadow_game::plugin::PluginContext) {
        ctx.add_sub_phase::<Init, AssetInit>();
        ctx.register_event::<StartAssetEvent>();
        ctx.register_event::<ImportFolder>();
        ctx.register_event::<ImportAsset>();
        ctx.register_event::<LoadAsset>();
        ctx.register_event::<ImportError>();
        ctx.register_event::<LoadError>();
        ctx.observe::<StartAssetEvent, _>(on_start_asset_event);
        ctx.observe::<ImportAsset, _>(on_import_asset);
        ctx.observe::<LoadAsset, _>(on_load_asset);
        ctx.observe::<ImportError, _>(on_import_error);
        ctx.observe::<LoadError, _>(on_load_error);

        if let None = ctx.try_resource::<AssetDatabase>() {
            ctx.add_resource(AssetDatabase::new());
        }
    }

    fn finish(&mut self, ctx: &mut PluginContext) {
        let db = ctx.resource::<AssetDatabase>();
        let config = db.filesystem().config();
        let fs = db.filesystem();

        if !fs.exists(config.root()) {
            fs.create_dir(config.root()).ok();
        }

        if !fs.exists(config.preferences()) {
            fs.create_dir(config.preferences()).ok();
        }

        if !fs.exists(config.assets()) {
            fs.create_dir(config.assets()).ok();
        }

        if !fs.exists(config.cache()) {
            fs.create_dir(config.cache()).ok();
        }

        if !fs.exists(config.temp()) {
            fs.create_dir(config.temp()).ok();
        }

        if !fs.exists(fs.dependents()) {
            fs.create_dir(fs.dependents()).ok();
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

pub trait AssetPluginExt {
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn register_importer<A: AssetImporter>(&mut self) -> &mut Self;
    fn register_processer<A: AssetProcessor>(&mut self) -> &mut Self;
}

impl AssetPluginExt for Game {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        self.add_resource(Assets::<A>::new());
        self.register_event::<AssetLoaded<A>>();
        self.register_event::<UnloadAsset<A>>();
        self.observe::<AssetLoaded<A>, _>(on_asset_loaded::<A>);
        self.observe::<UnloadAsset<A>, _>(on_unload_asset::<A>);
        self
    }

    fn register_importer<I: AssetImporter>(&mut self) -> &mut Self {
        self.register_asset::<I::Asset>();

        if let Some(db) = self.try_resource::<AssetDatabase>() {
            db.importers_mut().register::<I>();
        } else {
            let db = AssetDatabase::new();
            db.importers_mut().register::<I>();
            self.add_resource(AssetDatabase::new());
        }

        self
    }

    fn register_processer<P: AssetProcessor>(&mut self) -> &mut Self {
        if let Some(db) = self.try_resource::<AssetDatabase>() {
            db.importers_mut().register_processer::<P>();
        } else {
            let db = AssetDatabase::new();
            db.importers_mut().register_processer::<P>();
            self.add_resource(db);
        }

        self
    }
}

impl AssetPluginExt for PluginContext<'_> {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        self.add_resource(Assets::<A>::new());
        self.observe::<AssetLoaded<A>, _>(on_asset_loaded::<A>);
        self.observe::<UnloadAsset<A>, _>(on_unload_asset::<A>);
        self
    }

    fn register_importer<I: AssetImporter>(&mut self) -> &mut Self {
        self.register_asset::<I::Asset>();

        if let Some(db) = self.try_resource::<AssetDatabase>() {
            db.importers_mut().register::<I>();
        } else {
            self.add_resource(AssetDatabase::new());
            self.register_importer::<I>();
        }

        self
    }

    fn register_processer<P: AssetProcessor>(&mut self) -> &mut Self {
        if let Some(db) = self.try_resource::<AssetDatabase>() {
            db.importers_mut().register_processer::<P>();
        } else {
            let db = AssetDatabase::new();
            db.importers_mut().register_processer::<P>();
            self.add_resource(db);
        }

        self
    }
}
