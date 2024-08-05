use crate::{
    artifact::{Artifact, ArtifactHeader, ArtifactMeta},
    asset::{AssetId, AssetSettings, Settings},
    bytes::IntoBytes,
};
use std::{
    error::Error,
    fmt::Debug,
    path::{Path, PathBuf},
};

pub mod local;
pub mod vfs;

#[derive(Debug)]
pub enum AssetIoError {
    NotFound(PathBuf),
    Io(std::io::Error),
    Http(u16),
    Other(Box<dyn Error + Send + Sync + 'static>),
}

impl AssetIoError {
    pub fn other<E: Error + Send + Sync + 'static>(err: E) -> Self {
        AssetIoError::Other(Box::new(err))
    }
}

impl std::fmt::Display for AssetIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AssetIoError::NotFound(path) => write!(f, "Asset not found: {:?}", path),
            AssetIoError::Io(err) => write!(f, "I/O error: {}", err),
            AssetIoError::Http(status) => write!(f, "HTTP error: {}", status),
            AssetIoError::Other(err) => write!(f, "Error: {}", err),
        }
    }
}

impl Error for AssetIoError {}

impl From<PathBuf> for AssetIoError {
    fn from(path: PathBuf) -> Self {
        AssetIoError::NotFound(path)
    }
}

impl From<std::io::Error> for AssetIoError {
    fn from(err: std::io::Error) -> Self {
        AssetIoError::Io(err)
    }
}

impl From<std::io::ErrorKind> for AssetIoError {
    fn from(kind: std::io::ErrorKind) -> Self {
        AssetIoError::Io(std::io::Error::from(kind))
    }
}

impl From<toml::ser::Error> for AssetIoError {
    fn from(err: toml::ser::Error) -> Self {
        AssetIoError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }
}

impl From<toml::de::Error> for AssetIoError {
    fn from(err: toml::de::Error) -> Self {
        AssetIoError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }
}

impl From<u16> for AssetIoError {
    fn from(status: u16) -> Self {
        AssetIoError::Http(status)
    }
}

impl From<Box<dyn Error + Send + Sync + 'static>> for AssetIoError {
    fn from(err: Box<dyn Error + Send + Sync + 'static>) -> Self {
        AssetIoError::Other(err)
    }
}

pub type Result<T> = std::result::Result<T, AssetIoError>;

pub trait AssetReader {
    fn path(&self) -> &Path;
    fn read(&mut self, amount: usize) -> Result<usize>;
    fn read_to_end(&mut self) -> Result<usize>;
    fn read_dir(&self) -> Result<Vec<PathBuf>>;
    fn bytes(&self) -> &[u8];
    fn flush(&mut self) -> Result<Vec<u8>>;
}

impl AssetReader for Box<dyn AssetReader> {
    fn path(&self) -> &Path {
        self.as_ref().path()
    }

    fn read(&mut self, amount: usize) -> Result<usize> {
        self.as_mut().read(amount)
    }

    fn read_to_end(&mut self) -> Result<usize> {
        self.as_mut().read_to_end()
    }

    fn read_dir(&self) -> Result<Vec<PathBuf>> {
        self.as_ref().read_dir()
    }

    fn bytes(&self) -> &[u8] {
        self.as_ref().bytes()
    }

    fn flush(&mut self) -> Result<Vec<u8>> {
        self.as_mut().flush()
    }
}

pub trait AssetWriter {
    fn path(&self) -> &Path;
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn create_dir(&mut self) -> Result<()>;
    fn remove_file(&mut self) -> Result<()>;
    fn remove_dir(&mut self) -> Result<()>;
    fn flush(&mut self) -> Result<Vec<u8>>;
    fn clear(&mut self);
}

impl AssetWriter for Box<dyn AssetWriter> {
    fn path(&self) -> &Path {
        self.as_ref().path()
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.as_mut().write(buf)
    }

    fn create_dir(&mut self) -> Result<()> {
        self.as_mut().create_dir()
    }

    fn remove_file(&mut self) -> Result<()> {
        self.as_mut().remove_file()
    }

    fn remove_dir(&mut self) -> Result<()> {
        self.as_mut().remove_dir()
    }

    fn flush(&mut self) -> Result<Vec<u8>> {
        self.as_mut().flush()
    }

    fn clear(&mut self) {
        self.as_mut().clear()
    }
}

pub trait AssetFileSystem: Debug + 'static {
    fn is_dir(&self, path: &Path) -> bool;
    fn exists(&self, path: &Path) -> bool;
    fn reader(&self, path: &Path) -> Box<dyn AssetReader>;
    fn writer(&self, path: &Path) -> Box<dyn AssetWriter>;
}

pub struct FileSystem {
    root: PathBuf,
    inner: Box<dyn AssetFileSystem>,
}

impl FileSystem {
    pub fn new<F: AssetFileSystem>(path: impl AsRef<Path>, fs: F) -> Self {
        Self {
            root: path.as_ref().to_path_buf(),
            inner: Box::new(fs),
        }
    }

    pub fn is_dir(&self, path: impl AsRef<Path>) -> bool {
        self.inner.is_dir(path.as_ref())
    }

    pub fn exists(&self, path: impl AsRef<Path>) -> bool {
        self.inner.exists(path.as_ref())
    }

    pub fn reader(&self, path: impl AsRef<Path>) -> Box<dyn AssetReader> {
        self.inner.reader(path.as_ref())
    }

    pub fn writer(&self, path: impl AsRef<Path>) -> Box<dyn AssetWriter> {
        self.inner.writer(path.as_ref())
    }
}

impl std::fmt::Debug for FileSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Root: {:?} ", &self.root)?;
        writeln!(f, "{:?}", &self.inner)
    }
}

pub struct AssetIo {
    fs: FileSystem,
}

impl AssetIo {
    pub fn new<F: AssetFileSystem>(path: impl AsRef<Path>, fs: F) -> Self {
        Self {
            fs: FileSystem::new::<F>(path, fs),
        }
    }

    pub fn filesystem(&self) -> &FileSystem {
        &self.fs
    }

    pub fn root(&self) -> &Path {
        &self.fs.root
    }

    pub fn assets(&self) -> PathBuf {
        self.fs.root.join("assets")
    }

    pub fn cache(&self) -> PathBuf {
        self.fs.root.join(".cache")
    }

    pub fn temp(&self) -> PathBuf {
        self.cache().join("temp")
    }

    pub fn artifact(&self, id: AssetId) -> PathBuf {
        self.artifacts().join(id.to_string())
    }

    pub fn artifacts(&self) -> PathBuf {
        self.cache().join("artifacts")
    }

    pub fn reader(&self, path: impl AsRef<Path>) -> Box<dyn AssetReader> {
        self.fs.reader(path.as_ref())
    }

    pub fn writer(&self, path: impl AsRef<Path>) -> Box<dyn AssetWriter> {
        self.fs.writer(path.as_ref())
    }

    pub fn checksum(&self, asset: &[u8], settings: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(asset);
        hasher.update(settings);
        hasher.finalize()
    }

    pub fn remove_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut writer = self.writer(path);
        writer.remove_file()
    }

    pub fn remove_dir(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut writer = self.writer(path);
        writer.remove_dir()
    }

    pub fn load_metadata<S: Settings>(&self, path: impl AsRef<Path>) -> Result<AssetSettings<S>> {
        let mut reader = self.reader(path);
        reader.read_to_end()?;

        let meta = String::from_utf8(reader.flush()?).map_err(AssetIoError::other)?;
        toml::from_str(&meta).map_err(AssetIoError::from)
    }

    pub fn save_metadata<S: Settings>(
        &self,
        path: impl AsRef<Path>,
        settings: &AssetSettings<S>,
    ) -> Result<String> {
        let mut writer = self.writer(path);
        let meta = toml::to_string(settings).map_err(AssetIoError::from)?;

        writer.write(meta.as_bytes())?;
        writer.flush()?;
        Ok(meta)
    }

    pub fn load_artifact_meta(&self, id: AssetId) -> Result<ArtifactMeta> {
        let path = self.artifact(id);
        if !self.filesystem().exists(&path) {
            return Err(AssetIoError::NotFound(path));
        }

        let mut reader = self.reader(path);
        reader.read(ArtifactHeader::SIZE)?;

        let header = ArtifactHeader::from_bytes(reader.bytes())
            .ok_or(AssetIoError::from(std::io::ErrorKind::InvalidData))?;

        reader.read(header.meta())?;

        let meta_bytes =
            &reader.bytes()[ArtifactHeader::SIZE..ArtifactHeader::SIZE + header.meta()];

        ArtifactMeta::from_bytes(meta_bytes)
            .ok_or(AssetIoError::from(std::io::ErrorKind::InvalidData))
    }

    pub fn load_artifact(&self, id: AssetId) -> Result<Artifact> {
        let path = self.artifact(id);
        if !self.filesystem().exists(&path) {
            return Err(AssetIoError::NotFound(path));
        }

        let mut reader = self.reader(path);
        reader.read_to_end()?;

        Artifact::from_bytes(&reader.flush()?)
            .ok_or(AssetIoError::from(std::io::ErrorKind::InvalidData))
    }
}

pub trait PathExt {
    fn ext(&self) -> Option<&str>;
    fn append_ext(&self, ext: &str) -> PathBuf;
}

impl<T: AsRef<Path>> PathExt for T {
    fn ext(&self) -> Option<&str> {
        self.as_ref().extension().and_then(|ext| ext.to_str())
    }

    fn append_ext(&self, ext: &str) -> PathBuf {
        let path = self.as_ref().to_path_buf();
        format!("{}.{}", path.display(), ext).into()
    }
}
