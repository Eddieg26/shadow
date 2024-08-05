use crate::{
    asset::{Asset, AssetId, AssetType},
    bytes::IntoBytes,
};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ArtifactMeta {
    id: AssetId,
    ty: AssetType,
    checksum: u32,
    dependencies: HashSet<AssetId>,
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

impl IntoBytes for ArtifactMeta {
    fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.into_bytes());
        bytes.extend_from_slice(&self.ty.into_bytes());
        bytes.extend_from_slice(&self.checksum.into_bytes());

        let deps = self.dependencies.into_bytes();
        bytes.extend_from_slice(&deps.len().into_bytes());
        bytes.extend_from_slice(&deps);

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(&bytes[..8])?;
        let ty = AssetType::from_bytes(&bytes[8..12])?;
        let checksum = u32::from_bytes(&bytes[12..16])?;
        let dependencies_len = usize::from_bytes(&bytes[16..24])?;
        let dependencies = HashSet::from_bytes(&bytes[24..24 + dependencies_len])?;

        Some(Self::with_type(id, ty, checksum, dependencies))
    }
}

#[derive(Debug, Clone)]
pub struct ArtifactHeader {
    meta: usize,
    settings: usize,
    asset: usize,
}

impl ArtifactHeader {
    pub const SIZE: usize = std::mem::size_of::<ArtifactHeader>();

    pub fn new(meta: usize, asset: usize, settings: usize) -> Self {
        Self {
            meta,
            settings,
            asset,
        }
    }

    pub fn meta(&self) -> usize {
        self.meta
    }

    pub fn settings(&self) -> usize {
        self.settings
    }

    pub fn asset(&self) -> usize {
        self.asset
    }
}

impl IntoBytes for ArtifactHeader {
    fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.meta.into_bytes());
        bytes.extend_from_slice(&self.settings.into_bytes());
        bytes.extend_from_slice(&self.asset.into_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let meta = usize::from_bytes(&bytes[..8])?;
        let settings = usize::from_bytes(&bytes[8..16])?;
        let asset = usize::from_bytes(&bytes[16..24])?;

        Some(Self::new(meta, asset, settings))
    }
}

pub struct Artifact {
    pub header: ArtifactHeader,
    pub meta: ArtifactMeta,
    data: Vec<u8>,
}

impl Artifact {
    pub fn new(asset: &[u8], settings: &[u8], meta: ArtifactMeta) -> Self {
        let mut data = Vec::new();
        let meta_bytes = meta.into_bytes();
        data.extend_from_slice(&meta.into_bytes());
        data.extend_from_slice(asset);
        data.extend_from_slice(settings);

        let header = ArtifactHeader::new(meta_bytes.len(), asset.len(), settings.len());

        Self { header, meta, data }
    }

    pub fn bytes(asset: &[u8], settings: &[u8], meta: &ArtifactMeta) -> Vec<u8> {
        let mut data = Vec::new();
        let meta_bytes = meta.into_bytes();
        data.extend_from_slice(&meta_bytes);
        data.extend_from_slice(asset);
        data.extend_from_slice(settings);

        let header = ArtifactHeader::new(meta_bytes.len(), asset.len(), settings.len());
        let mut bytes = header.into_bytes();
        bytes.extend(data);
        bytes
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

    pub fn settings(&self) -> &[u8] {
        &self.data[self.header.meta() + self.header.asset()..]
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.meta.dependencies()
    }
}

impl IntoBytes for Artifact {
    fn into_bytes(&self) -> Vec<u8> {
        Self::bytes(self.asset(), self.settings(), &self.meta)
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        const HEADER_SIZE: usize = std::mem::size_of::<ArtifactHeader>();
        let header = ArtifactHeader::from_bytes(&bytes[..HEADER_SIZE])?;
        let meta = ArtifactMeta::from_bytes(&bytes[HEADER_SIZE..HEADER_SIZE + header.meta()])?;
        let data = bytes[HEADER_SIZE..].to_vec();

        Some(Self { header, meta, data })
    }
}
