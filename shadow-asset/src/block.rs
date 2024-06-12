use crate::{
    asset::{Asset, AssetId, AssetMetadata, Settings},
    bytes::ToBytes,
};

pub struct BlockHeader {
    asset: usize,
    settings: usize,
    dependencies: usize,
}

impl BlockHeader {
    pub fn new(asset: usize, settings: usize, dependencies: usize) -> Self {
        BlockHeader {
            asset,
            settings,
            dependencies,
        }
    }

    pub fn asset(&self) -> usize {
        self.asset
    }

    pub fn settings(&self) -> usize {
        self.settings
    }

    pub fn dependencies(&self) -> usize {
        self.dependencies
    }
}

impl ToBytes for BlockHeader {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.asset.to_bytes();
        bytes.extend_from_slice(&self.settings.to_bytes());
        bytes.extend_from_slice(&self.dependencies.to_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let asset = usize::from_bytes(bytes)?;
        let settings = usize::from_bytes(&bytes[8..])?;
        let dependencies = usize::from_bytes(&bytes[16..])?;

        Some(BlockHeader::new(asset, settings, dependencies))
    }
}

pub struct AssetBlock {
    header: BlockHeader,
    data: Vec<u8>,
}

impl AssetBlock {
    pub fn new<A: Asset, S: Settings>(asset: &A, settings: &S, dependencies: Vec<AssetId>) -> Self {
        let mut data = asset.to_bytes();
        let asset = data.len();

        let settings_bytes = settings.to_bytes();
        let settings = settings_bytes.len();
        data.extend_from_slice(&settings_bytes);

        let dependency_bytes = dependencies.to_bytes();
        let dependencies = dependency_bytes.len();
        data.extend_from_slice(&dependency_bytes);

        let header = BlockHeader::new(asset, settings, dependencies);
        Self { header, data }
    }

    pub fn header(&self) -> &BlockHeader {
        &self.header
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn asset<A: Asset>(&self) -> Option<A> {
        A::from_bytes(&self.data[..self.header.asset])
    }

    pub fn settings<S: Settings>(&self) -> Option<S> {
        S::from_bytes(&self.data[self.header.asset..(self.header.settings + self.header.asset)])
    }

    pub fn dependencies(&self) -> Vec<AssetId> {
        Vec::from_bytes(&self.data[(self.header.settings + self.header.asset)..])
            .unwrap_or_default()
    }

    pub fn take<A: Asset, S: Settings>(self) -> (Option<A>, Option<S>, Vec<AssetId>) {
        let asset = self.asset();
        let settings = self.settings();
        let dependencies = self.dependencies();

        (asset, settings, dependencies)
    }
}

impl ToBytes for AssetBlock {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let header = self.header.to_bytes();
        bytes.extend(header.len().to_bytes());
        bytes.extend(header);

        bytes.extend_from_slice(&self.data);

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let len = usize::from_bytes(bytes)?;
        let header = BlockHeader::from_bytes(&bytes[8..(8 + len)])?;
        let data = bytes[8 + len..].to_vec();

        Some(AssetBlock { header, data })
    }
}

pub struct MetadataBlock {
    id: AssetId,
    data: Vec<u8>,
}

impl MetadataBlock {
    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn take(self) -> (AssetId, Vec<u8>) {
        (self.id, self.data)
    }

    pub fn into<S: Settings>(self) -> Option<AssetMetadata<S>> {
        let id = self.id;
        let settings = S::from_bytes(&self.data)?;

        Some(AssetMetadata::new(id, settings))
    }
}

impl ToBytes for MetadataBlock {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.id.to_bytes();
        bytes.extend_from_slice(&self.data);

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(bytes)?;
        let data = bytes[..].to_vec();

        Some(MetadataBlock { id, data })
    }
}
