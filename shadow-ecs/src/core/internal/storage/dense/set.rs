use super::hash_value;
use std::{collections::HashMap, hash::Hash};

pub struct DenseSet<K: Hash + Eq> {
    keys: Vec<K>,
    map: HashMap<u64, usize>,
}

impl<K: Hash + Eq> DenseSet<K> {
    pub fn new() -> Self {
        Self {
            keys: vec![],
            map: HashMap::new(),
        }
    }

    pub fn contains(&self, value: &K) -> bool {
        let key = hash_value(value);
        self.map.contains_key(&key)
    }

    pub fn get(&self, index: usize) -> Option<&K> {
        self.keys.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut K> {
        self.keys.get_mut(index)
    }

    pub fn insert(&mut self, value: K) {
        let key = hash_value(&value);
        if let Some(index) = self.map.get(&key).copied() {
            self.keys[index] = value;
        } else {
            let index = self.keys.len();
            self.keys.push(value);
            self.map.insert(key, index);
        }
    }

    pub fn insert_before(&mut self, index: usize, value: K) {
        self.keys.insert(index, value);
        for index in index..self.keys.len() {
            let key = hash_value(&self.keys[index]);
            self.map.insert(key, index);
        }
    }

    pub fn insert_after(&mut self, index: usize, value: K) {
        self.keys.insert(index + 1, value);
        for index in index..self.keys.len() {
            let key = hash_value(&self.keys[index]);
            self.map.insert(key, index);
        }
    }

    pub fn remove(&mut self, value: &K) -> Option<usize> {
        let key = hash_value(value);
        if let Some(index) = self.map.remove(&key) {
            for index in index..(self.keys.len().max(index)) {
                let key = hash_value(&self.keys[index]);
                self.map.insert(key, index);
            }

            Some(index)
        } else {
            None
        }
    }

    pub fn remove_at(&mut self, index: usize) -> Option<K> {
        let value = self.keys.remove(index);
        for index in index..(self.keys.len().max(index)) {
            let key = hash_value(&self.keys[index]);
            self.map.insert(key, index);
        }

        Some(value)
    }

    pub fn swap_remove(&mut self, value: &K) -> bool {
        let key = hash_value(value);
        if let Some(index) = self.map.remove(&key) {
            self.keys.swap_remove(index);
            let key = hash_value(&self.keys[index]);
            self.map.insert(key, index);
            true
        } else {
            false
        }
    }

    pub fn swap_remove_at(&mut self, index: usize) -> K {
        let value = self.keys.swap_remove(index);
        let key = hash_value(&self.keys[index]);
        self.map.insert(key, index);
        value
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = K>) {
        for value in iter {
            self.insert(value);
        }
    }

    pub fn append(&mut self, other: &mut Self) {
        for value in other.keys.drain(..) {
            self.insert(value);
        }

        other.clear();
    }

    pub fn retain(&mut self, mut f: impl FnMut(&K) -> bool) {
        self.keys.retain(|value| f(value));
        self.map.clear();

        for (index, value) in self.keys.iter().enumerate() {
            let key = hash_value(value);
            self.map.insert(key, index);
        }
    }

    pub fn drain(&mut self) -> std::vec::Drain<K> {
        self.map.clear();
        self.keys.drain(..)
    }

    pub fn index_of(&self, value: &K) -> Option<usize> {
        let key = hash_value(value);
        self.map.get(&key).copied()
    }

    pub fn keys(&self) -> &[K] {
        &self.keys
    }

    pub fn iter(&self) -> std::slice::Iter<K> {
        self.keys.iter()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn into_immutable(self) -> ImmutableDenseSet<K> {
        ImmutableDenseSet {
            keys: self.keys,
            map: self.map,
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.keys.clear();
    }
}

impl<K: Hash + Eq + Ord> DenseSet<K> {
    pub fn sort(&mut self) {
        self.keys.sort();
        self.map.clear();

        for (index, value) in self.keys.iter().enumerate() {
            let key = hash_value(value);
            self.map.insert(key, index);
        }
    }
}

impl<K: Hash + Eq> std::ops::Index<usize> for DenseSet<K> {
    type Output = K;

    fn index(&self, index: usize) -> &Self::Output {
        &self.keys[index]
    }
}

impl<K: Hash + Eq + Clone> Clone for DenseSet<K> {
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            map: self.map.clone(),
        }
    }
}

impl<K: Hash + Eq> IntoIterator for DenseSet<K> {
    type Item = K;
    type IntoIter = std::vec::IntoIter<K>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter()
    }
}

impl<K: Hash + Eq> FromIterator<K> for DenseSet<K> {
    fn from_iter<I: IntoIterator<Item = K>>(iter: I) -> Self {
        let mut set = Self::new();
        set.extend(iter);
        set
    }
}

impl<K: Hash + Eq> Default for DenseSet<K> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ImmutableDenseSet<K: Hash + Eq> {
    keys: Vec<K>,
    map: HashMap<u64, usize>,
}

impl<K: Hash + Eq> ImmutableDenseSet<K> {
    pub fn contains(&self, value: &K) -> bool {
        let key = hash_value(value);
        self.map.contains_key(&key)
    }

    pub fn get(&self, index: usize) -> Option<&K> {
        self.keys.get(index)
    }

    pub fn index_of(&self, value: &K) -> Option<usize> {
        let key = hash_value(value);
        self.map.get(&key).copied()
    }

    pub fn iter(&self) -> std::slice::Iter<K> {
        self.keys.iter()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl<K: Hash + Eq> std::ops::Index<usize> for ImmutableDenseSet<K> {
    type Output = K;

    fn index(&self, index: usize) -> &Self::Output {
        &self.keys[index]
    }
}

impl<K: Hash + Eq> IntoIterator for ImmutableDenseSet<K> {
    type Item = K;
    type IntoIter = std::vec::IntoIter<K>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter()
    }
}

impl<K: Hash + Eq> From<DenseSet<K>> for ImmutableDenseSet<K> {
    fn from(set: DenseSet<K>) -> Self {
        Self {
            keys: set.keys,
            map: set.map,
        }
    }
}

impl<K: Hash + Eq + Clone> Clone for ImmutableDenseSet<K> {
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            map: self.map.clone(),
        }
    }
}

impl<K: Hash + Eq> Default for ImmutableDenseSet<K> {
    fn default() -> Self {
        Self {
            keys: vec![],
            map: HashMap::new(),
        }
    }
}
