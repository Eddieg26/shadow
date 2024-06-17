use crate::{
    asset::{Asset, AssetId, AssetMetadata, Settings},
    bytes::ToBytes,
};

pub struct BlockHeader {
    asset: usize,
    settings: usize,
}

impl BlockHeader {
    pub fn new(asset: usize, settings: usize) -> Self {
        BlockHeader { asset, settings }
    }

    pub fn asset(&self) -> usize {
        self.asset
    }

    pub fn settings(&self) -> usize {
        self.settings
    }
}

impl ToBytes for BlockHeader {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.asset.to_bytes();
        bytes.extend_from_slice(&self.settings.to_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let asset = usize::from_bytes(bytes)?;
        let settings = usize::from_bytes(&bytes[8..])?;

        Some(BlockHeader::new(asset, settings))
    }
}

pub struct AssetBlock {
    header: BlockHeader,
    data: Vec<u8>,
}

impl AssetBlock {
    pub fn new<A: Asset, S: Settings>(asset: &A, settings: &S) -> Self {
        let mut data = asset.to_bytes();
        let asset = data.len();

        let settings_bytes = settings.to_bytes();
        let settings = settings_bytes.len();
        data.extend_from_slice(&settings_bytes);

        let header = BlockHeader::new(asset, settings);
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

    pub fn take<A: Asset, S: Settings>(self) -> (Option<A>, Option<S>) {
        let asset = self.asset();
        let settings = self.settings();

        (asset, settings)
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
    pub fn new(id: AssetId, data: Vec<u8>) -> Self {
        MetadataBlock { id, data }
    }

    pub fn from_data(data: Vec<u8>) -> Option<Self> {
        let id = AssetId::from_bytes(&data[..])?;
        let data = data[id.to_bytes().len()..].to_vec();

        Some(MetadataBlock { id, data })
    }

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

    pub fn parse_asset_id(data: &[u8]) -> Option<AssetId> {
        AssetId::from_bytes(&data[..])
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
