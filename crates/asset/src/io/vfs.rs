use super::{AssetFileSystem, AssetIoError, AssetReader, AssetWriter, PathExt};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub struct Entry {
    path: PathBuf,
    buffer: Vec<u8>,
    read_offset: usize,
    write_offset: usize,
    storage: Arc<Mutex<VirtualFileStorage>>,
}

impl Entry {
    pub fn new(path: impl AsRef<Path>, storage: Arc<Mutex<VirtualFileStorage>>) -> Self {
        let path = path.as_ref().to_path_buf();

        let buffer = {
            let storage = storage.lock().unwrap();
            match storage.get_node(&path) {
                Some(INode::File { buffer, .. }) => buffer.clone(),
                _ => vec![],
            }
        };

        Self {
            path,
            buffer,
            read_offset: 0,
            write_offset: 0,
            storage,
        }
    }
}

impl AssetReader for Entry {
    fn path(&self) -> &Path {
        &self.path
    }

    fn read(&mut self, amount: usize) -> super::Result<usize> {
        let offset = (self.read_offset + amount).min(self.buffer.len());
        let size = offset - self.read_offset;
        self.read_offset = offset;
        Ok(size)
    }

    fn read_to_end(&mut self) -> super::Result<usize> {
        self.read(self.buffer.len())
    }

    fn read_dir(&self) -> super::Result<Vec<PathBuf>> {
        let storage = self.storage.lock().unwrap();
        let node = storage
            .get_node(&self.path)
            .ok_or(AssetIoError::from(std::io::ErrorKind::NotFound))?;

        match node {
            INode::Dir { children, .. } => {
                let paths = children
                    .iter()
                    .map(|child| self.path.join(child.name()))
                    .collect();
                Ok(paths)
            }
            _ => Err(AssetIoError::from(std::io::ErrorKind::InvalidInput)),
        }
    }

    fn bytes(&self) -> &[u8] {
        &self.buffer
    }

    fn buf_reader(&self) -> Result<Box<dyn std::io::BufRead + '_>, AssetIoError> {
        Ok(Box::new(std::io::Cursor::new(&self.buffer)))
    }

    fn flush(&mut self) -> super::Result<Vec<u8>> {
        self.read_offset = 0;
        Ok(self.buffer.clone())
    }
}

impl AssetWriter for Entry {
    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&mut self, buf: &[u8]) -> super::Result<usize> {
        let offset = self.write_offset + buf.len();
        self.buffer.resize(offset, 0);
        self.buffer[self.write_offset..offset].copy_from_slice(buf);
        self.write_offset = offset;
        Ok(buf.len())
    }

    fn create_dir(&mut self) -> super::Result<()> {
        let mut storage = self.storage.lock().unwrap();
        storage.create_dir(&self.path)
    }

    fn remove_file(&mut self) -> super::Result<()> {
        let mut storage = self.storage.lock().unwrap();
        storage.remove_file(&self.path)
    }

    fn remove_dir(&mut self) -> super::Result<()> {
        let mut storage = self.storage.lock().unwrap();
        storage.remove_dir(&self.path)
    }

    fn flush(&mut self) -> super::Result<Vec<u8>> {
        self.write_offset = 0;
        let bytes = std::mem::take(&mut self.buffer);
        let mut storage = self.storage.lock().unwrap();
        match storage.get_node_mut(&self.path) {
            Some(node) => match node {
                INode::File { buffer, .. } => {
                    *buffer = bytes.clone();
                }
                _ => return Err(AssetIoError::from(std::io::ErrorKind::InvalidInput)),
            },
            None => {
                storage.create_file(&self.path, bytes.clone())?;
            }
        }

        Ok(bytes)
    }

    fn clear(&mut self) {
        self.write_offset = 0;
        self.buffer.clear();
    }
}

#[derive(Clone)]
pub enum INode {
    Dir {
        name: OsString,
        children: Vec<INode>,
    },
    File {
        name: OsString,
        buffer: Vec<u8>,
    },
}

impl INode {
    pub fn dir(name: OsString) -> Self {
        Self::Dir {
            name,
            children: Vec::new(),
        }
    }

    pub fn child(&self, name: &OsStr) -> Option<&INode> {
        match self {
            Self::Dir { children, .. } => children.iter().find(|child| child.name() == name),
            Self::File { .. } => None,
        }
    }

    pub fn child_mut(&mut self, name: &OsStr) -> Option<&mut INode> {
        match self {
            Self::Dir { children, .. } => children.iter_mut().find(|child| child.name() == name),
            Self::File { .. } => None,
        }
    }

    pub fn remove_child(&mut self, name: &OsStr) -> Option<INode> {
        match self {
            Self::Dir { children, .. } => {
                let index = children.iter().position(|child| child.name() == name)?;
                Some(children.remove(index))
            }
            Self::File { .. } => None,
        }
    }

    pub fn file(name: OsString, buffer: Vec<u8>) -> Self {
        Self::File { name, buffer }
    }

    pub fn name(&self) -> &OsStr {
        match self {
            Self::Dir { name, .. } => &name,
            Self::File { name, .. } => &name,
        }
    }

    pub fn is_dir(&self) -> bool {
        match self {
            Self::Dir { .. } => true,
            Self::File { .. } => false,
        }
    }

    fn display(&self, f: &mut std::fmt::Formatter, depth: usize) -> std::fmt::Result {
        let pad = " ".repeat(depth * 2);
        match self {
            Self::Dir { name, children } => {
                writeln!(f, "{}Dir:{:?}", pad, name)?;
                for child in children {
                    child.display(f, depth + 1)?;
                }
                Ok(())
            }
            Self::File { name, .. } => writeln!(f, "{}File:{:?}", pad, name),
        }
    }
}

impl std::fmt::Debug for INode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.display(f, 0)
    }
}

impl std::fmt::Display for INode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.display(f, 0)
    }
}

pub struct VirtualFileStorage {
    root: INode,
}

impl VirtualFileStorage {
    pub fn new(root: OsString) -> Self {
        Self {
            root: INode::Dir {
                name: root,
                children: Vec::new(),
            },
        }
    }

    pub fn create_file(&mut self, path: &Path, buffer: Vec<u8>) -> Result<(), AssetIoError> {
        let parent = path
            .parent()
            .and_then(|path| self.get_node_mut(path))
            .ok_or(AssetIoError::from(std::io::ErrorKind::NotFound))?;

        let name = path
            .file_name()
            .ok_or(AssetIoError::from(std::io::ErrorKind::NotFound))?;

        match parent {
            INode::Dir { children, .. } => match children.iter().find(|child| child.name() == name)
            {
                Some(_) => return Err(AssetIoError::from(std::io::ErrorKind::AlreadyExists)),
                None => {
                    let file = INode::file(name.to_os_string(), buffer);
                    children.push(file);
                }
            },
            _ => return Err(AssetIoError::from(std::io::ErrorKind::InvalidInput)),
        };

        Ok(())
    }

    pub fn remove_file(&mut self, path: &Path) -> Result<(), AssetIoError> {
        let parent = path
            .parent()
            .and_then(|path| self.get_node_mut(path))
            .ok_or(AssetIoError::from(std::io::ErrorKind::NotFound))?;

        let name = path
            .file_name()
            .ok_or(AssetIoError::from(std::io::ErrorKind::NotFound))?;

        match parent {
            INode::Dir { .. } => parent.remove_child(name),
            _ => return Err(AssetIoError::from(std::io::ErrorKind::InvalidInput)),
        };

        Ok(())
    }

    pub fn create_dir(&mut self, path: &Path) -> Result<(), AssetIoError> {
        let mut current = &mut self.root;
        for component in path.components() {
            match component {
                std::path::Component::Normal(name) => {
                    let node: *mut INode = current as *mut INode;
                    let node: &mut INode = unsafe { &mut *node };

                    if let Some(child) = current.child_mut(name) {
                        current = child;
                    } else {
                        let dir = INode::dir(name.to_os_string());
                        match node {
                            INode::Dir { children, .. } => children.push(dir),
                            _ => return Err(AssetIoError::from(std::io::ErrorKind::InvalidInput)),
                        }
                        current = node.child_mut(name).unwrap();
                    }
                }
                std::path::Component::RootDir => continue,
                _ => return Err(AssetIoError::from(std::io::ErrorKind::InvalidInput)),
            }
        }

        Ok(())
    }

    pub fn remove_dir(&mut self, path: &Path) -> Result<(), AssetIoError> {
        let parent = path
            .parent()
            .and_then(|path| self.get_node_mut(path))
            .ok_or(AssetIoError::from(std::io::ErrorKind::NotFound))?;

        let name = path
            .file_name()
            .ok_or(AssetIoError::from(std::io::ErrorKind::NotFound))?;

        match parent {
            INode::Dir { .. } => parent.remove_child(name),
            _ => return Err(AssetIoError::from(std::io::ErrorKind::InvalidInput)),
        };

        Ok(())
    }

    pub fn get_node(&self, path: impl AsRef<Path>) -> Option<&INode> {
        let mut current = &self.root;
        for component in path.as_ref().components() {
            match component {
                std::path::Component::Normal(name) => {
                    if let Some(child) = current.child(name) {
                        current = child;
                    } else {
                        return None;
                    }
                }
                std::path::Component::RootDir => continue,
                _ => return None,
            }
        }

        Some(current)
    }

    pub fn get_node_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut INode> {
        let mut current = &mut self.root;
        for component in path.as_ref().components() {
            match component {
                std::path::Component::Normal(name) => {
                    if let Some(child) = current.child_mut(name) {
                        current = child;
                    } else {
                        return None;
                    }
                }
                std::path::Component::RootDir => continue,
                _ => return None,
            }
        }

        Some(current)
    }
}

impl std::fmt::Debug for VirtualFileStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.root)
    }
}

impl std::fmt::Display for VirtualFileStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.root)
    }
}

pub struct VirtualFileSystem {
    storage: Arc<Mutex<VirtualFileStorage>>,
    root: PathBuf,
}

impl VirtualFileSystem {
    pub fn new(root: impl Into<OsString>) -> Self {
        let root = PathBuf::from(root.into());
        Self {
            root: root.clone(),
            storage: Arc::new(Mutex::new(VirtualFileStorage::new(root.into_os_string()))),
        }
    }

    pub fn storage(&self) -> Arc<Mutex<VirtualFileStorage>> {
        self.storage.clone()
    }
}

impl Default for VirtualFileSystem {
    fn default() -> Self {
        Self::new("/")
    }
}

impl AssetFileSystem for VirtualFileSystem {
    fn root(&self) -> &Path {
        &self.root
    }

    fn is_dir(&self, path: &Path) -> bool {
        let storage = self.storage.lock().unwrap();
        storage.get_node(path).map_or(false, |node| node.is_dir())
    }

    fn exists(&self, path: &Path) -> bool {
        let path = path.with_prefix(self.root());
        self.storage.lock().unwrap().get_node(path).is_some()
    }

    fn reader(&self, path: &Path) -> Box<dyn AssetReader> {
        Box::new(Entry::new(path, self.storage.clone()))
    }

    fn writer(&self, path: &Path) -> Box<dyn AssetWriter> {
        Box::new(Entry::new(path, self.storage.clone()))
    }
}

impl std::fmt::Debug for VirtualFileSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.storage.lock().unwrap())
    }
}

impl std::fmt::Display for VirtualFileSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.storage.lock().unwrap())
    }
}
