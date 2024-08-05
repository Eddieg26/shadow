use super::{AssetEvent, LoadAssets};
use crate::{
    asset::AssetPath,
    loader::{AssetError, LoadErrorKind, LoadedAssets},
};
use shadow_ecs::world::event::Events;

impl AssetEvent for LoadAssets {
    fn execute(&self, database: &crate::database::AssetDatabase, _: &Events) {
        let mut errors = vec![];
        let mut assets = LoadedAssets::new();
        let io = database.io();

        let loaders = database.loaders();
        for load in &self.loads {
            let id = match &load.path {
                AssetPath::Id(id) => *id,
                AssetPath::Path(path) => match database.library().id(&path).copied() {
                    Some(id) => id,
                    None => continue,
                },
            };

            let meta = match database.io().load_artifact_meta(id) {
                Ok(artifact) => artifact,
                Err(error) => {
                    errors.push(AssetError::load(id, error));
                    continue;
                }
            };

            let loader = match loaders.get_ty(meta.ty()) {
                Some(loader) => loader,
                None => {
                    errors.push(AssetError::load(id, LoadErrorKind::NoLoader));
                    continue;
                }
            };

            let loaded = match loader.load(id, &loaders, io, &mut assets, load.load_dependencies) {
                Ok(loaded) => loaded,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };

            assets.add_erased(id, loaded);
        }
    }
}
