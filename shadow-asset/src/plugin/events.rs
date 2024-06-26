use crate::{
    asset::{Asset, AssetId, AssetMetadata, AssetPath, AssetSettings, AssetType, Assets, Settings},
    config::AssetConfig,
    loader::AssetLoader,
    tracker::{AssetStatus, AssetTrackers},
};
use shadow_ecs::ecs::{
    core::Resource,
    event::{Event, Events},
    storage::dense::DenseMap,
    world::World,
};
use std::path::PathBuf;

pub struct ImportFolder {
    path: PathBuf,
}

impl ImportFolder {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl Event for ImportFolder {
    type Output = PathBuf;

    fn invoke(&mut self, _: &mut World) -> Option<Self::Output> {
        Some(std::mem::take(&mut self.path))
    }
}

pub struct ImportAsset<A: Asset> {
    path: PathBuf,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> ImportAsset<A> {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Event for ImportAsset<A> {
    type Output = PathBuf;

    fn invoke(&mut self, _: &mut World) -> Option<Self::Output> {
        Some(self.path.clone())
    }
}

pub struct AssetImported<A: Asset, S: Settings> {
    asset: Option<A>,
    metadata: Option<AssetMetadata<S>>,
}

impl<A: Asset, S: Settings> AssetImported<A, S> {
    pub fn new(asset: A, metadata: AssetMetadata<S>) -> Self {
        Self {
            asset: Some(asset),
            metadata: Some(metadata),
        }
    }
}

impl<A: Asset, S: Settings> Event for AssetImported<A, S> {
    type Output = (A, AssetMetadata<S>);

    fn invoke(&mut self, _: &mut World) -> Option<Self::Output> {
        Some((self.asset.take()?, self.metadata.take()?))
    }
}

pub struct LoadAsset<A: Asset> {
    path: AssetPath,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> LoadAsset<A> {
    pub fn new(path: impl Into<AssetPath>) -> Self {
        Self {
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

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let id = match &self.path {
            AssetPath::Id(id) => *id,
            AssetPath::Path(path) => {
                let trackers = world.resource_mut::<AssetTrackers>();
                if let Some(id) = trackers.get_path_id(path) {
                    if trackers.status(&id) == AssetStatus::Importing {
                        trackers.enqueue::<A>(id);
                        return None;
                    } else {
                        trackers.add::<A>(id);
                        id
                    }
                } else {
                    let config = world.resource::<AssetConfig>();
                    if let Ok(info) = config.load_asset_info(&path) {
                        let trackers = world.resource_mut::<AssetTrackers>();
                        trackers.set_path_id(path.clone(), info.id());
                        if trackers.status(&info.id()) == AssetStatus::Importing {
                            trackers.enqueue::<A>(info.id());
                            return None;
                        } else {
                            trackers.add::<A>(info.id());
                            info.id()
                        }
                    } else {
                        return None;
                    }
                }
            }
        };

        Some(id)
    }
}

pub struct UnloadAsset<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> UnloadAsset<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Event for UnloadAsset<A> {
    type Output = (AssetId, A);

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let asset = world.resource_mut::<Assets<A>>().remove(&self.id)?;
        Some((self.id, asset))
    }
}

pub struct UnloadSettings<S: Settings> {
    id: AssetId,
    _marker: std::marker::PhantomData<S>,
}

impl<S: Settings> UnloadSettings<S> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S: Settings> Event for UnloadSettings<S> {
    type Output = S;

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let settings = world.resource_mut::<AssetSettings<S>>().remove(&self.id)?;
        Some(settings)
    }
}

pub struct AssetLoaded<A: Asset> {
    id: AssetId,
    asset: Option<A>,
}

impl<A: Asset> AssetLoaded<A> {
    pub fn new(id: AssetId, asset: A) -> Self {
        Self {
            id,
            asset: Some(asset),
        }
    }
}

impl<A: Asset> Event for AssetLoaded<A> {
    type Output = AssetId;

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let asset = self.asset.take()?;
        world.resource_mut::<Assets<A>>().insert(self.id, asset);
        Some(self.id)
    }
}

pub struct SettingsLoaded<S: Settings> {
    id: AssetId,
    settings: Option<S>,
}

impl<S: Settings> SettingsLoaded<S> {
    pub fn new(id: AssetId, settings: S) -> Self {
        Self {
            id,
            settings: Some(settings),
        }
    }
}

impl<S: Settings> Event for SettingsLoaded<S> {
    type Output = AssetId;

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let settings = self.settings.take()?;
        world
            .resource_mut::<AssetSettings<S>>()
            .insert(self.id, settings);
        Some(self.id)
    }
}

pub struct ProcessAsset<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> ProcessAsset<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Event for ProcessAsset<A> {
    type Output = AssetId;

    fn invoke(&mut self, world: &mut World) -> Option<Self::Output> {
        let trackers = world.resource::<AssetTrackers>();
        let assets = world.resource::<Assets<A>>();
        if trackers.status(&self.id) == AssetStatus::Loaded && assets.contains(&self.id) {
            Some(self.id)
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct AssetMeta {
    import: fn(&Events, path: PathBuf),
    load: fn(&Events, path: AssetPath),
    process: fn(&Events, id: AssetId),
    unload: fn(&Events, id: AssetId),
    unload_settings: fn(&Events, id: AssetId),
}

impl AssetMeta {
    pub fn new<L: AssetLoader>() -> Self {
        Self {
            import: |events, path| {
                events.add(ImportAsset::<L::Asset>::new(path));
            },
            load: |events, path| {
                events.add(LoadAsset::<L::Asset>::new(path));
            },
            process: |events, id| {
                events.add(ProcessAsset::<L::Asset>::new(id));
            },
            unload: |events, id| {
                events.add(UnloadAsset::<L::Asset>::new(id));
            },
            unload_settings: |events, id| {
                events.add(UnloadSettings::<L::Settings>::new(id));
            },
        }
    }

    pub fn import(&self, events: &Events, path: PathBuf) {
        (self.import)(events, path);
    }

    pub fn load(&self, events: &Events, path: impl Into<AssetPath>) {
        (self.load)(events, path.into());
    }

    pub fn process(&self, events: &Events, id: AssetId) {
        (self.process)(events, id);
    }

    pub fn unload(&self, events: &Events, id: AssetId) {
        (self.unload)(events, id);
    }

    pub fn unload_settings(&self, events: &Events, id: AssetId) {
        (self.unload_settings)(events, id);
    }
}

#[derive(Clone)]
pub struct AssetMetas {
    metas: DenseMap<AssetType, AssetMeta>,
    ext_map: DenseMap<&'static str, AssetType>,
}

impl AssetMetas {
    pub fn new() -> Self {
        Self {
            metas: DenseMap::new(),
            ext_map: DenseMap::new(),
        }
    }

    pub fn add<L: AssetLoader>(&mut self) {
        let meta = AssetMeta::new::<L>();
        for ext in L::extensions() {
            self.ext_map.insert(ext, AssetType::of::<L::Asset>());
        }
        self.metas.insert(AssetType::of::<L::Asset>(), meta);
    }

    pub fn get<A: Asset>(&self) -> Option<&AssetMeta> {
        self.metas.get(&AssetType::of::<A>())
    }

    pub fn get_dyn(&self, ty: AssetType) -> Option<&AssetMeta> {
        self.metas.get(&ty)
    }

    pub fn get_by_ext(&self, ext: &str) -> Option<&AssetMeta> {
        self.ext_map.get(&ext).and_then(|ty| self.metas.get(ty))
    }
}

impl Resource for AssetMetas {}
