use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt::Debug,
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

    pub fn get_at(&self, index: usize) -> Option<&V> {
        self.values.get(index)
    }

    pub fn get_at_mut(&mut self, index: usize) -> Option<&mut V> {
        self.values.get_mut(index)
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

    pub fn insert_before(&mut self, key: K, value: V, before: K) -> Option<V> {
        if self.contains(&key) {
            self.remove(&key);
        }
        let hasher = hash(&before);
        if self.map.contains_key(&hasher) {
            let index = *self.map.get(&hasher).unwrap();
            self.map.insert(hash(&key), index);
            self.keys.insert(index, key);
            self.values.insert(index, value);
            self.map.insert(hasher, index + 1);
            None
        } else {
            self.insert(key, value)
        }
    }

    pub fn insert_after(&mut self, key: K, value: V, after: K) -> Option<V> {
        let removed = if self.contains(&key) {
            self.remove(&key)
        } else {
            None
        };

        let hasher = hash(&after);
        if self.map.contains_key(&hasher) {
            let index = *self.map.get(&hasher).unwrap();
            self.map.insert(hash(&key), index + 1);
            if index + 1 < self.keys.len() {
                self.keys.insert(index + 1, key);
                self.values.insert(index + 1, value);
            } else {
                self.keys.push(key);
                self.values.push(value);
            }
            if index + 2 < self.len() {
                self.map.insert(hash(&self.keys[index + 2]), index + 2);
            }
        } else {
            return self.insert(key, value);
        }

        removed
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

    pub fn remove_at(&mut self, index: usize) -> Option<(K, V)> {
        if index >= self.len() {
            return None;
        }
        let key = self.keys.swap_remove(index);
        let value = self.values.swap_remove(index);
        let hashed = hash(&key);
        self.map.remove(&hashed);

        if index < self.len() {
            let hashed = hash(&self.keys[index]);
            self.map.insert(hashed, index);
        }

        Some((key, value))
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

    pub fn sort(&mut self, f: impl Fn((&K, &V), (&K, &V)) -> std::cmp::Ordering) {
        self.keys.sort_by(|a, b| {
            let key_a = hash(a);
            let key_b = hash(b);
            let value_a = &self.values[*self.map.get(&key_a).unwrap()];
            let value_b = &self.values[*self.map.get(&key_b).unwrap()];
            f((a, value_a), (b, value_b))
        });

        self.map.clear();

        let mut tracker = HashSet::new();
        for (index, key) in self.keys.iter().enumerate() {
            let key = hash(key);
            if let Some(prev) = self.map.insert(key, index) {
                if !tracker.contains(&prev) {
                    self.values.swap(prev, index);
                    tracker.insert(prev);
                    tracker.insert(index);
                }
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

    pub fn to_immutable(self) -> ImmutableDenseMap<K, V> {
        ImmutableDenseMap::new(self)
    }
}

impl<K: Hash + PartialEq + PartialOrd, V> Default for DenseMap<K, V> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            keys: Default::default(),
            values: Default::default(),
        }
    }
}

impl<K: Hash + PartialEq + PartialOrd + Debug, V: Debug> Debug for DenseMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K: Hash + PartialEq + PartialOrd, V> FromIterator<(K, V)> for DenseMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = DenseMap::new();
        for (key, value) in iter {
            map.insert(key, value);
        }

        map
    }
}

impl<K: Clone + Hash + PartialEq + PartialOrd, V: Clone> Clone for DenseMap<K, V> {
    fn clone(&self) -> Self {
        let mut map = DenseMap::new();
        for (key, value) in self.iter() {
            map.insert(key.clone(), value.clone());
        }

        map
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

    pub fn insert_before(&mut self, value: V, before: V) -> bool {
        let hasher = hash(&before);
        if self.map.contains_key(&hasher) {
            let index = *self.map.get(&hasher).unwrap();
            self.values.insert(index, value);
            for (i, value) in self.values.iter().enumerate() {
                let hashed = hash(value);
                self.map.insert(hashed, i);
            }
            true
        } else {
            false
        }
    }

    pub fn insert_after(&mut self, value: V, after: V) -> bool {
        let hasher = hash(&after);
        if self.map.contains_key(&hasher) {
            let index = *self.map.get(&hasher).unwrap();
            self.values.insert(index + 1, value);
            for (i, value) in self.values.iter().enumerate() {
                let hashed = hash(value);
                self.map.insert(hashed, i);
            }
            true
        } else {
            false
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

    pub fn remove(&mut self, value: &V) -> Option<V> {
        let hashed = hash(value);

        if let Some(index) = self.map.remove(&hashed) {
            let value = self.values.swap_remove(index);
            if index < self.len() {
                let hashed = hash(&self.values[index]);
                self.map.insert(hashed, index);
            }

            Some(value)
        } else {
            None
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

    pub fn sort(&mut self) {
        self.values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        self.map.clear();
        for (index, value) in self.values.iter().enumerate() {
            let hashed = hash(value);
            self.map.insert(hashed, index);
        }
    }

    pub fn sort_by(&mut self, f: impl Fn(&V, &V) -> std::cmp::Ordering) {
        self.values.sort_by(|a, b| f(a, b));

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

    pub fn drain(&mut self) -> impl Iterator<Item = V> + '_ {
        self.map.clear();
        self.values.drain(..)
    }

    pub fn destruct(self) -> (Vec<V>, HashMap<u64, usize>) {
        (self.values, self.map)
    }

    pub fn to_immutable(self) -> ImmutableDenseSet<V> {
        ImmutableDenseSet::new(self)
    }
}

impl<V: Hash + PartialEq + PartialOrd> Default for DenseSet<V> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            values: Default::default(),
        }
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

impl<V: Clone + Hash + PartialEq + PartialOrd> Clone for DenseSet<V> {
    fn clone(&self) -> Self {
        let mut set = DenseSet::new();
        for v in self.iter() {
            set.insert(v.clone());
        }

        set
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

impl<V: Clone + Hash + PartialEq + PartialOrd> From<Vec<V>> for DenseSet<V> {
    fn from(value: Vec<V>) -> Self {
        let mut set = DenseSet::new();
        for v in value {
            set.insert(v.clone());
        }

        set
    }
}

impl<V: Clone + Hash + PartialEq + PartialOrd> From<&Vec<V>> for DenseSet<V> {
    fn from(value: &Vec<V>) -> Self {
        let mut set = DenseSet::new();
        for v in value {
            set.insert(v.clone());
        }

        set
    }
}

impl<V: Hash + PartialEq + PartialOrd> FromIterator<V> for DenseSet<V> {
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        let mut set = DenseSet::new();
        for v in iter {
            set.insert(v);
        }

        set
    }
}

impl<K: Hash + PartialEq + PartialOrd + Debug> Debug for DenseSet<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

pub struct ImmutableDenseSet<V: Hash + PartialEq + PartialOrd> {
    set: DenseSet<V>,
}

impl<V: Hash + PartialEq + PartialOrd> ImmutableDenseSet<V> {
    pub fn new(set: DenseSet<V>) -> Self {
        ImmutableDenseSet { set }
    }

    pub fn contains(&self, value: &V) -> bool {
        self.set.contains(value)
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.set.iter()
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    pub fn values(&self) -> &[V] {
        self.set.values()
    }
}

impl<V: Clone + Hash + PartialEq + PartialOrd> Clone for ImmutableDenseSet<V> {
    fn clone(&self) -> Self {
        ImmutableDenseSet::new(self.set.clone())
    }
}

impl<V: Clone + Hash + PartialEq + PartialOrd> From<&DenseSet<V>> for ImmutableDenseSet<V> {
    fn from(set: &DenseSet<V>) -> Self {
        ImmutableDenseSet::new(set.clone())
    }
}

impl<V: Hash + PartialEq + PartialOrd> From<DenseSet<V>> for ImmutableDenseSet<V> {
    fn from(set: DenseSet<V>) -> Self {
        ImmutableDenseSet::new(set)
    }
}

pub struct ImmutableDenseMap<K: Hash + PartialEq + PartialOrd, V> {
    map: DenseMap<K, V>,
}

impl<K: Hash + PartialEq + PartialOrd, V> ImmutableDenseMap<K, V> {
    pub fn new(map: DenseMap<K, V>) -> Self {
        ImmutableDenseMap { map }
    }

    pub fn contains(&self, key: &K) -> bool {
        self.map.contains(key)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(key)
    }

    pub fn get_at(&self, index: usize) -> Option<&V> {
        self.map.get_at(index)
    }

    pub fn get_at_mut(&mut self, index: usize) -> Option<&mut V> {
        self.map.get_at_mut(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.map.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.map.iter_mut()
    }

    pub fn keys(&self) -> &[K] {
        self.map.keys()
    }

    pub fn values(&self) -> &[V] {
        self.map.values()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

impl<K: Clone + Hash + PartialEq + PartialOrd, V: Clone> Clone for ImmutableDenseMap<K, V> {
    fn clone(&self) -> Self {
        ImmutableDenseMap::new(self.map.clone())
    }
}

impl<K: Clone + Hash + PartialEq + PartialOrd, V: Clone> From<&DenseMap<K, V>>
    for ImmutableDenseMap<K, V>
{
    fn from(map: &DenseMap<K, V>) -> Self {
        ImmutableDenseMap::new(map.clone())
    }
}

impl<K: Hash + PartialEq + PartialOrd, V> From<DenseMap<K, V>> for ImmutableDenseMap<K, V> {
    fn from(map: DenseMap<K, V>) -> Self {
        ImmutableDenseMap::new(map)
    }
}
