use shadow_ecs::ecs::{event::Event, world::World};
use std::path::{Path, PathBuf};

pub struct ImportAsset {
    path: PathBuf,
}

impl ImportAsset {
    pub fn new(path: impl AsRef<Path>) -> ImportAsset {
        ImportAsset {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Event for ImportAsset {
    type Output = PathBuf;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self.path)
    }
}

pub struct RemoveAsset {
    path: PathBuf,
}

impl RemoveAsset {
    pub fn new(path: impl AsRef<Path>) -> RemoveAsset {
        RemoveAsset {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Event for RemoveAsset {
    type Output = PathBuf;

    fn invoke(self, _: &mut World) -> Option<Self::Output> {
        Some(self.path)
    }
}
