use std::collections::HashMap;

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct SparseArray<V> {
    values: Vec<Option<V>>,
}

impl<V> SparseArray<V> {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, index: usize, value: V) {
        if index >= self.values.len() {
            self.values.resize_with(index + 1, || None);
        }
        self.values[index] = Some(value);
    }

    pub fn get(&self, index: usize) -> Option<&V> {
        self.values.get(index).map(|value| value.as_ref().unwrap())
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        self.values
            .get_mut(index)
            .map(|value| value.as_mut().unwrap())
    }

    pub fn remove(&mut self, index: usize) -> Option<V> {
        self.values
            .get_mut(index)
            .map(|value| value.take().unwrap())
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.values.iter().filter_map(|value| value.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.values.iter_mut().filter_map(|value| value.as_mut())
    }

    pub fn contains(&self, index: usize) -> bool {
        self.values
            .get(index)
            .map(|value| value.is_some())
            .unwrap_or(false)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn into_immutable(self) -> ImmutableSparseArray<V> {
        ImmutableSparseArray {
            values: self.values.into_boxed_slice(),
        }
    }
}

pub struct SparseSet<V> {
    values: Vec<V>,
    indices: Vec<usize>,
    array: SparseArray<usize>,
}

impl<V> SparseSet<V> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            indices: Vec::new(),
            array: SparseArray::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            indices: Vec::with_capacity(capacity),
            array: SparseArray::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, index: usize, value: V) -> Option<V> {
        if let Some(mapped_index) = self.array.get(index) {
            let old = std::mem::replace(&mut self.values[*mapped_index], value);

            return Some(old);
        } else {
            let mapped_index = self.values.len();
            self.values.push(value);
            self.indices.push(index);
            self.array.insert(index, mapped_index);

            return None;
        }
    }

    pub fn get(&self, index: usize) -> Option<&V> {
        self.array
            .get(index)
            .map(|mapped_index| &self.values[*mapped_index])
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        self.array
            .get(index)
            .map(|mapped_index| &mut self.values[*mapped_index])
    }

    pub fn remove(&mut self, index: usize) -> Option<V> {
        if let Some(mapped_index) = self.array.remove(index) {
            let value = self.values.swap_remove(mapped_index);
            let index = self.indices.swap_remove(mapped_index);
            self.array.insert(index, mapped_index);
            Some(value)
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.values.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.values.iter_mut()
    }

    pub fn values(&self) -> &[V] {
        &self.values
    }

    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.indices.iter().cloned()
    }

    pub fn contains(&self, index: usize) -> bool {
        self.array.contains(index)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.indices.clear();
        self.array = SparseArray::new();
    }

    pub fn into_immutable(self) -> ImmutableSparseSet<V> {
        ImmutableSparseSet {
            values: self.values.into_boxed_slice(),
            indices: self.indices.into_boxed_slice(),
            array: self.array.into_immutable(),
        }
    }
}

pub struct SparseMap<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    values: Vec<V>,
    keys: Vec<K>,
    map: HashMap<K, usize>,
}

impl<K, V> SparseMap<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            values: Vec::new(),
            map: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            keys: Vec::with_capacity(capacity),
            values: Vec::with_capacity(capacity),
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(index) = self.map.get(&key) {
            let old = std::mem::replace(&mut self.values[*index], value);

            return Some(old);
        } else {
            let index = self.values.len();
            self.keys.push(key);
            self.values.push(value);
            self.map.insert(self.keys[index].clone(), index);

            return None;
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key).map(|index| &self.values[*index])
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get(key).map(|index| &mut self.values[*index])
    }

    pub fn get_mut_slice<'a, F: FnMut((usize, &mut V)) -> Option<&mut V> + 'a>(
        &'a mut self,
        filter: F,
    ) -> impl Iterator<Item = &mut V> + 'a {
        self.values.iter_mut().enumerate().filter_map(filter)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(index) = self.map.remove(key) {
            let value = self.values.swap_remove(index);
            self.keys.swap_remove(index);
            if self.keys.len() > index {
                self.map.insert(self.keys[index].clone(), index);
            }
            Some(value)
        } else {
            None
        }
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (K, V)> + '_ {
        self.keys
            .drain(..)
            .zip(self.values.drain(..))
            .map(|(key, value)| (key, value))
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

    pub fn values(&self) -> &[V] {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut [V] {
        &mut self.values
    }

    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.values.clear();
        self.map.clear();
    }

    pub fn append(&mut self, other: &mut Self) {
        for key in other.keys.drain(..) {
            let value = other.map.remove(&key).unwrap();
            self.keys.push(key);
            self.values.push(other.values.swap_remove(value));
            self.map
                .insert(self.keys.last().unwrap().clone(), self.values.len() - 1);
        }

        other.clear();
    }

    pub fn sort(&mut self, sorter: fn(&V, &V) -> std::cmp::Ordering) {
        self.keys.sort_by(|a, b| {
            let value_a = &self.values[*self.map.get(a).unwrap()];
            let value_b = &self.values[*self.map.get(b).unwrap()];
            sorter(value_a, value_b)
        });

        self.map.clear();

        for (index, key) in self.keys.iter().enumerate() {
            self.map.insert(key.clone(), index);
        }

        self.values.sort_by(sorter);
    }

    pub fn into_immutable(self) -> ImmutableSparseMap<K, V> {
        ImmutableSparseMap {
            keys: self.keys.into_boxed_slice(),
            values: self.values.into_boxed_slice(),
            map: self.map,
        }
    }

    pub fn filter(&self, mut f: impl FnMut(&K, &V) -> bool) -> ImmutableSparseMap<K, &V> {
        let mut map = SparseMap::<K, &V>::new();

        for (key, value) in self.iter() {
            if f(key, value) {
                map.insert(key.clone(), value);
            }
        }

        map.into_immutable()
    }

    pub fn filter_mut(
        &mut self,
        mut f: impl FnMut(&K, &mut V) -> bool,
    ) -> ImmutableSparseMap<K, &mut V> {
        let mut map = SparseMap::<K, &mut V>::new();

        for (key, value) in self.iter_mut() {
            if f(key, value) {
                map.insert(key.clone(), value);
            }
        }

        map.into_immutable()
    }
}

impl<K, V> Default for SparseMap<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct ImmutableSparseArray<V> {
    values: Box<[Option<V>]>,
}

impl<V> ImmutableSparseArray<V> {
    pub fn get(&self, index: usize) -> Option<&V> {
        self.values.get(index).map(|v| v.as_ref()).unwrap_or(None)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Option<V>> {
        self.values.iter()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

pub struct ImmutableSparseSet<V> {
    values: Box<[V]>,
    indices: Box<[usize]>,
    array: ImmutableSparseArray<usize>,
}

impl<V> ImmutableSparseSet<V> {
    pub fn get(&self, index: usize) -> Option<&V> {
        self.array
            .get(index)
            .map(|mapped_index| &self.values[*mapped_index])
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        self.array
            .get(index)
            .map(|mapped_index| &mut self.values[*mapped_index])
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.values.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.values.iter_mut()
    }

    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.indices.iter().cloned()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

pub struct ImmutableSparseMap<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    keys: Box<[K]>,
    values: Box<[V]>,
    map: HashMap<K, usize>,
}

impl<K, V> ImmutableSparseMap<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key).map(|index| &self.values[*index])
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get(key).map(|index| &mut self.values[*index])
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

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
