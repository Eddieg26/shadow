use crate::{
    asset::{Asset, AssetId, AssetPath},
    database::{library::AssetStatus, AssetDatabase},
};
use shadow_ecs::ecs::event::Event;

#[derive(Clone)]
pub struct LoadAsset<A: Asset> {
    path: AssetPath,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> LoadAsset<A> {
    pub fn new(path: impl Into<AssetPath>) -> Self {
        LoadAsset {
            path: path.into(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }
}

impl<A: Asset> Event for LoadAsset<A> {
    type Output = AssetId;

    fn invoke(&mut self, world: &mut shadow_ecs::ecs::world::World) -> Option<Self::Output> {
        let database = world.resource::<AssetDatabase>();

        let id = match database.status(&self.path) {
            AssetStatus::None | AssetStatus::Failed | AssetStatus::Done => match &self.path {
                AssetPath::Id(id) => Some(*id),
                AssetPath::Path(path) => database.source(path).map(|source| source.id()),
            },
            AssetStatus::Importing | AssetStatus::Processing => {
                database.state().enqueue_load::<A>(&self.path);
                return None;
            }
            AssetStatus::Loading => return None,
        }?;

        database.state().set_asset_status(id, AssetStatus::Loading);
        Some(id)
    }
}
