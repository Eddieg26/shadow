use super::{bytes::IntoBytes, Asset, AssetId, AssetType};
use std::{collections::HashSet, io::Read, path::Path};

#[derive(Clone, Debug, Default)]
pub struct ArtifactMeta {
    id: AssetId,
    ty: AssetType,
    checksum: u32,
    modified: u64,
    pub dependencies: HashSet<AssetId>,
}

impl ArtifactMeta {
    pub fn new(
        id: AssetId,
        ty: AssetType,
        checksum: u32,
        modified: u64,
        dependencies: HashSet<AssetId>,
    ) -> Self {
        ArtifactMeta {
            id,
            ty,
            checksum,
            modified,
            dependencies,
        }
    }

    pub fn from<A: Asset>(
        id: AssetId,
        checksum: u32,
        modified: u64,
        dependencies: HashSet<AssetId>,
    ) -> Self {
        ArtifactMeta {
            id,
            ty: AssetType::from::<A>(),
            checksum,
            modified,
            dependencies,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn checksum(&self) -> u32 {
        self.checksum
    }

    pub fn modified(&self) -> u64 {
        self.modified
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn set_dependencies(&mut self, dependencies: HashSet<AssetId>) {
        self.dependencies = dependencies;
    }

    pub fn removed_dependencies(&self, other: &ArtifactMeta) -> HashSet<AssetId> {
        self.dependencies
            .difference(&other.dependencies)
            .cloned()
            .collect()
    }
}

impl IntoBytes for ArtifactMeta {
    fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.into_bytes());
        bytes.extend_from_slice(&self.ty.into_bytes());
        bytes.extend_from_slice(&self.checksum.into_bytes());
        bytes.extend_from_slice(&self.modified.into_bytes());

        let deps = self.dependencies.into_bytes();
        bytes.extend_from_slice(&deps.len().into_bytes());
        bytes.extend_from_slice(&deps);

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(&bytes[0..8])?;
        let ty = AssetType::from_bytes(&bytes[8..16])?;
        let checksum = u32::from_bytes(&bytes[16..20])?;
        let modified = u64::from_bytes(&bytes[20..28])?;
        let len = usize::from_bytes(&bytes[28..36])?;
        let dependencies = HashSet::from_bytes(&bytes[36..36 + len])?;
        Some(ArtifactMeta {
            id,
            ty,
            checksum,
            modified,
            dependencies,
        })
    }
}

pub struct Artifact {
    pub meta: ArtifactMeta,
    asset: Vec<u8>,
}

impl Artifact {
    pub fn new(meta: ArtifactMeta, asset: Vec<u8>) -> Self {
        Artifact { meta, asset }
    }

    pub fn meta(&self) -> &ArtifactMeta {
        &self.meta
    }

    pub fn asset(&self) -> &[u8] {
        &self.asset
    }

    pub fn meta_mut(&mut self) -> &mut ArtifactMeta {
        &mut self.meta
    }

    pub fn read_meta(path: &Path) -> std::io::Result<ArtifactMeta> {
        let mut file = std::fs::File::open(path)?;
        let mut buffer = [0u8; 8];
        file.read(&mut buffer)?;
        let len = usize::from_bytes(&mut buffer)
            .ok_or::<std::io::Error>(std::io::ErrorKind::InvalidData.into())?;
        let mut bytes = vec![0u8; len];
        file.read_exact(&mut bytes)?;
        ArtifactMeta::from_bytes(&bytes).ok_or(std::io::ErrorKind::InvalidData.into())
    }
}

impl IntoBytes for Artifact {
    fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let meta = self.meta.into_bytes();
        bytes.extend_from_slice(&meta.len().into_bytes());
        bytes.extend_from_slice(&meta);
        bytes.extend_from_slice(&self.asset);
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let len = usize::from_bytes(bytes)?;
        let meta = ArtifactMeta::from_bytes(&bytes[8..8 + len])?;
        let asset = bytes[8 + len..].to_vec();
        Some(Artifact { meta, asset })
    }
}
