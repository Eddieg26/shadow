use super::{AssetIoError, AssetReader};
use crate::{AssetId, AssetKind};
use ecs::core::{DenseMap, Resource};
use std::{
    io::{BufRead, Cursor},
    path::{Path, PathBuf},
};

pub struct EmbeddedReader {
    path: PathBuf,
    bytes: &'static [u8],
    offset: usize,
}

impl EmbeddedReader {
    pub fn new(path: &'static str, bytes: &'static [u8]) -> Self {
        Self {
            path: PathBuf::from(path),
            bytes,
            offset: 0,
        }
    }
}

impl AssetReader for EmbeddedReader {
    fn path(&self) -> &Path {
        &self.path
    }

    fn read(&mut self, amount: usize) -> Result<usize, AssetIoError> {
        let end = self.offset + amount;
        let read = self.bytes[self.offset..end].len();
        self.offset = end;
        Ok(read)
    }

    fn read_to_end(&mut self) -> Result<usize, AssetIoError> {
        let read = self.bytes.len() - self.offset;
        self.offset = self.bytes.len();
        Ok(read)
    }

    fn read_to_string(&mut self) -> super::Result<String> {
        let string = String::from_utf8_lossy(&self.bytes[self.offset..]).to_string();
        self.offset = self.bytes.len();
        Ok(string)
    }

    fn read_dir(&self) -> Result<Vec<PathBuf>, AssetIoError> {
        Err(AssetIoError::NotFound(self.path.clone()))
    }

    fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn buf_reader(&self) -> Result<Box<dyn BufRead + '_>, AssetIoError> {
        Ok(Box::new(Cursor::new(&self.bytes)))
    }

    fn flush(&mut self) -> Result<Vec<u8>, AssetIoError> {
        let bytes = self.bytes[self.offset..].to_vec();
        self.offset = self.bytes.len();
        Ok(bytes)
    }
}

#[derive(Default)]
pub struct EmbeddedAssets {
    assets: DenseMap<AssetId, (PathBuf, AssetKind)>,
}

impl EmbeddedAssets {
    pub const EMBEDDED: &'static str = "embedded";

    pub fn new() -> Self {
        Self {
            assets: DenseMap::new(),
        }
    }

    pub fn add(&mut self, id: AssetId, path: &'static str, kind: AssetKind) {
        let path = PathBuf::from(Self::EMBEDDED).join(path);
        self.assets.insert(id, (path, kind));
    }

    pub fn drain(&mut self) -> DenseMap<AssetId, (PathBuf, AssetKind)> {
        std::mem::take(&mut self.assets)
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains(id)
    }
}

impl Resource for EmbeddedAssets {}

#[macro_export]
macro_rules! embed_asset {
    ($game:expr, $id:expr, $path:expr) => {
        $game.embed($id, $path, include_bytes!($path))
    };
}
