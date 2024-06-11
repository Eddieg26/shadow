use crate::{
    asset::{AssetId, AssetType},
    bytes::ToBytes,
};
use std::{
    collections::HashMap,
    ffi::OsString,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetStatus {
    None,
    Importing,
    Loading,
    Processing,
    Failed,
    Done,
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

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn checksum(&self) -> u64 {
        self.checksum
    }

    pub fn modified(&self) -> u64 {
        self.modified
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
    pub fn new(filepath: PathBuf, ty: AssetType) -> Self {
        BlockInfo { filepath, ty }
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

        let filepath = self.filepath.clone().into_os_string().to_bytes();
        bytes.extend(filepath.len().to_bytes());
        bytes.extend(filepath);
        bytes.extend(self.ty.to_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let len = usize::from_bytes(bytes)?;
        let filepath = OsString::from_bytes(&bytes[8..8 + len])?;
        let filepath = filepath.into();
        let ty = AssetType::from_bytes(&bytes[8 + len..])?;

        Some(BlockInfo::new(filepath, ty))
    }
}

#[derive(Debug, Clone)]
pub struct AssetLibrary {
    sources: Arc<RwLock<HashMap<PathBuf, SourceInfo>>>,
    blocks: Arc<RwLock<HashMap<AssetId, BlockInfo>>>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        AssetLibrary {
            sources: Arc::new(RwLock::new(HashMap::new())),
            blocks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn source(&self, path: &Path) -> Option<SourceInfo> {
        self.sources.read().unwrap().get(path).cloned()
    }

    pub fn block(&self, id: &AssetId) -> Option<BlockInfo> {
        self.blocks.read().unwrap().get(id).cloned()
    }

    pub fn add_source(&self, path: PathBuf, info: SourceInfo) -> Option<SourceInfo> {
        self.sources.write().unwrap().insert(path, info)
    }

    pub fn add_block(&self, id: AssetId, info: BlockInfo) -> Option<BlockInfo> {
        self.blocks.write().unwrap().insert(id, info)
    }

    pub fn remove_source(&self, path: &Path) -> Option<SourceInfo> {
        self.sources.write().unwrap().remove(path)
    }

    pub fn remove_block(&self, id: AssetId) -> Option<BlockInfo> {
        self.blocks.write().unwrap().remove(&id)
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
            sources: Arc::new(RwLock::new(sources)),
            blocks: Arc::new(RwLock::new(blocks)),
        })
    }
}
