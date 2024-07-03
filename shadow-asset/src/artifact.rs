use crate::{
    asset::{AssetId, AssetMetadata, Settings},
    bytes::ToBytes,
};

pub struct Header {
    asset: usize,
    settings: usize,
}

impl Header {
    pub fn new(asset: usize, settings: usize) -> Self {
        Header { asset, settings }
    }

    pub fn asset(&self) -> usize {
        self.asset
    }

    pub fn settings(&self) -> usize {
        self.settings
    }
}

impl ToBytes for Header {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.asset.to_bytes();
        bytes.extend_from_slice(&self.settings.to_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let asset = usize::from_bytes(bytes)?;
        let settings = usize::from_bytes(&bytes[8..])?;

        Some(Header::new(asset, settings))
    }
}

pub struct Artifact {
    header: Header,
    data: Vec<u8>,
}

impl Artifact {
    pub fn new(asset: &[u8], settings: &[u8]) -> Self {
        let mut data = asset.to_vec();
        let asset = data.len();

        let settings_len = settings.len();
        data.extend_from_slice(&settings_len.to_bytes());
        data.extend_from_slice(settings);

        let header = Header::new(asset, settings_len);
        Self { header, data }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn asset(&self) -> &[u8] {
        &self.data[..self.header.asset]
    }

    pub fn settings(&self) -> &[u8] {
        &self.data[self.header.asset..(self.header.settings + self.header.asset)]
    }
}

impl ToBytes for Artifact {
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
        let header = Header::from_bytes(&bytes[8..(8 + len)])?;
        let data = bytes[8 + len..].to_vec();

        Some(Artifact { header, data })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataBlock {
    id: AssetId,
    data: Vec<u8>,
}

impl MetadataBlock {
    pub fn new(id: AssetId, data: Vec<u8>) -> Self {
        MetadataBlock { id, data }
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

    pub fn into_metadata<S: Settings>(&self) -> Option<AssetMetadata<S>> {
        let data = String::from_utf8(self.data.clone()).ok()?;
        toml::from_str::<AssetMetadata<S>>(&data).ok()
    }
}

impl<S: Settings> From<AssetMetadata<S>> for MetadataBlock {
    fn from(value: AssetMetadata<S>) -> Self {
        let data = toml::to_string(&value).unwrap().into_bytes();
        MetadataBlock {
            id: value.id(),
            data,
        }
    }
}

impl<S: Settings> From<&AssetMetadata<S>> for MetadataBlock {
    fn from(value: &AssetMetadata<S>) -> Self {
        let data = toml::to_string(value).unwrap().into_bytes();
        MetadataBlock {
            id: value.id(),
            data,
        }
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
