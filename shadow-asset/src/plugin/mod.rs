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
    loader::AssetLoader,
    tracker::{AssetStatus, AssetTrackers},
};
use events::{AssetImported, UnloadSettings};
use shadow_ecs::ecs::event::Events;
use shadow_game::{
    game::Game,
    plugin::{PhaseExt, Plugin, PluginContext},
    schedule::{DefaultPhaseRunner, End, Init, Phase, Start},
};

pub mod events;
pub mod observers;

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn start(&mut self, ctx: &mut PluginContext) {
        ctx.add_sub_phase::<AssetImport, Init>();
        ctx.add_resource(AssetConfig::new("assets", "cache"));
        ctx.add_resource(AssetTrackers::new());
        ctx.register_event::<ImportFolder>();
        ctx.add_system(AssetImport, import_assets);
        ctx.add_system(Start, load_queued_assets);
        ctx.add_system(End, load_queued_assets);
        ctx.observe::<ImportFolder, _>(on_import_folder);
        if ctx.try_resource_mut::<AssetMetas>().is_none() {
            ctx.add_resource(AssetMetas::new());
        }
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
        if let Some(metas) = self.try_resource_mut::<AssetMetas>() {
            metas.add::<L>();
        } else {
            let mut metas = AssetMetas::new();
            metas.add::<L>();
            self.add_resource(metas);
        }
        self.register_asset::<L::Asset>()
            .register_event::<UnloadSettings<L::Settings>>()
            .register_event::<SettingsLoaded<L::Settings>>()
            .register_event::<AssetImported<L::Asset, L::Settings>>()
            .add_resource(AssetSettings::<L::Settings>::new())
            .observe::<ImportAsset<L::Asset>, _>(on_import_assets::<L>())
            .observe::<LoadAsset<L::Asset>, _>(on_load_assets::<L>())
    }
}

pub struct AssetImport;

impl Phase for AssetImport {
    type Runner = DefaultPhaseRunner;

    fn runner() -> Self::Runner {
        DefaultPhaseRunner
    }
}

pub fn import_assets(config: &AssetConfig, events: &Events) {
    std::fs::create_dir_all(config.assets()).ok();
    std::fs::create_dir_all(config.cache()).ok();
    events.add(ImportFolder::new(config.assets()));
}

pub fn load_queued_assets(trackers: &AssetTrackers, events: &Events, metas: &AssetMetas) {
    for (id, ty) in trackers.drain_queue() {
        let status = trackers.status(&id);
        match (metas.get_dyn(ty), status) {
            (Some(meta), AssetStatus::Importing) => meta.load(events, id),
            _ => {}
        }
    }
}
