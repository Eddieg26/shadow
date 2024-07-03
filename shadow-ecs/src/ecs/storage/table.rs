use crate::ecs::{
    core::internal::blob::Blob,
    storage::dense::{DenseMap, DenseSet, ImmutableDenseMap},
};
use std::hash::Hash;

pub struct TableCell<'a, C> {
    column: &'a Column,
    index: usize,
    _marker: std::marker::PhantomData<C>,
}

impl<'a, C> TableCell<'a, C> {
    fn new(column: &'a Column, index: usize) -> Self {
        if index >= column.data.len() {
            panic!("Index out of bounds");
        }

        TableCell {
            column,
            index,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn cast(&self) -> &C {
        self.column.data.get::<C>(self.index).unwrap()
    }

    pub fn cast_mut(&mut self) -> &mut C {
        self.column.data.get_mut::<C>(self.index).unwrap()
    }
}

impl<'a, C> std::ops::Deref for TableCell<'a, C> {
    type Target = Column;

    fn deref(&self) -> &Self::Target {
        self.column
    }
}

pub struct Column {
    data: Blob,
}

impl Column {
    pub fn new<C>() -> Self {
        Column {
            data: Blob::new::<C>(),
        }
    }

    pub fn with<C>(mut self, value: C) -> Self {
        self.push(value);
        self
    }

    pub fn from_column(column: &Column) -> Self {
        Column {
            data: column.data.copy(1),
        }
    }

    pub fn from_blob(data: Blob) -> Self {
        Column { data }
    }

    pub fn get<C>(&self, index: usize) -> Option<&C> {
        self.data.get::<C>(index)
    }

    pub fn get_mut<C>(&self, index: usize) -> Option<&mut C> {
        self.data.get_mut::<C>(index)
    }

    pub fn cell<'a, C>(&'a self, index: usize) -> TableCell<'a, C> {
        TableCell::new(self, index)
    }

    pub fn append(&mut self, mut value: Blob) {
        self.data.append(&mut value);
    }

    pub fn push<C>(&mut self, value: C) {
        self.data.push(value);
    }

    pub fn remove(&mut self, index: usize) -> Blob {
        self.data.swap_remove(index)
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

pub struct RowLayout<K> {
    columns: DenseMap<K, Column>,
}

impl<K: Hash + PartialEq + Ord> RowLayout<K> {
    pub fn new() -> Self {
        RowLayout {
            columns: DenseMap::new(),
        }
    }

    pub fn with_column(mut self, key: K, column: Column) -> Self {
        self.columns.insert(key, column);
        self
    }

    pub fn add_column(&mut self, key: K, column: Column) {
        self.columns.insert(key, column);
    }

    pub fn build(mut self) -> Row<K> {
        self.columns.sort(|a, b| a.0.cmp(&b.0));
        Row {
            columns: self.columns.to_immutable(),
        }
    }
}

pub struct Row<K> {
    columns: ImmutableDenseMap<K, Column>,
}

impl<K: Hash + Clone + PartialEq + Ord> Row<K> {
    pub fn new() -> RowLayout<K> {
        RowLayout::new()
    }

    pub fn column(&self, key: &K) -> Option<&Column> {
        self.columns.get(key)
    }

    pub fn column_mut(&mut self, key: &K) -> Option<&mut Column> {
        self.columns.get_mut(key)
    }

    pub fn select(&self, key: K) -> Option<TableCell<K>> {
        self.columns.get(&key).map(|column| column.cell(0))
    }

    pub fn set(&mut self, key: K, column: Column) {
        self.columns.get_mut(&key).map(|c| *c = column);
    }

    pub fn remove(&mut self, keys: impl IntoIterator<Item = K>) -> Row<K> {
        let keys = keys.into_iter().collect::<Vec<_>>();
        let mut layout = RowLayout::new();
        for key in keys {
            if let Some(column) = self.columns.get_mut(&key) {
                let data = column.data.swap_remove(0);
                layout.add_column(key, Column::from_blob(data));
            }
        }

        layout.build()
    }
}

pub struct FreeRow<K> {
    columns: DenseMap<K, Column>,
}

impl<K: Hash + PartialEq + Ord + Clone> FreeRow<K> {
    pub fn new() -> Self {
        Self {
            columns: DenseMap::new(),
        }
    }

    pub fn keys(&self) -> &[K] {
        self.columns.keys()
    }

    pub fn add_column(&mut self, key: K, column: Column) -> Option<Column> {
        self.columns.insert(key, column)
    }

    pub fn with<C>(mut self, key: K, value: C) -> Self {
        self.insert(key, value);
        self
    }

    pub fn insert<C>(&mut self, key: K, component: C) -> Option<Column> {
        let mut column = Column::new::<C>();
        column.push(component);
        self.columns.insert(key, column)
    }

    pub fn get(&self, key: &K) -> Option<&Column> {
        self.columns.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut Column> {
        self.columns.get_mut(key)
    }

    pub fn values(&self) -> &DenseMap<K, Column> {
        &self.columns
    }

    pub fn columns_mut(&mut self) -> &mut DenseMap<K, Column> {
        &mut self.columns
    }

    pub fn remove_at(&mut self, index: usize) -> Option<(K, Column)> {
        self.columns.remove_at(index)
    }

    pub fn remove(&mut self, key: &K) -> Option<Column> {
        self.columns.remove(key)
    }

    pub fn remove_component<C>(&mut self, key: &K) -> Option<C> {
        self.columns
            .get_mut(key)
            .and_then(|column| Some(column.remove(0)))
            .and_then(|mut blob| blob.remove::<C>(0))
    }

    pub fn sort(&mut self) {
        self.columns.sort(|a, b| a.0.cmp(&b.0));
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (K, Column)> + '_ {
        self.columns.drain()
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.len() == 0
    }

    pub fn has(&self, key: &K) -> bool {
        self.columns.contains(key)
    }

    pub fn layout(&self) -> TableLayout<K> {
        let mut layout = TableLayout::<K>::new();
        for (id, column) in self.columns.iter() {
            layout.add_column(id.clone(), Column::from_column(column));
        }

        layout
    }
}

impl<K: Hash + PartialEq + Ord + Clone> Into<Row<K>> for FreeRow<K> {
    fn into(mut self) -> Row<K> {
        let mut layout = RowLayout::<K>::new();
        for (id, column) in self.columns.drain() {
            layout.add_column(id, column);
        }

        layout.build()
    }
}

impl<K> Into<DenseMap<K, Column>> for FreeRow<K> {
    fn into(self) -> DenseMap<K, Column> {
        self.columns
    }
}

pub struct SelectedRow<'a, K> {
    columns: &'a ImmutableDenseMap<K, Column>,
    index: usize,
}

impl<'a, K: Hash + Clone + PartialEq + Ord> SelectedRow<'a, K> {
    pub fn get<C>(&self, key: K) -> TableCell<'a, C> {
        let column = self.columns.get(&key).expect("Column not found");
        column.cell::<C>(self.index)
    }
}

pub struct TableLayout<K> {
    columns: DenseMap<K, Column>,
}

impl<K: Hash + Clone + PartialEq + Ord> TableLayout<K> {
    pub fn new() -> Self {
        TableLayout {
            columns: DenseMap::new(),
        }
    }

    pub fn with_column(mut self, key: K, column: Column) -> Self {
        self.columns.insert(key, column);
        self
    }

    pub fn add_column(&mut self, key: K, column: Column) {
        self.columns.insert(key, column);
    }

    pub fn build<R: Hash + PartialEq>(mut self) -> Table<R, K> {
        self.columns.sort(|a, b| a.0.cmp(&b.0));
        Table {
            columns: self.columns.to_immutable(),
            rows: DenseSet::<R>::new(),
        }
    }
}

pub struct Table<R, K> {
    columns: ImmutableDenseMap<K, Column>,
    rows: DenseSet<R>,
}

impl<R: Hash + PartialEq, K: Hash + Clone + PartialEq + Ord> Table<R, K> {
    pub fn new() -> TableLayout<K> {
        TableLayout::new()
    }

    pub fn columns(&self) -> &[K] {
        self.columns.keys()
    }

    pub fn rows(&self) -> &[R] {
        self.rows.values()
    }

    pub fn column(&self, key: K) -> Option<&Column> {
        self.columns.get(&key)
    }

    pub fn column_mut(&mut self, key: K) -> Option<&mut Column> {
        self.columns.get_mut(&key)
    }

    pub fn select(&self, row: &R) -> Option<SelectedRow<K>> {
        self.rows.index(row).map(|index| SelectedRow {
            columns: &self.columns,
            index,
        })
    }

    pub fn item<C>(&self, row: &R, key: K) -> Option<&C> {
        self.rows.index(row).and_then(|index| {
            self.columns
                .get(&key)
                .and_then(|column| column.data.get::<C>(index))
        })
    }

    pub fn item_mut<C>(&self, row: &R, key: K) -> Option<&mut C> {
        self.rows.index(row).and_then(|index| {
            self.columns
                .get(&key)
                .and_then(|column| column.data.get_mut::<C>(index))
        })
    }

    pub fn insert(&mut self, index: R, mut row: Row<K>) {
        self.rows.insert(index);
        for (key, old_column) in row.columns.iter_mut() {
            let column = self.columns.get_mut(key).expect("Column not found");
            column.append(old_column.remove(0));
        }
    }

    pub fn remove(&mut self, row: &R) -> Option<FreeRow<K>> {
        self.rows.index(row).map(|index| {
            self.rows.swap_remove(row);
            let mut row = FreeRow::<K>::new();
            for (key, column) in self.columns.iter_mut() {
                let data = column.remove(index);
                row.add_column(key.clone(), Column::from_blob(data));
            }

            row
        })
    }

    pub fn contains(&self, row: &R) -> bool {
        self.rows.contains(row)
    }
}
