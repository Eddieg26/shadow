use crate::{
    asset::{Asset, AssetId, AssetInfo, AssetPath, AssetSettings, AssetType, Assets, Settings},
    bytes::AsBytes,
    config::AssetConfig,
    database::{AssetDatabase, AssetStatus},
    loader::AssetLoader,
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
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
        }
    }
}

impl Event for ImportFolder {
    type Output = PathBuf;

    fn invoke(&mut self, _: &mut World) -> Self::Output {
        std::mem::take(&mut self.path)
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

    fn invoke(&mut self, _: &mut World) -> Self::Output {
        self.path.clone()
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

    fn skip(&self, world: &World) -> bool {
        match &self.path {
            AssetPath::Path(path) => {
                let config = world.resource::<AssetConfig>();
                let path = config.asset_info_path(path);
                if let Ok(info) = std::fs::read(&path) {
                    if let Some(info) = AssetInfo::from_bytes(&info) {
                        let database = world.resource_mut::<AssetDatabase>();
                        let status = database.get_status(&info.id());
                        if status == AssetStatus::Unloaded {
                            database.load::<A>(info.id());
                            database.set_path_id(path.clone(), info.id());
                        }
                        return status != AssetStatus::Unloaded;
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            AssetPath::Id(id) => match world.resource::<AssetDatabase>().get_status(id) {
                AssetStatus::Unloaded => false,
                _ => true,
            },
        }
    }

    fn invoke(&mut self, world: &mut World) -> Self::Output {
        let id = match &self.path {
            AssetPath::Id(id) => *id,
            AssetPath::Path(path) => world.resource::<AssetDatabase>().get_path_id(path).unwrap(),
        };

        world.resource_mut::<AssetDatabase>().load::<A>(id);

        id
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
    type Output = A;

    fn skip(&self, world: &World) -> bool {
        !world.resource::<Assets<A>>().contains(&self.id)
    }

    fn invoke(&mut self, world: &mut World) -> Self::Output {
        world.resource_mut::<Assets<A>>().remove(&self.id).unwrap()
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

    fn invoke(&mut self, world: &mut World) -> Self::Output {
        let asset = self.asset.take().unwrap();
        world.resource_mut::<Assets<A>>().insert(self.id, asset);
        self.id
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

    fn invoke(&mut self, world: &mut World) -> Self::Output {
        let settings = self.settings.take().unwrap();
        world
            .resource_mut::<AssetSettings<S>>()
            .insert(self.id, settings);
        self.id
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

    fn invoke(&mut self, _: &mut World) -> Self::Output {
        self.id
    }
}

#[derive(Clone)]
pub struct AssetMeta {
    import: fn(&Events, path: PathBuf),
    load: fn(&Events, path: AssetPath),
    process: fn(&Events, id: AssetId),
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
}

#[derive(Clone)]
pub struct AssetMetas {
    metas: DenseMap<AssetType, AssetMeta>,
    ext_map: DenseMap<&'static str, Vec<AssetType>>,
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
        self.metas.insert(AssetType::of::<L::Asset>(), meta);
    }

    pub fn get<A: Asset>(&self) -> Option<&AssetMeta> {
        self.metas.get(&AssetType::of::<A>())
    }

    pub fn get_dyn(&self, ty: AssetType) -> Option<&AssetMeta> {
        self.metas.get(&ty)
    }

    pub fn get_by_ext(&self, ext: &str) -> Vec<&AssetMeta> {
        self.ext_map
            .get(&ext)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|ty| self.get_dyn(*ty).unwrap())
            .collect()
    }
}

impl Resource for AssetMetas {}
