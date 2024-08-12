use crate::{
    asset::{AssetId, AssetKind},
    bytes::IntoBytes,
    io::{AssetIoError, AssetWriter},
};
use shadow_ecs::core::{DenseMap, DenseSet};
use std::path::PathBuf;

use super::AssetConfig;

#[derive(Default, Debug)]
pub struct AssetLibrary {
    ids: DenseMap<AssetId, PathBuf>,
    paths: DenseMap<PathBuf, AssetId>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        Self {
            ids: DenseMap::new(),
            paths: DenseMap::new(),
        }
    }

    pub fn id(&self, path: &PathBuf) -> Option<&AssetId> {
        self.paths.get(path)
    }

    pub fn path(&self, id: &AssetId) -> Option<&PathBuf> {
        self.ids.get(id)
    }

    pub fn add_asset(&mut self, id: AssetId, path: PathBuf, kind: AssetKind) -> Option<PathBuf> {
        let old = self.ids.insert(id, path.clone()).map(|old_path| {
            self.paths.remove(&old_path);
            old_path
        });
        if kind == AssetKind::Main {
            self.paths.insert(path, id);
        }
        old
    }

    pub fn remove_asset(&mut self, id: &AssetId, kind: AssetKind) -> Option<PathBuf> {
        let path = self.ids.remove(id)?;
        if kind == AssetKind::Main {
            self.paths.remove(&path);
        }
        Some(path)
    }

    pub fn remove_path(&mut self, path: &PathBuf) -> Option<AssetId> {
        let id = self.paths.remove(path)?;
        self.ids.remove(&id);
        Some(id)
    }

    pub fn contains_id(&self, id: &AssetId) -> bool {
        self.ids.contains(id)
    }

    pub fn contains_path(&self, path: &PathBuf) -> bool {
        self.paths.contains(path)
    }

    pub fn save(&self, mut writer: impl AssetWriter) -> Result<Vec<u8>, AssetIoError> {
        let ids = self.ids.into_bytes();
        let paths = self.paths.into_bytes();

        writer.write(&ids.len().into_bytes())?;
        writer.write(&ids)?;
        writer.write(&paths)?;
        writer.flush()
    }

    pub fn load(&mut self, bytes: &[u8]) -> Option<&Self> {
        let ids_len = usize::from_bytes(&bytes)?;

        self.ids = DenseMap::from_bytes(&bytes[8..8 + ids_len])?;
        self.paths = DenseMap::from_bytes(&bytes[16 + ids_len..])?;

        Some(self)
    }
}

pub struct DependentUpdates {
    added: Vec<AssetId>,
    removed: Vec<AssetId>,
}

impl DependentUpdates {
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
        }
    }

    pub fn add(&mut self, id: AssetId) {
        self.added.push(id);
    }

    pub fn remove(&mut self, id: AssetId) {
        self.removed.push(id);
    }

    pub fn clear(&mut self) {
        self.added.clear();
        self.removed.clear();
    }

    pub fn added(&self) -> &[AssetId] {
        &self.added
    }

    pub fn removed(&self) -> &[AssetId] {
        &self.removed
    }
}

#[derive(Default)]
pub struct DependentLibrary {
    dependents: DenseMap<AssetId, DenseSet<AssetId>>,
    updates: DenseMap<AssetId, DependentUpdates>,
}

impl DependentLibrary {
    pub fn new() -> Self {
        Self {
            dependents: DenseMap::new(),
            updates: DenseMap::new(),
        }
    }

    pub fn get(&self, id: &AssetId) -> Option<&DenseSet<AssetId>> {
        self.dependents.get(id)
    }

    pub fn add_dependent(&mut self, id: AssetId, dependent: AssetId) {
        let updates = match self.updates.get_mut(&id) {
            Some(updates) => updates,
            None => {
                self.updates.insert(id, DependentUpdates::new());
                &mut self.updates[&id]
            }
        };

        updates.add(dependent);
    }

    pub fn remove_dependent(&mut self, id: &AssetId, dependent: &AssetId) -> Option<AssetId> {
        let updates = match self.updates.get_mut(id) {
            Some(updates) => updates,
            None => return None,
        };

        updates.remove(*dependent);
        Some(*dependent)
    }

    pub fn remove_asset(&mut self, id: &AssetId) -> DenseSet<AssetId> {
        let updates = match self.updates.remove(id) {
            Some(updates) => updates,
            None => DependentUpdates::new(),
        };

        let mut dependents = self.dependents.remove(id).unwrap_or_else(DenseSet::new);
        updates.added.iter().for_each(|id| dependents.insert(*id));
        for id in updates.removed {
            dependents.remove(&id);
        }

        dependents
    }

    pub fn save(&mut self, config: &AssetConfig) -> Result<Vec<u8>, AssetIoError> {
        let mut writer = config.writer(Self::path(config));
        for (id, mut update) in self.updates.drain() {
            let dependents = match self.dependents.get_mut(&id) {
                Some(dependents) => dependents,
                None => {
                    self.dependents.insert(id, DenseSet::new());
                    &mut self.dependents[&id]
                }
            };

            dependents.extend(update.added.drain(..));
            dependents.retain(|id| !update.removed.contains(id));
        }

        writer.write(&self.dependents.into_bytes())?;
        writer.flush()
    }

    pub fn load(config: &AssetConfig) -> Result<Self, AssetIoError> {
        let path = Self::path(config);

        if !path.exists() {
            return Ok(Self::new());
        }

        let reader = config.reader(Self::path(config));
        let bytes = reader.bytes();
        let dependents = DenseMap::from_bytes(&bytes)
            .ok_or(AssetIoError::from(std::io::ErrorKind::InvalidData))?;

        Ok(Self {
            dependents,
            updates: DenseMap::new(),
        })
    }

    pub fn path(config: &AssetConfig) -> PathBuf {
        config.temp().join("dependents.lib")
    }
}
