use crate::{
    asset::{Asset, AssetId, AssetMetadata, AssetPath, AssetType, Settings},
    bytes::AsBytes,
    object::AssetObject,
};
use std::{
    ffi::OsString,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct SourceInfo {
    id: AssetId,
    checksum: u64,
    modified: u64,
}

impl SourceInfo {
    pub fn new(id: AssetId, checksum: u64, modified: u64) -> Self {
        Self {
            id,
            checksum,
            modified,
        }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn checksum(&self) -> u64 {
        self.checksum
    }

    pub fn modified(&self) -> u64 {
        self.modified
    }

    pub fn modify(&mut self, checksum: u64, modified: u64) -> Self {
        Self::new(self.id, checksum, modified)
    }
}

impl AsBytes for SourceInfo {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend(&self.id.as_bytes());
        bytes.extend(&self.checksum.as_bytes());
        bytes.extend(&self.modified.as_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(bytes)?;
        let checksum = u64::from_bytes(&bytes[8..16])?;
        let modified = u64::from_bytes(&bytes[16..])?;

        Some(Self::new(id, checksum, modified))
    }
}

#[derive(Clone)]
pub struct ObjectInfo {
    path: PathBuf,
    ty: AssetType,
}

impl ObjectInfo {
    pub fn new<A: Asset>(path: impl Into<PathBuf>) -> Self {
        let ty = AssetType::of::<A>();
        Self {
            path: path.into(),
            ty,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn ty(&self) -> &AssetType {
        &self.ty
    }
}

impl AsBytes for ObjectInfo {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend(self.ty.as_bytes());
        bytes.extend(self.path.clone().into_os_string().as_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let ty = AssetType::from_bytes(bytes)?;
        let path = OsString::from_bytes(&bytes[8..])?;
        Some(Self {
            ty,
            path: path.into(),
        })
    }
}

#[derive(Clone)]
pub struct AssetConfig {
    assets: PathBuf,
    cache: PathBuf,
    objects: PathBuf,
    imports: PathBuf,
    sources: PathBuf,
}

impl AssetConfig {
    pub fn new(assets: impl Into<PathBuf>, cache: impl Into<PathBuf>) -> Self {
        let assets = assets.into();
        let cache: PathBuf = cache.into();
        let objects = cache.join("objects");
        let imports: PathBuf = cache.join("metas");
        let sources = cache.join("sources");
        Self {
            assets,
            cache,
            objects,
            imports,
            sources,
        }
    }

    pub fn init_paths(&self) -> Result<(), io::Error> {
        std::fs::create_dir_all(&self.assets)?;
        std::fs::create_dir_all(&self.cache)?;
        std::fs::create_dir(&self.objects)?;
        std::fs::create_dir(&self.imports)?;
        std::fs::create_dir(&self.sources)
    }

    pub fn assets(&self) -> &Path {
        &self.assets
    }

    pub fn cache(&self) -> &Path {
        &self.cache
    }

    pub fn objects(&self) -> &Path {
        &self.objects
    }

    pub fn imports(&self) -> &Path {
        &self.imports
    }

    pub fn sources(&self) -> &Path {
        &self.sources
    }

    pub fn metadata<S: Settings>(&self, path: &Path) -> Result<AssetMetadata<S>, io::Error> {
        let path = path.with_extension(".meta");
        let bytes = std::fs::read(&path)?;
        AssetMetadata::<S>::from_bytes(&bytes).ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Failed to load metadata.",
        ))
    }

    pub fn source(&self, path: &Path) -> Result<SourceInfo, io::Error> {
        let path = self.source_path(path);
        let bytes = std::fs::read(&path)?;
        SourceInfo::from_bytes(&bytes).ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Failed to load source info.",
        ))
    }

    pub fn import(&self, id: &AssetId) -> Result<ObjectInfo, io::Error> {
        let filename = id.to_string();
        let path = self.imports.join(&filename).join("obj.info");
        let bytes = std::fs::read(&path)?;
        ObjectInfo::from_bytes(&bytes).ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Failed to load object info.",
        ))
    }

    pub fn object<A: Asset, S: Settings>(
        &self,
        path: &AssetPath,
    ) -> Result<AssetObject<A, S>, io::Error> {
        let filename = match path {
            AssetPath::Id(id) => id.to_string(),
            AssetPath::Path(path) => self.source(path)?.id.to_string(),
        };

        let path = self.objects.join(filename);
        let bytes = std::fs::read(&path)?;
        AssetObject::<A, S>::from_bytes(&bytes).ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Failed to load asset object.",
        ))
    }

    pub fn save_metadata<S: Settings>(
        &self,
        path: &Path,
        metadata: &AssetMetadata<S>,
    ) -> Result<(), io::Error> {
        let path = path.with_extension(".meta");
        let bytes = metadata.as_bytes();
        std::fs::write(&path, &bytes)
    }

    pub fn save_source(&self, path: &Path, source: SourceInfo) -> Result<(), io::Error> {
        let path = self.source_path(path);
        let bytes = source.as_bytes();
        std::fs::write(&path, &bytes)
    }

    pub fn save_import(&self, id: &AssetId, import: ObjectInfo) -> Result<(), io::Error> {
        let filename = id.to_string();
        let path = self.imports.join(&filename).join("obj.info");
        std::fs::create_dir_all(&path)?;
        let bytes = import.as_bytes();
        std::fs::write(&path, &bytes)
    }

    pub fn save_object<A: Asset, S: Settings>(
        &self,
        id: &AssetId,
        object: &AssetObject<A, S>,
    ) -> Result<(), io::Error> {
        let filename = id.to_string();
        let path = self.objects.join(&filename);
        let bytes = object.as_bytes();
        std::fs::write(&path, &bytes)
    }

    fn source_path(&self, path: &Path) -> PathBuf {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        let filename = hasher.finish().to_string();
        self.sources.join(&filename)
    }
}
