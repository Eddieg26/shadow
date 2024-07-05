use super::config::AssetConfig;
use crate::{artifact::ArtifactMeta, asset::AssetId, bytes::ToBytes};
use shadow_ecs::ecs::storage::dense::DenseMap;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
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

    pub fn raw(id: AssetId, checksum: u32, asset_modified: u64, settings_modified: u64) -> Self {
        SourceInfo {
            id,
            checksum,
            asset_modified,
            settings_modified,
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

    pub fn calculate_checksum(asset: &[u8], metadata: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(asset);
        hasher.update(metadata);
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

#[derive(Debug, Default)]
pub struct DependencyInfo {
    dependencies: HashSet<AssetId>,
    dependents: HashSet<AssetId>,
}

impl DependencyInfo {
    pub fn new() -> Self {
        DependencyInfo {
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
        }
    }

    pub fn dependencies(&self) -> impl Iterator<Item = &AssetId> {
        self.dependencies.iter()
    }

    pub fn dependents(&self) -> impl Iterator<Item = &AssetId> {
        self.dependents.iter()
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn add_dependent(&mut self, id: AssetId) {
        self.dependents.insert(id);
    }

    pub fn remove_dependency(&mut self, id: &AssetId) -> bool {
        self.dependencies.remove(id)
    }

    pub fn remove_dependent(&mut self, id: &AssetId) -> bool {
        self.dependents.remove(id)
    }

    pub fn set_dependencies(&mut self, dependencies: HashSet<AssetId>) -> Vec<AssetId> {
        let removed = self
            .dependencies
            .difference(&dependencies)
            .copied()
            .collect::<Vec<_>>();
        self.dependencies = dependencies;

        removed
    }

    pub fn set_dependents(&mut self, dependents: HashSet<AssetId>) -> Vec<AssetId> {
        let removed = self
            .dependents
            .difference(&dependents)
            .copied()
            .collect::<Vec<_>>();

        self.dependents = dependents;

        removed
    }
}

impl ToBytes for DependencyInfo {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let dependencies = self.dependents.to_bytes();
        bytes.extend(dependencies.len().to_bytes());
        bytes.extend(dependencies);

        let dependents = self.dependents.to_bytes();
        bytes.extend(dependents.len().to_bytes());
        bytes.extend(dependents);

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut offset = 0;

        let dependencies_len = usize::from_bytes(&bytes[offset..(offset + 8)])?;
        offset += 8;

        let dependencies = HashSet::from_bytes(&bytes[offset..(offset + dependencies_len)])?;
        offset += dependencies_len;

        let dependents_len = usize::from_bytes(&bytes[offset..(offset + 8)])?;
        offset += 8;

        let dependents = HashSet::from_bytes(&bytes[offset..(offset + dependents_len)])?;

        Some(DependencyInfo {
            dependencies,
            dependents,
        })
    }
}

#[derive(Debug, Default)]
pub struct DependencyMap {
    infos: DenseMap<AssetId, DependencyInfo>,
}

impl DependencyMap {
    pub fn new() -> Self {
        DependencyMap {
            infos: DenseMap::new(),
        }
    }

    pub fn get(&self, id: &AssetId) -> Option<&DependencyInfo> {
        self.infos.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut DependencyInfo> {
        self.infos.get_mut(id)
    }

    pub fn insert(&mut self, id: AssetId, info: DependencyInfo) {
        self.infos.insert(id, info);
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<DependencyInfo> {
        self.infos.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &DependencyInfo)> {
        self.infos.iter()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (AssetId, DependencyInfo)> + '_ {
        self.infos.drain()
    }

    pub fn save(&self, config: &AssetConfig) -> std::io::Result<()> {
        for (id, info) in self.infos.iter() {
            let path = config.dependency_map().join(id.to_string());
            std::fs::write(path, info.to_bytes())?;
        }

        Ok(())
    }
}

impl ToBytes for DependencyMap {
    fn to_bytes(&self) -> Vec<u8> {
        self.infos.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let infos = DenseMap::from_bytes(bytes)?;

        Some(DependencyMap { infos })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStatus {
    None,
    Loading,
    Done,
    Failed,
}

pub struct AssetLibrary {
    sources: DenseMap<PathBuf, SourceInfo>,
    artifacts: DenseMap<AssetId, ArtifactMeta>,
    status: DenseMap<AssetId, AssetStatus>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        AssetLibrary {
            sources: DenseMap::default(),
            artifacts: DenseMap::default(),
            status: DenseMap::default(),
        }
    }

    pub fn source(&self, path: &PathBuf) -> Option<&SourceInfo> {
        self.sources.get(path)
    }

    pub fn artifact(&self, id: &AssetId) -> Option<&ArtifactMeta> {
        self.artifacts.get(id)
    }

    pub fn status(&self, id: &AssetId) -> AssetStatus {
        *self.status.get(id).unwrap_or(&AssetStatus::None)
    }

    pub fn insert_source(&mut self, path: PathBuf, info: SourceInfo) {
        self.sources.insert(path, info);
    }

    pub fn insert_artifact(&mut self, id: AssetId, meta: ArtifactMeta) {
        self.artifacts.insert(id, meta);
    }

    pub fn remove_source(&mut self, path: &PathBuf) -> Option<SourceInfo> {
        self.sources.remove(path)
    }

    pub fn remove_artifact(&mut self, id: &AssetId) -> Option<ArtifactMeta> {
        self.artifacts.remove(id)
    }

    pub fn set_status(&mut self, id: AssetId, status: AssetStatus) -> Option<AssetStatus> {
        self.status.insert(id, status)
    }

    pub fn save(&self, config: &AssetConfig) -> std::io::Result<()> {
        std::fs::write(config.sources_db(), self.sources.to_bytes())?;
        std::fs::write(config.artifacts_db(), self.artifacts.to_bytes())?;
        Ok(())
    }

    pub fn load(config: &AssetConfig) -> std::io::Result<Self> {
        let sources = DenseMap::from_bytes(&std::fs::read(config.sources_db())?).ok_or(
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to load sources"),
        )?;
        let artifacts = DenseMap::from_bytes(&std::fs::read(config.artifacts_db())?).ok_or(
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to load artifacts"),
        )?;

        Ok(AssetLibrary {
            sources,
            artifacts,
            status: DenseMap::new(),
        })
    }
}

pub struct AssetLibraryRef<'a>(RwLockReadGuard<'a, AssetLibrary>);

impl std::ops::Deref for AssetLibraryRef<'_> {
    type Target = AssetLibrary;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<RwLockReadGuard<'a, AssetLibrary>> for AssetLibraryRef<'a> {
    fn from(guard: RwLockReadGuard<'a, AssetLibrary>) -> Self {
        AssetLibraryRef(guard)
    }
}

pub struct AssetLibraryRefMut<'a>(RwLockWriteGuard<'a, AssetLibrary>);

impl<'a> From<RwLockWriteGuard<'a, AssetLibrary>> for AssetLibraryRefMut<'a> {
    fn from(guard: RwLockWriteGuard<'a, AssetLibrary>) -> Self {
        AssetLibraryRefMut(guard)
    }
}

impl std::ops::Deref for AssetLibraryRefMut<'_> {
    type Target = AssetLibrary;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for AssetLibraryRefMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub type AssetLibraryShared = RwLock<AssetLibrary>;
