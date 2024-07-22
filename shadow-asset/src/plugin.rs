use crate::{
    events::{ImportAsset, LoadAsset, ReloadAsset, StartAssetEvent},
    importer::ImportError,
    observers::{
        on_import_asset, on_import_error, on_load_asset, on_reload_assets, on_start_asset_event,
    },
    AssetConfig, AssetDatabase, AssetFileSystem, LocalFileSystem,
};
use shadow_game::plugin::Plugin;
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
        ctx.observe::<ImportAsset, _>(on_import_asset);
        ctx.observe::<LoadAsset, _>(on_load_asset);
        ctx.observe::<ReloadAsset, _>(on_reload_assets);
        ctx.observe::<StartAssetEvent, _>(on_start_asset_event);
        ctx.observe::<ImportError, _>(on_import_error);
    }

    fn run(&mut self, _ctx: &mut shadow_game::plugin::PluginContext) {}

    fn finish(&mut self, _ctx: &mut shadow_game::plugin::PluginContext) {}
}
