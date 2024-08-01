use super::hash_value;
use std::{collections::HashMap, hash::Hash};

pub struct DenseMap<K: Hash + Eq, V> {
    keys: Vec<K>,
    values: Vec<V>,
    map: HashMap<u64, usize>,
}

impl<K: Hash + Eq, V> DenseMap<K, V> {
    pub fn new() -> Self {
        Self {
            keys: vec![],
            values: vec![],
            map: HashMap::new(),
        }
    }

    pub fn contains(&self, key: &K) -> bool {
        let key = hash_value(key);
        self.map.contains_key(&key)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let key = hash_value(key);
        self.map.get(&key).map(|&index| &self.values[index])
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let key = hash_value(key);
        self.map.get(&key).map(|&index| &mut self.values[index])
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash = hash_value(&key);
        if let Some(old) = self.map.insert(hash, self.keys.len()) {
            let old_value = self.values.swap_remove(old);
            let key = self.keys.swap_remove(old);
            for index in old..self.keys.len() {
                let key = hash_value(&self.keys[index]);
                self.map.insert(key, index);
            }
            self.keys.push(key);
            self.values.push(value);
            Some(old_value)
        } else {
            self.keys.push(key);
            self.values.push(value);
            None
        }
    }

    pub fn insert_before(&mut self, index: usize, key: K, value: V) {
        let hash = hash_value(&key);
        if let Some(old_index) = self.map.remove(&hash) {
            self.keys.insert(index, key);
            self.values.insert(index, value);
            let start = old_index.min(index);
            for index in start..self.keys.len() {
                let key = hash_value(&self.keys[index]);
                self.map.insert(key, index);
            }
        } else {
            self.keys.insert(index, key);
            self.values.insert(index, value);
            for index in index..self.keys.len() {
                let key = hash_value(&self.keys[index]);
                self.map.insert(key, index);
            }
        }
    }

    pub fn insert_after(&mut self, index: usize, key: K, value: V) {
        let hash = hash_value(&key);
        if let Some(old_index) = self.map.remove(&hash) {
            self.keys.insert(index + 1, key);
            self.values.insert(index + 1, value);
            let start = old_index.min(index + 1);
            for index in start..self.keys.len() {
                let key = hash_value(&self.keys[index]);
                self.map.insert(key, index);
            }
        } else {
            self.keys.insert(index + 1, key);
            self.values.insert(index + 1, value);
            for index in index + 1..self.keys.len() {
                let key = hash_value(&self.keys[index]);
                self.map.insert(key, index);
            }
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let hash = hash_value(key);
        if let Some(index) = self.map.remove(&hash) {
            let value = self.values.remove(index);
            self.keys.remove(index);
            for index in index..self.keys.len() {
                let key = hash_value(&self.keys[index]);
                self.map.insert(key, index);
            }

            Some(value)
        } else {
            None
        }
    }

    pub fn remove_at(&mut self, index: usize) -> (K, V) {
        let key = self.keys.remove(index);
        let value = self.values.remove(index);
        for index in index..self.keys.len() {
            let key = hash_value(&self.keys[index]);
            self.map.insert(key, index);
        }

        (key, value)
    }

    pub fn swap_remove(&mut self, key: &K) -> Option<V> {
        let hash = hash_value(key);
        if let Some(index) = self.map.remove(&hash) {
            let value = self.values.swap_remove(index);
            self.keys.swap_remove(index);
            for index in index..self.keys.len() {
                let key = hash_value(&self.keys[index]);
                self.map.insert(key, index);
            }

            Some(value)
        } else {
            None
        }
    }

    pub fn swap_remove_at(&mut self, index: usize) -> (K, V) {
        let key = self.keys.swap_remove(index);
        let value = self.values.swap_remove(index);
        for index in index..self.keys.len() {
            let key = hash_value(&self.keys[index]);
            self.map.insert(key, index);
        }

        (key, value)
    }

    pub fn extend(&mut self, iter: impl Iterator<Item = (K, V)>) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }

    pub fn append(&mut self, other: &mut Self) {
        for (key, value) in other.keys.drain(..).zip(other.values.drain(..)) {
            self.insert(key, value);
        }
    }

    pub fn retain(&mut self, mut f: impl FnMut(&K, &V) -> bool) {
        let mut index = 0;
        while index < self.keys.len() {
            let key = &self.keys[index];
            let value = &self.values[index];
            if !f(key, value) {
                self.keys.remove(index);
                self.values.remove(index);
            } else {
                index += 1;
            }
        }

        for (index, key) in self.keys.iter().enumerate() {
            let hash = hash_value(key);
            self.map.insert(hash, index);
        }
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (K, V)> + '_ {
        self.map.clear();
        self.keys.drain(..).zip(self.values.drain(..))
    }

    pub fn sort(&mut self, mut sorter: impl FnMut(&K, &K) -> std::cmp::Ordering) {
        let mut keys = std::mem::take(&mut self.keys);
        let values = std::mem::take(&mut self.values);
        keys.sort_by(|a, b| sorter(a, b));
        for (index, key) in keys.iter().enumerate() {
            let hash = hash_value(key);
            self.map.insert(hash, index);
        }
        self.keys = keys;
        self.values = values;
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.keys.iter().zip(self.values.iter_mut())
    }

    pub fn keys(&self) -> &[K] {
        &self.keys
    }

    pub fn values(&self) -> &[V] {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut [V] {
        &mut self.values
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn into_keys(self) -> Vec<K> {
        self.keys
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.values.clear();
        self.map.clear();
    }
}

impl<K: Hash + Eq, V> Into<Vec<V>> for DenseMap<K, V> {
    fn into(self) -> Vec<V> {
        self.values
    }
}

impl<K: Hash + Eq, V> Into<Vec<(K, V)>> for DenseMap<K, V> {
    fn into(self) -> Vec<(K, V)> {
        self.keys.into_iter().zip(self.values).collect()
    }
}

impl<K: Hash + Eq, V> Default for DenseMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Eq, V> IntoIterator for DenseMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::iter::Zip<std::vec::IntoIter<K>, std::vec::IntoIter<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter().zip(self.values.into_iter())
    }
}

impl<'a, K: Hash + Eq, V> IntoIterator for &'a DenseMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::iter::Zip<std::slice::Iter<'a, K>, std::slice::Iter<'a, V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.iter().zip(self.values.iter())
    }
}

impl<'a, K: Hash + Eq, V> IntoIterator for &'a mut DenseMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = std::iter::Zip<std::slice::Iter<'a, K>, std::slice::IterMut<'a, V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.iter().zip(self.values.iter_mut())
    }
}

impl<K: Hash + Eq, V> Clone for DenseMap<K, V>
where
    K: Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            values: self.values.clone(),
            map: self.map.clone(),
        }
    }
}

impl<K: Hash + Eq, V> std::ops::Index<usize> for DenseMap<K, V> {
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

impl<K: Hash + Eq, V> std::ops::IndexMut<usize> for DenseMap<K, V> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.values[index]
    }
}

impl<K: Hash + Eq, V> std::ops::Index<&K> for DenseMap<K, V> {
    type Output = V;

    fn index(&self, key: &K) -> &Self::Output {
        self.get(key).expect("key not found")
    }
}

impl<K: Hash + Eq, V> std::ops::IndexMut<&K> for DenseMap<K, V> {
    fn index_mut(&mut self, key: &K) -> &mut Self::Output {
        self.get_mut(key).expect("key not found")
    }
}

impl<K: Hash + Eq, V> std::fmt::Debug for DenseMap<K, V>
where
    K: std::fmt::Debug,
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

pub struct ImmutableDenseMap<K: Hash + Eq, V> {
    keys: Vec<K>,
    values: Vec<V>,
    map: HashMap<u64, usize>,
}

impl<K: Hash + Eq, V> ImmutableDenseMap<K, V> {
    pub fn contains(&self, key: &K) -> bool {
        let key = hash_value(key);
        self.map.contains_key(&key)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let key = hash_value(key);
        self.map.get(&key).map(|&index| &self.values[index])
    }

    pub fn index_of(&self, key: &K) -> Option<usize> {
        let key = hash_value(key);
        self.map.get(&key).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().zip(self.values.iter())
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl<K: Hash + Eq, V> std::ops::Index<usize> for ImmutableDenseMap<K, V> {
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

impl<K: Hash + Eq, V> std::ops::Index<&K> for ImmutableDenseMap<K, V> {
    type Output = V;

    fn index(&self, key: &K) -> &Self::Output {
        self.get(key).expect("key not found")
    }
}

impl<K: Hash + Eq, V> std::fmt::Debug for ImmutableDenseMap<K, V>
where
    K: std::fmt::Debug,
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K: Hash + Eq, V> Clone for ImmutableDenseMap<K, V>
where
    K: Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            values: self.values.clone(),
            map: self.map.clone(),
        }
    }
}

impl<K: Hash + Eq, V> Default for ImmutableDenseMap<K, V> {
    fn default() -> Self {
        Self {
            keys: vec![],
            values: vec![],
            map: HashMap::new(),
        }
    }
}

impl<K: Hash + Eq, V> IntoIterator for ImmutableDenseMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::iter::Zip<std::vec::IntoIter<K>, std::vec::IntoIter<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter().zip(self.values.into_iter())
    }
}

impl<'a, K: Hash + Eq, V> IntoIterator for &'a ImmutableDenseMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::iter::Zip<std::slice::Iter<'a, K>, std::slice::Iter<'a, V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.iter().zip(self.values.iter())
    }
}

impl<K: Hash + Eq, V> From<DenseMap<K, V>> for ImmutableDenseMap<K, V> {
    fn from(map: DenseMap<K, V>) -> Self {
        Self {
            keys: map.keys,
            values: map.values,
            map: map.map,
        }
    }
}
