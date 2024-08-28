use super::{AssetFileSystem, AssetIoError, AssetReader, AssetWriter, PathExt};
use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};

pub struct LocalAsset {
    path: PathBuf,
    file: Option<File>,
    buffer: Vec<u8>,
}

impl LocalAsset {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            file: File::open(path).ok(),
            buffer: Vec::new(),
        }
    }

    pub fn file(&self) -> &Option<File> {
        &self.file
    }

    pub fn file_mut(&mut self) -> Option<&mut File> {
        self.file.as_mut()
    }
}

impl AssetReader for LocalAsset {
    fn path(&self) -> &Path {
        &self.path
    }

    fn read(&mut self, amount: usize) -> super::Result<usize> {
        let mut buffer = vec![0; amount];
        let file = self
            .file_mut()
            .ok_or(AssetIoError::from(std::io::ErrorKind::NotFound))?;

        file.read_exact(&mut buffer).map_err(AssetIoError::from)?;
        let size = buffer.len();

        self.buffer.extend_from_slice(&buffer);
        Ok(size)
    }

    fn read_to_end(&mut self) -> super::Result<usize> {
        let mut buffer = Vec::new();
        let file = self
            .file_mut()
            .ok_or(AssetIoError::from(std::io::ErrorKind::NotFound))?;

        file.read_to_end(&mut buffer).map_err(AssetIoError::from)?;
        let size = buffer.len();

        self.buffer.extend_from_slice(&buffer);

        Ok(size)
    }

    fn read_dir(&self) -> super::Result<Vec<PathBuf>> {
        if let None = &self.file {
            let read = std::fs::read_dir(&self.path).map_err(AssetIoError::from)?;
            let paths = read
                .map(|entry| entry.map(|e| e.path()))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(paths)
        } else {
            Err(AssetIoError::from(std::io::ErrorKind::NotFound))
        }
    }

    fn bytes(&self) -> &[u8] {
        &self.buffer
    }

    fn flush(&mut self) -> super::Result<Vec<u8>> {
        Ok(std::mem::take(&mut self.buffer))
    }
}

impl AssetWriter for LocalAsset {
    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&mut self, buf: &[u8]) -> super::Result<usize> {
        let size = match self.file_mut().and_then(|file| file.write(buf).ok()) {
            Some(size) => size,
            None => buf.len(),
        };
        self.buffer.extend_from_slice(&buf);
        Ok(size)
    }

    fn create_dir(&mut self) -> super::Result<()> {
        std::fs::create_dir_all(&self.path).map_err(AssetIoError::from)
    }

    fn remove_file(&mut self) -> super::Result<()> {
        std::fs::remove_file(&self.path).map_err(AssetIoError::from)
    }

    fn remove_dir(&mut self) -> super::Result<()> {
        std::fs::remove_dir_all(&self.path).map_err(AssetIoError::from)
    }

    fn flush(&mut self) -> super::Result<Vec<u8>> {
        if let Some(file) = self.file_mut() {
            file.rewind().map_err(AssetIoError::from)?
        }
        std::fs::write(&self.path, self.buffer.clone()).map_err(AssetIoError::from)?;
        Ok(std::mem::take(&mut self.buffer))
    }

    fn clear(&mut self) {
        match self.file_mut().and_then(|file| file.set_len(0).ok()) {
            Some(_) => self.buffer.clear(),
            None => self.buffer = Vec::new(),
        }
    }
}

#[derive(Default, Debug)]
pub struct LocalFileSystem {
    root: PathBuf,
}

impl LocalFileSystem {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
}

impl AssetFileSystem for LocalFileSystem {
    fn root(&self) -> &Path {
        &self.root
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn exists(&self, path: &Path) -> bool {
        let path = path.with_prefix(&self.root);
        path.exists()
    }

    fn reader(&self, path: &Path) -> Box<dyn AssetReader> {
        Box::new(LocalAsset::new(path))
    }

    fn writer(&self, path: &Path) -> Box<dyn AssetWriter> {
        Box::new(LocalAsset::new(path))
    }
}
