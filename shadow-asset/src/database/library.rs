use crate::{
    asset::{Asset, AssetId, AssetType},
    bytes::ToBytes,
};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

pub trait AssetStatus: Copy + Clone {
    type Id: Eq + PartialEq + Hash;
    fn fetch(id: &Self::Id, library: &AssetLibrary) -> Self;
    fn set(self, id: Self::Id, library: &AssetLibrary) -> Self {
        library.set_status(id, self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStatus {
    None,
    Loading,
    Done,
    Failed,
}

impl AssetStatus for LoadStatus {
    type Id = AssetId;

    fn fetch(id: &Self::Id, library: &AssetLibrary) -> Self {
        library.load_status(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStatus {
    None,
    Importing,
    Processing,
    PostProccessing,
    Done,
    Failed,
}

impl AssetStatus for ImportStatus {
    type Id = PathBuf;

    fn fetch(id: &Self::Id, library: &AssetLibrary) -> Self {
        library.import_status(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceInfo {
    id: AssetId,
    checksum: u64,
    modified: u64,
    settings_modified: u64,
}

impl SourceInfo {
    pub fn new(id: AssetId) -> Self {
        SourceInfo {
            id,
            checksum: 0,
            modified: 0,
            settings_modified: 0,
        }
    }

    pub fn from(id: AssetId, checksum: u64, modified: u64, settings_modified: u64) -> Self {
        Self {
            id,
            checksum,
            modified,
            settings_modified,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn checksum(&self) -> u64 {
        self.checksum
    }

    pub fn modified(&self) -> u64 {
        self.modified
    }

    pub fn settings_modified(&self) -> u64 {
        self.settings_modified
    }

    pub fn with_id(&self, id: AssetId) -> Self {
        let mut info = self.clone();
        info.id = id;
        info
    }

    pub fn with_checksum(&self, checksum: u64) -> Self {
        let mut info = self.clone();
        info.checksum = checksum;
        info
    }

    pub fn with_modified(&self, modified: u64) -> Self {
        let mut info = self.clone();
        info.modified = modified;
        info
    }

    pub fn with_settings_modified(&self, settings_modified: u64) -> Self {
        let mut info = self.clone();
        info.settings_modified = settings_modified;
        info
    }

    pub fn calculate_checksum(asset: &[u8], metadata: &[u8]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        asset.hash(&mut hasher);
        metadata.hash(&mut hasher);
        hasher.finish()
    }

    pub fn system_time_to_secs(sys_time: std::time::SystemTime) -> u64 {
        sys_time
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

impl Default for SourceInfo {
    fn default() -> Self {
        SourceInfo {
            id: AssetId::gen(),
            checksum: 0,
            modified: 0,
            settings_modified: 0,
        }
    }
}

impl ToBytes for SourceInfo {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.id.to_bytes();
        bytes.extend_from_slice(&self.checksum.to_bytes());
        bytes.extend_from_slice(&self.modified.to_bytes());
        bytes.extend_from_slice(&self.settings_modified.to_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(bytes)?;
        let checksum = u64::from_bytes(&bytes[8..])?;
        let modified = u64::from_bytes(&bytes[16..])?;
        let settings_modified = u64::from_bytes(&bytes[24..])?;

        Some(SourceInfo {
            id,
            checksum,
            modified,
            settings_modified,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    filepath: PathBuf,
    ty: AssetType,
}

impl BlockInfo {
    pub fn new(filepath: impl AsRef<Path>, ty: AssetType) -> Self {
        BlockInfo {
            filepath: filepath.as_ref().to_path_buf(),
            ty,
        }
    }

    pub fn of<A: Asset>(filepath: impl AsRef<Path>) -> Self {
        BlockInfo {
            filepath: filepath.as_ref().to_path_buf(),
            ty: AssetType::of::<A>(),
        }
    }

    pub fn filepath(&self) -> &PathBuf {
        &self.filepath
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn with_path(&self, path: PathBuf) -> Self {
        BlockInfo {
            filepath: path,
            ty: self.ty,
        }
    }
}

impl ToBytes for BlockInfo {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let filepath = self.filepath.to_bytes();
        bytes.extend(filepath.len().to_bytes());
        bytes.extend(filepath);
        bytes.extend(self.ty.to_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let len = usize::from_bytes(bytes)?;
        let filepath = PathBuf::from_bytes(&bytes[8..8 + len])?;
        let ty = AssetType::from_bytes(&bytes[8 + len..])?;

        Some(BlockInfo::new(filepath, ty))
    }
}

#[derive(Debug, Default)]
pub struct DependencyMap {
    dependencies: HashMap<AssetId, HashSet<AssetId>>,
    dependents: HashMap<AssetId, HashSet<AssetId>>,
}

impl DependencyMap {
    pub fn new() -> Self {
        DependencyMap {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    pub fn add_dependency(&mut self, id: AssetId, dependency: AssetId) {
        self.dependencies
            .entry(id)
            .or_insert_with(HashSet::new)
            .insert(dependency);

        self.dependents
            .entry(dependency)
            .or_insert_with(HashSet::new)
            .insert(id);
    }

    pub fn remove_dependency(&mut self, id: &AssetId, dependency: &AssetId) {
        if let Some(dependencies) = self.dependencies.get_mut(id) {
            dependencies.remove(dependency);
        }

        if let Some(dependents) = self.dependents.get_mut(dependency) {
            dependents.remove(id);
        }
    }

    pub fn insert(&mut self, id: AssetId, dependencies: HashSet<AssetId>) {
        for dependency in &dependencies {
            self.dependents
                .entry(*dependency)
                .or_insert_with(HashSet::new)
                .insert(id);
        }

        self.dependencies.insert(id, dependencies);
    }

    pub fn remove(&mut self, id: &AssetId) {
        if let Some(dependents) = self.dependencies.remove(id) {
            for dependent in dependents {
                self.dependents
                    .get_mut(&dependent)
                    .map(|dependents| dependents.remove(id));
            }
        }
    }

    pub fn dependencies(&self, id: &AssetId) -> HashSet<AssetId> {
        self.dependencies.get(id).cloned().unwrap_or_default()
    }

    pub fn dependents(&self, id: &AssetId) -> HashSet<AssetId> {
        self.dependents.get(id).cloned().unwrap_or_default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &HashSet<AssetId>)> {
        self.dependencies.iter()
    }
}

#[derive(Debug, Clone)]
pub struct AssetLibrary {
    path: PathBuf,
    sources: Arc<RwLock<HashMap<PathBuf, SourceInfo>>>,
    blocks: Arc<RwLock<HashMap<AssetId, BlockInfo>>>,
    dependency_map: Arc<RwLock<DependencyMap>>,
    loading: Arc<RwLock<HashMap<AssetId, LoadStatus>>>,
    importing: Arc<RwLock<HashMap<PathBuf, ImportStatus>>>,
}

impl AssetLibrary {
    pub fn new(path: impl AsRef<Path>) -> Self {
        AssetLibrary {
            path: path.as_ref().to_path_buf(),
            sources: Arc::default(),
            blocks: Arc::default(),
            dependency_map: Arc::default(),
            loading: Arc::default(),
            importing: Arc::default(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn status<S: AssetStatus>(&self, id: &S::Id) -> S {
        S::fetch(id, self)
    }

    pub fn source(&self, path: impl AsRef<Path>) -> Option<SourceInfo> {
        self.sources.read().unwrap().get(path.as_ref()).cloned()
    }

    pub fn block(&self, id: &AssetId) -> Option<BlockInfo> {
        self.blocks.read().unwrap().get(id).cloned()
    }

    pub fn dependencies(&self, id: &AssetId) -> HashSet<AssetId> {
        self.dependency_map.read().unwrap().dependencies(id)
    }

    pub fn dependents(&self, id: &AssetId) -> HashSet<AssetId> {
        self.dependency_map.read().unwrap().dependents(id)
    }

    pub fn add_dependency(&self, id: AssetId, dependency: AssetId) {
        self.dependency_map
            .write()
            .unwrap()
            .add_dependency(id, dependency);
    }

    pub fn remove_dependency(&self, id: &AssetId, dependency: &AssetId) {
        self.dependency_map
            .write()
            .unwrap()
            .remove_dependency(id, dependency);
    }

    pub fn set_source(&self, path: impl AsRef<Path>, info: SourceInfo) -> Option<SourceInfo> {
        self.sources
            .write()
            .unwrap()
            .insert(path.as_ref().to_path_buf(), info)
    }

    pub fn set_block(&self, id: AssetId, info: BlockInfo) -> Option<BlockInfo> {
        self.blocks.write().unwrap().insert(id, info)
    }

    pub fn remove_source(&self, path: &Path) -> Option<SourceInfo> {
        self.sources.write().unwrap().remove(path)
    }

    pub fn remove_block(&self, id: &AssetId) -> Option<BlockInfo> {
        self.blocks.write().unwrap().remove(id)
    }

    pub fn set_status<S: AssetStatus>(&self, id: S::Id, status: S) -> S {
        status.set(id, self)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let bytes = self.to_bytes();
        std::fs::write(self.path(), &bytes)
    }

    pub fn load(&self) -> std::io::Result<()> {
        let bytes = std::fs::read(self.path())?;

        let mut dst_sources = self
            .sources
            .write()
            .map_err(|_| std::io::ErrorKind::Other)?;
        let mut dst_blocks = self.blocks.write().map_err(|_| std::io::ErrorKind::Other)?;

        let library = AssetLibrary::from_bytes(&bytes).ok_or(std::io::ErrorKind::InvalidData)?;
        let mut src_sources = library
            .sources
            .write()
            .map_err(|_| std::io::ErrorKind::InvalidData)?;
        let mut src_blocks = library
            .blocks
            .write()
            .map_err(|_| std::io::ErrorKind::InvalidData)?;

        std::mem::swap(&mut *dst_sources, &mut *src_sources);
        std::mem::swap(&mut *dst_blocks, &mut *src_blocks);

        Ok(())
    }
}

impl AssetLibrary {
    fn load_status(&self, id: &AssetId) -> LoadStatus {
        self.loading
            .read()
            .unwrap()
            .get(id)
            .copied()
            .unwrap_or(LoadStatus::None)
    }

    fn import_status(&self, path: &Path) -> ImportStatus {
        self.importing
            .read()
            .unwrap()
            .get(path)
            .copied()
            .unwrap_or(ImportStatus::None)
    }

    fn set_load_status(&self, id: AssetId, status: LoadStatus) -> LoadStatus {
        self.loading
            .write()
            .unwrap()
            .insert(id, status)
            .unwrap_or(LoadStatus::None)
    }

    fn set_import_status(&self, path: PathBuf, status: ImportStatus) -> ImportStatus {
        self.importing
            .write()
            .unwrap()
            .insert(path, status)
            .unwrap_or(ImportStatus::None)
    }
}

impl ToBytes for AssetLibrary {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let sources = self.sources.read().unwrap();
        let blocks = self.blocks.read().unwrap();

        bytes.extend(sources.len().to_bytes());
        for (path, info) in sources.iter() {
            let path = path.clone().into_os_string().to_bytes();
            bytes.extend(path.len().to_bytes());
            bytes.extend(path);

            let info = info.to_bytes();
            bytes.extend(info.len().to_bytes());
            bytes.extend(info);
        }

        bytes.extend(blocks.len().to_bytes());
        for (id, info) in blocks.iter() {
            bytes.extend(id.to_bytes());

            let info = info.to_bytes();
            bytes.extend(info.len().to_bytes());
            bytes.extend(info);
        }

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut offset = 0;

        let sources_len = usize::from_bytes(&bytes[offset..])?;
        offset += 8;

        let mut sources = HashMap::new();
        for _ in 0..sources_len {
            let len = usize::from_bytes(&bytes[offset..])?;
            offset += 8;

            let path = OsString::from_bytes(&bytes[offset..offset + len])?;
            offset += len;

            let len = usize::from_bytes(&bytes[offset..])?;
            let info = SourceInfo::from_bytes(&bytes[offset..offset + len])?;
            offset += len;

            sources.insert(path.into(), info);
        }

        let blocks_len = usize::from_bytes(&bytes[offset..])?;
        offset += 8;

        let mut blocks = HashMap::new();
        for _ in 0..blocks_len {
            let id = AssetId::from_bytes(&bytes[offset..])?;
            offset += 8;

            let len = usize::from_bytes(&bytes[offset..])?;
            offset += 8;

            let info = BlockInfo::from_bytes(&bytes[offset..offset + len])?;
            offset += len;

            blocks.insert(id, info);
        }

        Some(AssetLibrary {
            path: PathBuf::new(),
            sources: Arc::new(RwLock::new(sources)),
            blocks: Arc::new(RwLock::new(blocks)),
            dependency_map: Arc::default(),
            loading: Arc::default(),
            importing: Arc::default(),
        })
    }
}
