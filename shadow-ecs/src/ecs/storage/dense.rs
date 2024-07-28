use std::{collections::HashMap, hash::Hash};

pub struct DenseMap<K: Clone + Hash + Eq, V> {
    values: Vec<V>,
    keys: Vec<K>,
    map: HashMap<K, usize>,
}

impl<K: Clone + Hash + Eq, V> DenseMap<K, V> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            keys: Vec::new(),
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(index) = self.map.get(&key).copied() {
            let old_value = std::mem::replace(&mut self.values[index], value);
            self.keys[index] = key;
            return Some(old_value);
        }

        let index = self.values.len();
        self.values.push(value);
        self.keys.push(key.clone());
        self.map.insert(key, index);

        None
    }

    pub fn insert_before(&mut self, key: K, value: V, before: &K) -> Option<V> {
        if let Some(index) = self.map.get(before).copied() {
            self.values.insert(index, value);
            self.keys.insert(index, key.clone());

            for i in index + 1..self.values.len() {
                self.map.insert(self.keys[i].clone(), i);
            }

            None
        } else {
            self.insert(key, value)
        }
    }

    pub fn insert_after(&mut self, key: K, value: V, after: &K) -> Option<V> {
        if let Some(index) = self.map.get(after).copied() {
            self.values.insert(index + 1, value);
            self.keys.insert(index + 1, key.clone());

            for i in index + 1..self.values.len() {
                self.map.insert(self.keys[i].clone(), i);
            }

            None
        } else {
            self.insert(key, value)
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key).map(|&index| &self.values[index])
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let index = self.map.get(key)?;
        Some(&mut self.values[*index])
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let index = self.map.remove(key)?;
        let value = self.values.remove(index);
        self.keys.remove(index);
        self.map.insert(self.keys[index].clone(), index);

        Some(value)
    }

    pub fn swap_remove(&mut self, key: &K) -> Option<V> {
        let index = self.map.remove(key)?;
        let value = self.values.swap_remove(index);
        self.keys.swap_remove(index);
        self.map.insert(self.keys[index].clone(), index);

        Some(value)
    }

    pub fn retain(&mut self, mut f: impl FnMut(&K, &mut V) -> bool) {
        let mut i = 0;
        while i < self.values.len() {
            let key = &self.keys[i];
            let value = &mut self.values[i];
            if !f(key, value) {
                self.map.remove(key);
                self.values.remove(i);
                self.keys.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.keys.iter().zip(self.values.iter_mut())
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.keys.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.values.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.values.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn drain_keys(&mut self) -> Vec<K> {
        self.map.clear();
        self.values.clear();
        self.keys.drain(..).collect()
    }

    pub fn drain(&mut self) -> Vec<(K, V)> {
        self.map.clear();
        self.keys.drain(..).zip(self.values.drain(..)).collect()
    }

    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn sort(&mut self, f: impl Fn(&K, &K) -> std::cmp::Ordering) {
        let mut keys = self.keys.clone();
        keys.sort_by(|a, b| f(a, b));

        let mut map = HashMap::new();
        for (index, key) in keys.iter().enumerate() {
            map.insert(key.clone(), index);
        }

        self.keys = keys;
        self.map = map;
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.values.clear();
        self.keys.clear();
    }
}

#[derive(Clone, Default)]
pub struct DenseSet<K: Clone + Hash + Eq> {
    keys: Vec<K>,
    map: HashMap<K, usize>,
}

impl<K: Clone + Hash + Eq> DenseSet<K> {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            map: HashMap::new(),
        }
    }

    pub fn index(&self, key: &K) -> Option<usize> {
        self.map.get(key).copied()
    }

    pub fn insert(&mut self, key: K) -> usize {
        let index = self.keys.len();
        self.keys.push(key.clone());
        self.map.insert(key, index);
        index
    }

    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn remove(&mut self, key: &K) -> Option<K> {
        let index = self.map.remove(key)?;
        let key = self.keys.remove(index);
        self.map.insert(self.keys[index].clone(), index);

        Some(key)
    }

    pub fn swap_remove(&mut self, key: &K) -> Option<K> {
        let index = self.map.remove(key)?;
        let key = self.keys.swap_remove(index);
        self.map.insert(self.keys[index].clone(), index);

        Some(key)
    }

    pub fn retain(&mut self, mut f: impl FnMut(&K) -> bool) {
        let mut i = 0;
        while i < self.keys.len() {
            let key = &self.keys[i];
            if !f(key) {
                self.map.remove(key);
                self.keys.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn drain(&mut self) -> Vec<K> {
        self.map.clear();
        std::mem::take(&mut self.keys)
    }

    pub fn keys(&self) -> &[K] {
        &self.keys
    }

    pub fn iter(&self) -> impl Iterator<Item = &K> {
        self.keys.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut K> {
        self.keys.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn sort_by(&mut self, f: impl Fn(&K, &K) -> std::cmp::Ordering) {
        self.keys.sort_by(|a, b| f(a, b));
        self.map.clear();

        for (index, key) in self.keys.iter().enumerate() {
            self.map.insert(key.clone(), index);
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.keys.clear();
    }
}

impl<K: Clone + Hash + Eq> FromIterator<K> for DenseSet<K> {
    fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
        let mut set = DenseSet::new();
        for key in iter {
            set.insert(key);
        }
        set
    }
}

impl<K: Clone + Hash + Eq> IntoIterator for DenseSet<K> {
    type Item = K;
    type IntoIter = std::vec::IntoIter<K>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter()
    }
}

impl<K: Clone + Hash + Eq> std::iter::Extend<K> for DenseSet<K> {
    fn extend<T: IntoIterator<Item = K>>(&mut self, iter: T) {
        for key in iter {
            self.insert(key);
        }
    }
}

impl<K: Clone + Hash + Eq> From<&[K]> for DenseSet<K> {
    fn from(slice: &[K]) -> Self {
        slice.iter().cloned().collect()
    }
}
