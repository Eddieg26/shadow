use crate::{
    asset::{Asset, AssetId, AssetPath, AssetType},
    bytes::ToBytes,
};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetStatus {
    None,
    Loading,
    Done,
    Importing,
    Failed,
}

impl AssetStatus {
    pub fn none(&self) -> bool {
        matches!(self, AssetStatus::None)
    }

    pub fn loading(&self) -> bool {
        matches!(self, AssetStatus::Loading)
    }

    pub fn done(&self) -> bool {
        matches!(self, AssetStatus::Done)
    }

    pub fn importing(&self) -> bool {
        matches!(self, AssetStatus::Importing)
    }

    pub fn failed(&self) -> bool {
        matches!(self, AssetStatus::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceInfo {
    id: AssetId,
    checksum: u64,
    modified: u64,
}

impl SourceInfo {
    pub fn new(id: AssetId, checksum: u64, modified: u64) -> Self {
        SourceInfo {
            id,
            checksum,
            modified,
        }
    }

    pub fn from(id: AssetId, asset: &[u8], metadata: &[u8], modified: u64) -> Self {
        let checksum = Self::calculate_checksum(asset, metadata);
        Self {
            id,
            checksum,
            modified,
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

    pub fn set_id(&mut self, id: AssetId) {
        self.id = id
    }

    pub fn update(&mut self, checksum: u64, modified: u64) {
        self.checksum = checksum;
        self.modified = modified;
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
        }
    }
}

impl ToBytes for SourceInfo {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.id.to_bytes();
        bytes.extend_from_slice(&self.checksum.to_bytes());
        bytes.extend_from_slice(&self.modified.to_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(bytes)?;
        let checksum = u64::from_bytes(&bytes[8..])?;
        let modified = u64::from_bytes(&bytes[16..])?;

        Some(SourceInfo::new(id, checksum, modified))
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

    pub fn filepath(&self) -> &Path {
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

#[derive(Debug, Clone)]
pub struct AssetLibrary {
    path: PathBuf,
    sources: Arc<RwLock<HashMap<PathBuf, SourceInfo>>>,
    blocks: Arc<RwLock<HashMap<AssetId, BlockInfo>>>,
    dependents: Arc<RwLock<HashMap<AssetId, HashSet<AssetId>>>>,
    assets: Arc<RwLock<HashMap<AssetId, AssetStatus>>>,
    importing: Arc<RwLock<HashSet<PathBuf>>>,
}

impl AssetLibrary {
    pub fn new(path: impl AsRef<Path>) -> Self {
        AssetLibrary {
            path: path.as_ref().to_path_buf(),
            sources: Arc::default(),
            blocks: Arc::default(),
            dependents: Arc::default(),
            assets: Arc::default(),
            importing: Arc::default(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn status(&self, path: impl Into<AssetPath>) -> AssetStatus {
        let path: AssetPath = path.into();
        let assets = self.assets.read().unwrap();

        let status = match &path {
            AssetPath::Id(id) => assets.get(id).copied().or_else(|| {
                self.block(id).map(|info| {
                    if self.importing(&info.filepath) {
                        AssetStatus::Importing
                    } else {
                        AssetStatus::None
                    }
                })
            }),
            AssetPath::Path(path) => {
                if let Some(source) = self.source(path) {
                    assets.get(&source.id()).copied()
                } else if self.importing(path) {
                    Some(AssetStatus::Importing)
                } else {
                    None
                }
            }
        };

        status.unwrap_or(AssetStatus::None)
    }

    pub fn importing(&self, path: impl AsRef<Path>) -> bool {
        self.importing
            .read()
            .unwrap()
            .get(&path.as_ref().to_path_buf())
            .is_some()
    }

    pub fn source(&self, path: impl AsRef<Path>) -> Option<SourceInfo> {
        self.sources.read().unwrap().get(path.as_ref()).cloned()
    }

    pub fn block(&self, id: &AssetId) -> Option<BlockInfo> {
        self.blocks.read().unwrap().get(id).cloned()
    }

    pub fn dependents(&self) -> std::sync::RwLockReadGuard<HashMap<AssetId, HashSet<AssetId>>> {
        self.dependents.read().unwrap()
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

    pub fn remove_block(&self, id: AssetId) -> Option<BlockInfo> {
        self.blocks.write().unwrap().remove(&id)
    }

    pub fn set_status(&self, id: AssetId, status: AssetStatus) -> Option<AssetStatus> {
        match status {
            AssetStatus::Importing => {
                if let Some(info) = self.block(&id) {
                    let path = info.filepath.clone();
                    self.importing.write().unwrap().insert(path);
                    self.assets.write().unwrap().remove(&id)
                } else {
                    None
                }
            }
            _ => {
                if let Some(info) = self.block(&id) {
                    let path = info.filepath.clone();
                    self.importing
                        .write()
                        .unwrap()
                        .remove(&path)
                        .then(|| AssetStatus::Importing)
                } else {
                    self.assets.write().unwrap().insert(id, status)
                }
            }
        }
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
    pub(crate) fn add_import(&self, path: &impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        self.importing.write().unwrap().insert(path);
    }

    pub(crate) fn remove_import(&self, path: &impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        self.importing.write().unwrap().remove(&path);
    }
}

impl ToBytes for AssetLibrary {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let sources = self.sources.read().unwrap();
        let blocks = self.blocks.read().unwrap();
        let dependents = self.dependents.read().unwrap();

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

        bytes.extend(dependents.len().to_bytes());
        for (id, dependents) in dependents.iter() {
            bytes.extend(id.to_bytes());

            let dependents = dependents.iter().copied().collect::<Vec<_>>().to_bytes();
            bytes.extend(dependents.len().to_bytes());
            bytes.extend(dependents);
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

        let dependents_len = usize::from_bytes(&bytes[offset..])?;
        offset += 8;

        let mut dependents = HashMap::new();
        for _ in 0..dependents_len {
            let id = AssetId::from_bytes(&bytes[offset..])?;
            offset += 8;

            let len = usize::from_bytes(&bytes[offset..])?;
            offset += 8;

            let set = Vec::<AssetId>::from_bytes(&bytes[offset..offset + len])?;
            offset += len;

            dependents.insert(id, set.into_iter().collect());
        }

        Some(AssetLibrary {
            path: PathBuf::new(),
            sources: Arc::new(RwLock::new(sources)),
            blocks: Arc::new(RwLock::new(blocks)),
            dependents: Arc::new(RwLock::new(dependents)),
            assets: Arc::default(),
            importing: Arc::default(),
        })
    }
}
