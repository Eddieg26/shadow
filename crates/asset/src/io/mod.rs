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
    fn read_to_string(&mut self) -> Result<String> {
        self.read_to_end()?;
        String::from_utf8(self.flush()?)
            .map_err(|e| AssetIoError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
    }
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

pub trait AssetFileSystem: Send + Sync + Debug + 'static {
    fn root(&self) -> &Path;
    fn is_dir(&self, path: &Path) -> bool;
    fn exists(&self, path: &Path) -> bool;
    fn reader(&self, path: &Path) -> Box<dyn AssetReader>;
    fn writer(&self, path: &Path) -> Box<dyn AssetWriter>;
}

pub trait PathExt {
    fn ext(&self) -> Option<&str>;
    fn append_ext(&self, ext: &str) -> PathBuf;
    fn with_prefix(&self, prefix: impl AsRef<Path>) -> PathBuf;
    fn without_prefix(&self, prefix: impl AsRef<Path>) -> &Path;
}

impl<T: AsRef<Path>> PathExt for T {
    fn ext(&self) -> Option<&str> {
        self.as_ref().extension().and_then(|ext| ext.to_str())
    }

    fn append_ext(&self, ext: &str) -> PathBuf {
        let path = self.as_ref().to_path_buf();
        format!("{}.{}", path.display(), ext).into()
    }

    fn with_prefix(&self, prefix: impl AsRef<Path>) -> PathBuf {
        match self.as_ref().starts_with(prefix.as_ref()) {
            true => self.as_ref().to_path_buf(),
            false => prefix.as_ref().join(self.as_ref()),
        }
    }

    fn without_prefix(&self, prefix: impl AsRef<Path>) -> &Path {
        let path = self.as_ref();
        let prefix = prefix.as_ref();

        if path.starts_with(prefix) {
            path.strip_prefix(prefix).unwrap()
        } else {
            path
        }
    }
}
