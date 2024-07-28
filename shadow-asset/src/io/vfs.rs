use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::{FileReader, FileSystem};

pub struct VirtualFile {
    name: PathBuf,
    data: Vec<u8>,
    read_offset: usize,
}

impl VirtualFile {
    pub fn open(path: impl ToString) -> Self {
        Self {
            name: PathBuf::from(path.to_string()),
            data: vec![],
            read_offset: 0,
        }
    }

    pub fn name(&self) -> &PathBuf {
        &self.name
    }
}

impl Read for VirtualFile {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let len = buffer.len().min(self.data.len() - self.read_offset);
        buffer[..len].copy_from_slice(&self.data[self.read_offset..self.read_offset + len]);
        self.read_offset += len;
        Ok(len)
    }

    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
        let mut total = 0;
        for buf in bufs {
            let len = self.read(buf)?;
            total += len;
            if len == 0 {
                break;
            }
        }

        Ok(total)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let len = self.data.len() - self.read_offset;
        buf.extend_from_slice(&self.data[self.read_offset..]);
        self.read_offset = self.data.len();
        Ok(len)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        let len = self.data.len() - self.read_offset;
        buf.push_str(&String::from_utf8_lossy(&self.data[self.read_offset..]));
        self.read_offset = self.data.len();
        Ok(len)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        let len = self.read(buf)?;
        if len == buf.len() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ))
        }
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

impl Write for VirtualFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.data.extend_from_slice(buf);
        Ok(())
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        struct Adapter<'a, T: ?Sized + 'a> {
            inner: &'a mut T,
            error: std::io::Result<()>,
        }

        impl<T: Write + ?Sized> std::fmt::Write for Adapter<'_, T> {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                match self.inner.write_all(s.as_bytes()) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        self.error = Err(e);
                        Err(std::fmt::Error)
                    }
                }
            }
        }

        let mut output = Adapter {
            inner: self,
            error: Ok(()),
        };

        std::fmt::write(&mut output, fmt)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "fmt error"))
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

pub struct Folder {
    name: PathBuf,
    entries: Vec<VirtualEntry>,
}

impl Folder {
    pub fn new(path: impl ToString) -> Self {
        Self {
            name: PathBuf::from(path.to_string()),
            entries: vec![],
        }
    }

    pub fn name(&self) -> &PathBuf {
        &self.name
    }

    pub fn add_entry(&mut self, entry: VirtualEntry) {
        self.entries.push(entry);
    }

    pub fn add_file(&mut self, file: VirtualFile) {
        self.entries.push(VirtualEntry::File(file));
    }

    pub fn add_folder(&mut self, folder: Folder) {
        self.entries.push(VirtualEntry::Folder(folder));
    }

    pub fn entry(&self, name: impl AsRef<Path>) -> Option<&VirtualEntry> {
        self.entries.iter().find(|entry| match entry {
            VirtualEntry::File(file) => file.name() == name.as_ref(),
            VirtualEntry::Folder(folder) => folder.name() == name.as_ref(),
        })
    }

    pub fn entry_mut(&mut self, name: impl AsRef<Path>) -> Option<&mut VirtualEntry> {
        self.entries.iter_mut().find(|entry| match entry {
            VirtualEntry::File(file) => file.name() == name.as_ref(),
            VirtualEntry::Folder(folder) => folder.name() == name.as_ref(),
        })
    }

    pub fn remove_entry(&mut self, name: impl AsRef<Path>) -> Option<VirtualEntry> {
        let index = self.entries.iter().position(|entry| match entry {
            VirtualEntry::File(file) => file.name() == name.as_ref(),
            VirtualEntry::Folder(folder) => folder.name() == name.as_ref(),
        })?;

        Some(self.entries.remove(index))
    }

    pub fn iter(&self) -> FolderIter {
        FolderIter {
            folder: self,
            index: 0,
        }
    }

    pub fn iter_mut(&mut self) -> FolderIterMut {
        FolderIterMut {
            folder: self,
            index: 0,
        }
    }
}

pub enum VirtualEntry {
    File(VirtualFile),
    Folder(Folder),
}

impl VirtualEntry {
    pub fn is_file(&self) -> bool {
        matches!(self, VirtualEntry::File(_))
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, VirtualEntry::Folder(_))
    }

    pub fn as_file(&self) -> Option<&VirtualFile> {
        match self {
            VirtualEntry::File(file) => Some(file),
            _ => None,
        }
    }

    pub fn as_folder(&self) -> Option<&Folder> {
        match self {
            VirtualEntry::Folder(folder) => Some(folder),
            _ => None,
        }
    }

    pub fn as_file_mut(&mut self) -> Option<&mut VirtualFile> {
        match self {
            VirtualEntry::File(file) => Some(file),
            _ => None,
        }
    }

    pub fn as_folder_mut(&mut self) -> Option<&mut Folder> {
        match self {
            VirtualEntry::Folder(folder) => Some(folder),
            _ => None,
        }
    }

    pub fn into_file(self) -> Option<VirtualFile> {
        match self {
            VirtualEntry::File(file) => Some(file),
            _ => None,
        }
    }

    pub fn into_folder(self) -> Option<Folder> {
        match self {
            VirtualEntry::Folder(folder) => Some(folder),
            _ => None,
        }
    }
}

pub struct FolderIter<'a> {
    folder: &'a Folder,
    index: usize,
}

impl<'a> Iterator for FolderIter<'a> {
    type Item = &'a VirtualEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.folder.entries.len() {
            let entry = &self.folder.entries[self.index];
            self.index += 1;
            Some(entry)
        } else {
            None
        }
    }
}

pub struct FolderIterMut<'a> {
    folder: &'a mut Folder,
    index: usize,
}

impl<'a> Iterator for FolderIterMut<'a> {
    type Item = &'a mut VirtualEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.folder.entries.len() {
            let folder_ptr = self.folder as *mut Folder;

            unsafe {
                let folder = &mut *folder_ptr;
                let entry = &mut folder.entries[self.index];
                self.index += 1;
                Some(entry)
            }
        } else {
            None
        }
    }
}

pub struct VirtualFileSystem {
    root: Folder,
}

impl VirtualFileSystem {
    pub fn new(path: impl ToString) -> Self {
        Self {
            root: Folder::new(path),
        }
    }

    pub fn root(&self) -> &Folder {
        &self.root
    }
}

impl FileSystem for VirtualFileSystem {
    fn read(&mut self, path: &Path) -> Result<Vec<u8>, crate::AssetIoError> {
        match self.entry_mut(path) {
            Some(VirtualEntry::File(file)) => {
                let mut data = vec![];
                file.read_to_end(&mut data)?;
                Ok(data)
            }
            Some(VirtualEntry::Folder(_)) => Err(crate::AssetIoError::NotFound(path.to_path_buf())),
            None => Err(crate::AssetIoError::NotFound(path.to_path_buf())),
        }
    }

    fn read_to_string(&mut self, path: &Path) -> Result<String, crate::AssetIoError> {
        let data = self.read(path)?;
        Ok(String::from_utf8_lossy(&data).to_string())
    }

    fn read_exact(&mut self, path: &Path, buffer: &mut [u8]) -> Result<(), crate::AssetIoError> {
        match self.entry_mut(path) {
            Some(VirtualEntry::File(file)) => {
                file.read_exact(buffer)?;
                Ok(())
            }
            Some(VirtualEntry::Folder(_)) => Err(crate::AssetIoError::NotFound(path.to_path_buf())),
            None => Err(crate::AssetIoError::NotFound(path.to_path_buf())),
        }
    }

    fn reader(&mut self, path: &Path) -> Result<crate::FileReader, crate::AssetIoError> {
        match self.entry_mut(path) {
            Some(VirtualEntry::File(file)) => Ok(FileReader::new(file)),
            Some(VirtualEntry::Folder(_)) => Err(crate::AssetIoError::NotFound(path.to_path_buf())),
            None => Err(crate::AssetIoError::NotFound(path.to_path_buf())),
        }
    }

    fn write(&mut self, path: &Path, data: &[u8]) -> Result<(), crate::AssetIoError> {
        match self.entry_mut(path) {
            Some(VirtualEntry::File(file)) => {
                file.data.clear();
                file.data.extend_from_slice(data);
                Ok(())
            }
            Some(VirtualEntry::Folder(_)) => Err(crate::AssetIoError::NotFound(path.to_path_buf())),
            None => Err(crate::AssetIoError::NotFound(path.to_path_buf())),
        }
    }

    fn remove(&mut self, path: &Path) -> Result<Vec<PathBuf>, crate::AssetIoError> {
        let parent = path.parent().unwrap();
        let entry = self
            .entry_mut(parent)
            .ok_or_else(|| crate::AssetIoError::NotFound(path.to_path_buf()))?;

        match entry {
            VirtualEntry::File(_) => todo!(),
            VirtualEntry::Folder(folder) => {
                // let paths = vec![]
            },
        }


        todo!()
    }

    fn rename(&mut self, old: &Path, new: &Path) -> Result<(), crate::AssetIoError> {
        todo!()
    }

    fn read_directory(
        &self,
        path: &Path,
        recursive: bool,
    ) -> Result<Vec<PathBuf>, crate::AssetIoError> {
        todo!()
    }

    fn create_dir(&mut self, path: &Path) -> Result<(), crate::AssetIoError> {
        todo!()
    }
}

impl VirtualFileSystem {
    fn entry(&self, path: impl AsRef<Path>) -> Option<&VirtualEntry> {
        let mut folder = &self.root;
        let mut components = path.as_ref().components();
        while let Some(component) = components.next() {
            match component {
                std::path::Component::Normal(name) => {
                    if let Some(entry) = folder.entry(name) {
                        match entry {
                            VirtualEntry::File(file) => {
                                if components.next().is_none() {
                                    return Some(entry);
                                } else {
                                    return None;
                                }
                            }
                            VirtualEntry::Folder(dir) => {
                                folder = dir;
                            }
                        }
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        None
    }

    fn entry_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut VirtualEntry> {
        let mut folder = &mut self.root;
        let mut components = path.as_ref().components();
        while let Some(component) = components.next() {
            match component {
                std::path::Component::Normal(name) => {
                    if let Some(entry) = folder.entry_mut(name) {
                        match entry {
                            VirtualEntry::File(file) => {
                                if components.next().is_none() {
                                    return Some(entry);
                                } else {
                                    return None;
                                }
                            }
                            VirtualEntry::Folder(dir) => {
                                folder = dir;
                            }
                        }
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        None
    }
}
