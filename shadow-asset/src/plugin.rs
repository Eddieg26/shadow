use crate::{
    events::{ImportAsset, LoadAsset, StartAssetEvent},
    importer::{AssetImporter, AssetProcessor, ImportError, LoadError},
    observers::{
        on_import_asset, on_import_error, on_load_asset, on_load_error, on_start_asset_event,
    },
    Asset, AssetConfig, AssetDatabase, AssetFileSystem, Assets, LocalFileSystem,
};
use shadow_game::{game::Game, plugin::Plugin};
use std::path::{Path, PathBuf};

pub struct AssetPlugin {
    root: PathBuf,
}

impl AssetPlugin {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
}

impl Plugin for AssetPlugin {
    fn start(&mut self, ctx: &mut shadow_game::plugin::PluginContext) {
        let config = AssetConfig::new(&self.root);
        ctx.add_resource(AssetFileSystem::new(config, LocalFileSystem));
        ctx.add_resource(AssetDatabase::new());
        ctx.register_event::<StartAssetEvent>();
        ctx.register_event::<ImportAsset>();
        ctx.register_event::<LoadAsset>();
        ctx.register_event::<ImportError>();
        ctx.register_event::<LoadError>();
        ctx.observe::<StartAssetEvent, _>(on_start_asset_event);
        ctx.observe::<ImportAsset, _>(on_import_asset);
        ctx.observe::<LoadAsset, _>(on_load_asset);
        ctx.observe::<ImportError, _>(on_import_error);
        ctx.observe::<LoadError, _>(on_load_error);
    }

    fn run(&mut self, _ctx: &mut shadow_game::plugin::PluginContext) {
        //TODO: Create paths
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
        self
    }

    fn register_importer<A: AssetImporter>(&mut self) -> &mut Self {
        todo!()
    }

    fn register_processer<A: AssetProcessor>(&mut self) -> &mut Self {
        todo!()
    }
}
