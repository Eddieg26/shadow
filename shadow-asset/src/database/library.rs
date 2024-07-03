use crate::{
    asset::{Asset, AssetId, AssetType},
    bytes::ToBytes,
};
use shadow_ecs::ecs::storage::dense::DenseMap;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::SystemTime,
};

#[derive(Clone, Copy, Debug)]
pub struct SourceInfo {
    id: AssetId,
    checksum: u32,
    asset_modified: u64,
    settings_modified: u64,
}

impl SourceInfo {
    pub fn new(id: AssetId) -> Self {
        SourceInfo {
            id,
            checksum: 0,
            asset_modified: 0,
            settings_modified: 0,
        }
    }

    pub fn with_checksum(mut self, checksum: u32) -> Self {
        self.checksum = checksum;
        self
    }

    pub fn with_asset_modified(mut self, asset_modified: u64) -> Self {
        self.asset_modified = asset_modified;
        self
    }

    pub fn with_settings_modified(mut self, source_modified: u64) -> Self {
        self.settings_modified = source_modified;
        self
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn checksum(&self) -> u32 {
        self.checksum
    }

    pub fn asset_modified(&self) -> u64 {
        self.asset_modified
    }

    pub fn settings_modified(&self) -> u64 {
        self.settings_modified
    }

    pub fn calculate_checksum(asset: &[u8], settings: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(asset);
        hasher.update(settings);
        hasher.finalize()
    }

    pub fn modified(path: &Path) -> u64 {
        match path.metadata() {
            Ok(data) => data
                .modified()
                .unwrap_or(SystemTime::now())
                .elapsed()
                .unwrap_or_default()
                .as_secs(),
            Err(_) => 0,
        }
    }
}

impl ToBytes for SourceInfo {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.to_bytes());
        bytes.extend_from_slice(&self.checksum.to_bytes());
        bytes.extend_from_slice(&self.asset_modified.to_bytes());
        bytes.extend_from_slice(&self.settings_modified.to_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(&bytes[..8])?;
        let checksum = u32::from_bytes(&bytes[8..12])?;
        let asset_modified = u64::from_bytes(&bytes[12..20])?;
        let settings_modified = u64::from_bytes(&bytes[20..])?;

        Some(SourceInfo {
            id,
            checksum,
            asset_modified,
            settings_modified,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ArtifactInfo {
    id: AssetId,
    filepath: PathBuf,
    ty: AssetType,
    dependencies: HashSet<AssetId>,
    dependents: HashSet<AssetId>,
}

impl ArtifactInfo {
    pub fn new<A: Asset>(id: AssetId, filepath: impl AsRef<Path>) -> Self {
        ArtifactInfo {
            id,
            filepath: filepath.as_ref().to_path_buf(),
            ty: AssetType::of::<A>(),
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
        }
    }

    pub fn of_ty(id: AssetId, filepath: impl AsRef<Path>, ty: AssetType) -> Self {
        ArtifactInfo {
            id,
            filepath: filepath.as_ref().to_path_buf(),
            ty,
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
        }
    }

    pub fn with_id(mut self, id: AssetId) -> Self {
        self.id = id;
        self
    }

    pub fn with_ty(mut self, ty: AssetType) -> Self {
        self.ty = ty;
        self
    }

    pub fn with_path(mut self, path: impl AsRef<Path>) -> Self {
        self.filepath = path.as_ref().to_path_buf();
        self
    }

    pub fn with_dependencies(mut self, dependencies: HashSet<AssetId>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_dependents(mut self, dependents: HashSet<AssetId>) -> Self {
        self.dependents = dependents;
        self
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn remove_dependency(&mut self, id: &AssetId) {
        self.dependencies.retain(|x| x != id);
    }

    pub fn add_dependent(&mut self, id: AssetId) {
        self.dependents.insert(id);
    }

    pub fn remove_dependent(&mut self, id: &AssetId) {
        self.dependents.retain(|x| x != id);
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn filepath(&self) -> &PathBuf {
        &self.filepath
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn dependents(&self) -> &HashSet<AssetId> {
        &self.dependents
    }
}

impl ToBytes for ArtifactInfo {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.to_bytes());
        bytes.extend_from_slice(&self.ty.to_bytes());
        bytes.extend_from_slice(&self.filepath.to_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(&bytes[..8])?;
        let ty = AssetType::from_bytes(&bytes[8..16])?;
        let filepath = PathBuf::from_bytes(&bytes[16..])?;
        Some(ArtifactInfo::of_ty(id, PathBuf::from(filepath), ty))
    }
}

pub struct Sources<'a> {
    sources: MutexGuard<'a, DenseMap<PathBuf, SourceInfo>>,
}

impl Sources<'_> {
    pub fn get(&self, path: &PathBuf) -> Option<&SourceInfo> {
        self.sources.get(path)
    }

    pub fn get_mut(&mut self, path: &PathBuf) -> Option<&mut SourceInfo> {
        self.sources.get_mut(path)
    }

    pub fn insert(&mut self, path: PathBuf, info: SourceInfo) {
        self.sources.insert(path, info);
    }

    pub fn remove(&mut self, path: &PathBuf) -> Option<SourceInfo> {
        self.sources.remove(path)
    }
}

pub struct Artifacts<'a> {
    artifacts: MutexGuard<'a, DenseMap<AssetId, ArtifactInfo>>,
}

impl Artifacts<'_> {
    pub fn get(&self, id: &AssetId) -> Option<&ArtifactInfo> {
        self.artifacts.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut ArtifactInfo> {
        self.artifacts.get_mut(id)
    }

    pub fn insert(&mut self, id: AssetId, info: ArtifactInfo) {
        self.artifacts.insert(id, info);
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<ArtifactInfo> {
        self.artifacts.remove(id)
    }
}

#[derive(Clone)]
pub struct AssetLibrary {
    source_info: Arc<Mutex<DenseMap<PathBuf, SourceInfo>>>,
    artifact_info: Arc<Mutex<DenseMap<AssetId, ArtifactInfo>>>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        AssetLibrary {
            source_info: Arc::default(),
            artifact_info: Arc::default(),
        }
    }

    pub fn sources(&self) -> Sources {
        self.source_info
            .lock()
            .map(|sources| Sources { sources })
            .unwrap()
    }

    pub fn artifacts(&self) -> Artifacts {
        self.artifact_info
            .lock()
            .map(|artifacts| Artifacts { artifacts })
            .unwrap()
    }

    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        let sources = self.sources();
        let artifacts = self.artifacts();

        let mut bytes = Vec::new();
        bytes.extend(sources.sources.len().to_bytes());

        for (path, source) in sources.sources.iter() {
            let path = path.to_bytes();
            let path_len = path.len();

            bytes.extend_from_slice(&path_len.to_bytes());
            bytes.extend_from_slice(&path);

            let info = source.to_bytes();
            let len = info.len();

            bytes.extend_from_slice(&len.to_bytes());
            bytes.extend_from_slice(&info);
        }

        bytes.extend(sources.sources.len().to_bytes());
        for artifact in artifacts.artifacts.values() {
            let info = artifact.to_bytes();
            let len = info.len();

            bytes.extend_from_slice(&len.to_bytes());
            bytes.extend_from_slice(&info);
        }

        std::fs::write(path, bytes)
    }

    pub fn load(&self, path: &PathBuf) -> std::io::Result<()> {
        let bytes = std::fs::read(path)?;

        let mut sources = DenseMap::new();
        let mut artifacts = DenseMap::new();

        let len = usize::from_bytes(&bytes[..8]).ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to read source info length",
        ))?;
        let mut offset = 8;
        for _ in 0..len {
            let path_len =
                usize::from_bytes(&bytes[offset..offset + 8]).ok_or(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to read source path length",
                ))?;

            let path = PathBuf::from_bytes(&bytes[offset + 8..offset + 8 + path_len]).ok_or(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to read source path",
                ),
            )?;

            let info_len =
                usize::from_bytes(&bytes[offset..offset + 8]).ok_or(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to read source info length",
                ))?;
            let info = SourceInfo::from_bytes(&bytes[offset + 8..offset + 8 + info_len]).ok_or(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to read source info",
                ),
            )?;
            sources.insert(path, info);
            offset += 8 + info_len;
        }

        let len = usize::from_bytes(&bytes[offset..offset + 8]).ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to read artifact info length",
        ))?;
        offset += 8;
        for _ in 0..len {
            let info_len =
                usize::from_bytes(&bytes[offset..offset + 8]).ok_or(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to read artifact info length",
                ))?;
            let info = ArtifactInfo::from_bytes(&bytes[offset + 8..offset + 8 + info_len]).ok_or(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to read artifact info",
                ),
            )?;
            artifacts.insert(info.id(), info);
            offset += 8 + info_len;
        }

        *self.source_info.lock().unwrap() = sources;
        *self.artifact_info.lock().unwrap() = artifacts;

        Ok(())
    }
}
