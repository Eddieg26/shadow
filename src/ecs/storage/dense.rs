use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

pub struct DenseMap<K: Hash + PartialEq + PartialOrd, V> {
    map: HashMap<u64, usize>,
    keys: Vec<K>,
    values: Vec<V>,
}

impl<K: Hash + PartialEq + PartialOrd, V> DenseMap<K, V> {
    pub fn new() -> Self {
        DenseMap {
            map: HashMap::new(),
            keys: vec![],
            values: vec![],
        }
    }

    pub fn contains(&self, key: &K) -> bool {
        let hasher = hash(key);
        self.map.contains_key(&hasher)
    }

    pub fn index(&self, key: &K) -> Option<usize> {
        let hasher = hash(key);
        self.map.get(&hasher).copied()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let hasher = hash(key);
        self.map
            .get(&hasher)
            .and_then(|index| self.values.get(*index))
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let hasher = hash(key);
        self.map
            .get(&hasher)
            .and_then(|index| self.values.get_mut(*index))
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hasher = hash(&key);
        if let Some(index) = self.map.get(&hasher) {
            let old = std::mem::replace(&mut self.values[*index], value);
            Some(old)
        } else {
            let index = self.keys.len();
            self.keys.push(key);
            self.values.push(value);
            self.map.insert(hasher, index);
            None
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let hashed = hash(key);

        if let Some(index) = self.map.remove(&hashed) {
            self.keys.swap_remove(index);
            let value = self.values.swap_remove(index);
            if index < self.len() {
                let hashed = hash(&self.keys[index]);
                self.map.insert(hashed, index);
            }

            Some(value)
        } else {
            None
        }
    }

    pub fn keys(&self) -> &[K] {
        &self.keys
    }

    pub fn values(&self) -> &[V] {
        &self.values
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.keys.iter().zip(self.values.iter_mut())
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (K, V)> + '_ {
        self.keys.drain(..).zip(self.values.drain(..))
    }

    pub fn retain(&mut self, f: impl Fn(&K, &V) -> bool) {
        let mut index = 0usize;
        while index < self.keys.len() {
            if !f(&self.keys[index], &self.values[index]) {
                self.map.remove(&hash(&self.keys[index]));
                self.keys.remove(index);
                self.values.remove(index);
            } else {
                let hashed = hash(&self.keys[index]);
                self.map.insert(hashed, index);
                index += 1;
            }
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.keys.clear();
        self.values.clear();
    }

    pub fn destruct(self) -> (Vec<K>, Vec<V>, HashMap<u64, usize>) {
        (self.keys, self.values, self.map)
    }
}

fn hash<H: Hash>(value: &H) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub struct DenseSet<K: Hash + PartialEq + PartialOrd> {
    map: HashMap<u64, usize>,
    values: Vec<K>,
}

impl<V: Hash + PartialEq + PartialOrd> DenseSet<V> {
    pub fn new() -> Self {
        DenseSet {
            map: HashMap::new(),
            values: vec![],
        }
    }

    pub fn get(&self, index: usize) -> Option<&V> {
        self.values.get(index)
    }

    pub fn contains(&self, value: &V) -> bool {
        let hasher = hash(value);
        self.map.contains_key(&hasher)
    }

    pub fn insert(&mut self, value: V) -> bool {
        let hasher = hash(&value);
        if self.map.contains_key(&hasher) {
            false
        } else {
            let index = self.values.len();
            self.values.push(value);
            self.map.insert(hasher, index);
            true
        }
    }

    pub fn remove_at(&mut self, index: usize) -> V {
        let value = self.values.swap_remove(index);
        let hashed = hash(&value);
        self.map.remove(&hashed);

        let hashed = hash(&self.values[index]);
        self.map.insert(hashed, index);

        value
    }

    pub fn swap_remove(&mut self, value: &V) -> bool {
        let hashed = hash(value);

        if let Some(index) = self.map.remove(&hashed) {
            self.values.swap_remove(index);
            if index < self.len() {
                let hashed = hash(&self.values[index]);
                self.map.insert(hashed, index);
            }
            true
        } else {
            false
        }
    }

    pub fn retain(&mut self, f: impl Fn(&V) -> bool) {
        self.values.retain(|value| f(value));

        self.map.clear();
        for (index, value) in self.values.iter().enumerate() {
            let hashed = hash(value);
            self.map.insert(hashed, index);
        }
    }

    pub fn values(&self) -> &[V] {
        &self.values
    }

    pub fn index(&self, value: &V) -> Option<usize> {
        let hashed = hash(value);
        self.map.get(&hashed).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.values.iter()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.values.clear();
    }

    pub fn destruct(self) -> (Vec<V>, HashMap<u64, usize>) {
        (self.values, self.map)
    }
}

impl<V: Clone + Hash + PartialEq + PartialOrd> DenseSet<V> {
    pub fn intersection(&self, other: &Self) -> Self {
        let mut intersection = Self::new();
        let (values, map) = if self.values.len() < other.values.len() {
            (&self.values, &other.map)
        } else {
            (&other.values, &self.map)
        };

        for v in values {
            let hashed = hash(v);
            if map.contains_key(&hashed) {
                intersection.insert(v.clone());
            }
        }

        intersection
    }
}

impl<V: Clone + Hash + PartialEq + PartialOrd> From<&[V]> for DenseSet<V> {
    fn from(value: &[V]) -> Self {
        let mut set = DenseSet::new();
        for v in value {
            set.insert(v.clone());
        }

        set
    }
}
