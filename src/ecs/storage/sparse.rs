pub struct SparseArray<V> {
    values: Vec<Option<V>>,
}

impl<V> SparseArray<V> {
    pub fn new() -> Self {
        SparseArray { values: vec![] }
    }

    pub fn contains(&self, index: usize) -> bool {
        self.values.get(index).is_some()
    }

    pub fn get(&self, index: usize) -> Option<&V> {
        self.values.get(index).and_then(|v| v.as_ref())
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        self.values.get_mut(index).and_then(|v| v.as_mut())
    }

    pub fn insert(&mut self, index: usize, value: V) -> Option<V> {
        if index >= self.values.len() {
            self.values.resize_with(index + 1, || None);
        }

        std::mem::replace(&mut self.values[index], Some(value))
    }

    pub fn remove(&mut self, index: usize) -> Option<V> {
        self.values.get_mut(index).and_then(|v| v.take())
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.values.iter().filter_map(|v| v.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.values.iter_mut().filter_map(|v| v.as_mut())
    }

    pub fn len(&self) -> usize {
        self.values.iter().filter(|v| v.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }
}

impl<V> Default for SparseArray<V> {
    fn default() -> Self {
        Self::new()
    }
}
