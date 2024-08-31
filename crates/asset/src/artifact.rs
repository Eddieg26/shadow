use crate::asset::{Asset, AssetId, AssetType};
use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactMeta {
    pub id: AssetId,
    pub ty: AssetType,
    pub checksum: u32,
    pub dependencies: HashSet<AssetId>,
}

impl ArtifactMeta {
    pub fn new<A: Asset>(id: AssetId, checksum: u32, dependencies: HashSet<AssetId>) -> Self {
        Self {
            id,
            ty: AssetType::of::<A>(),
            checksum,
            dependencies,
        }
    }

    pub fn with_type(
        id: AssetId,
        ty: AssetType,
        checksum: u32,
        dependencies: HashSet<AssetId>,
    ) -> Self {
        Self {
            id,
            ty,
            checksum,
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

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactHeader {
    meta: usize,
    asset: usize,
}

impl ArtifactHeader {
    pub const SIZE: usize = std::mem::size_of::<ArtifactHeader>();

    pub fn new(meta: usize, asset: usize) -> Self {
        Self { meta, asset }
    }

    pub fn meta(&self) -> usize {
        self.meta
    }

    pub fn asset(&self) -> usize {
        self.asset
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
    pub header: ArtifactHeader,
    pub meta: ArtifactMeta,
    data: Vec<u8>,
}

impl Artifact {
    pub fn new(asset: &[u8], meta: ArtifactMeta) -> Self {
        let mut data = Vec::new();
        let meta_bytes = bincode::serialize(&meta).unwrap();
        data.extend(meta_bytes);
        data.extend_from_slice(asset);

        let meta_len = data.len() - asset.len();
        let header = ArtifactHeader::new(meta_len, asset.len());

        Self { header, meta, data }
    }

    pub fn header(&self) -> &ArtifactHeader {
        &self.header
    }

    pub fn meta(&self) -> &ArtifactMeta {
        &self.meta
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn asset(&self) -> &[u8] {
        &self.data[self.header.meta()..self.header.meta() + self.header.asset()]
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.meta.dependencies()
    }
}
