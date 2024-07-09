use crate::{
    asset::{AssetId, AssetMetadata, AssetType, Settings},
    bytes::ToBytes,
};
use std::{collections::HashSet, mem::size_of, path::PathBuf};

pub struct Header {
    asset: usize,
    dependencies: usize,
}

impl Header {
    pub fn new(asset: usize, dependencies: usize) -> Self {
        Header {
            asset,
            dependencies,
        }
    }

    pub fn asset(&self) -> usize {
        self.asset
    }

    pub fn dependencies(&self) -> usize {
        self.dependencies
    }
}

impl ToBytes for Header {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend(self.asset.to_bytes());
        bytes.extend(self.dependencies.to_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let asset = usize::from_bytes(bytes)?;
        let dependencies = usize::from_bytes(&bytes[size_of::<usize>()..])?;

        Some(Header {
            asset,
            dependencies,
        })
    }
}

pub struct Artifact {
    header: Header,
    data: Vec<u8>,
}

impl Artifact {
    pub fn new(asset: &[u8], dependencies: Vec<AssetId>) -> Self {
        let asset_len = asset.len();

        let mut data = asset.to_vec();
        data.extend(dependencies.to_bytes());

        let dep_len = data.len() - asset_len;
        let header = Header::new(asset_len, dep_len);

        Self { header, data }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn asset(&self) -> &[u8] {
        &self.data[..self.header.asset()]
    }

    pub fn dependencies(&self) -> Vec<AssetId> {
        Vec::<AssetId>::from_bytes(&self.data[self.header.asset()..]).unwrap_or_default()
    }
}

impl ToBytes for Artifact {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.data);

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let header = Header::from_bytes(bytes)?;
        let data = bytes[size_of::<Header>()..].to_vec();

        Some(Artifact { header, data })
    }
}

#[derive(Debug)]
pub struct ArtifactMeta {
    id: AssetId,
    ty: AssetType,
    filepath: PathBuf,
    dependencies: HashSet<AssetId>,
    dependents: HashSet<AssetId>,
}

impl ArtifactMeta {
    pub fn new(
        id: AssetId,
        ty: AssetType,
        filepath: PathBuf,
        dependencies: HashSet<AssetId>,
    ) -> Self {
        ArtifactMeta {
            id,
            ty,
            filepath,
            dependencies,
            dependents: HashSet::new(),
        }
    }

    pub fn empty() -> Self {
        ArtifactMeta {
            id: AssetId::new(0),
            ty: AssetType::of::<()>(),
            filepath: PathBuf::new(),
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn filepath(&self) -> &PathBuf {
        &self.filepath
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn dependents(&self) -> &HashSet<AssetId> {
        &self.dependents
    }

    pub fn is_empty(&self) -> bool {
        self.id == AssetId::new(0) && self.ty == AssetType::of::<()>()
    }

    pub fn set_dependencies(&mut self, dependencies: HashSet<AssetId>) {
        self.dependencies = dependencies;
    }

    pub fn set_dependents(&mut self, dependents: HashSet<AssetId>) {
        self.dependents = dependents;
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn add_dependent(&mut self, id: AssetId) {
        self.dependents.insert(id);
    }

    pub fn remove_dependency(&mut self, id: &AssetId) {
        self.dependencies.remove(id);
    }

    pub fn remove_dependent(&mut self, id: &AssetId) {
        self.dependents.remove(id);
    }
}

impl ToBytes for ArtifactMeta {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.id.to_bytes();
        bytes.extend(self.ty.to_bytes());

        let path = self.filepath.to_bytes();
        bytes.extend(path.len().to_bytes());
        bytes.extend(path);

        let deps = self.dependencies.to_bytes();
        bytes.extend(deps.len().to_bytes());
        bytes.extend(deps);

        let deps = self.dependents.to_bytes();
        bytes.extend(deps);

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let id = AssetId::from_bytes(bytes)?;
        let ty = AssetType::from_bytes(&bytes[size_of::<AssetId>()..])?;
        offset += size_of::<AssetId>() + size_of::<AssetType>();

        let path_len = usize::from_bytes(&bytes[offset..(offset + 8)])?;
        offset += 8;

        let filepath = PathBuf::from_bytes(&bytes[offset..(offset + path_len)])?;
        offset += path_len;

        let deps_len = usize::from_bytes(&bytes[offset..(offset + 8)])?;
        offset += 8;

        let dependencies = HashSet::<AssetId>::from_bytes(&bytes[offset..(offset + deps_len)])?;
        offset += deps_len;

        let dependents = HashSet::<AssetId>::from_bytes(&bytes[offset..])?;

        Some(ArtifactMeta {
            id,
            ty,
            filepath,
            dependencies,
            dependents,
        })
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
