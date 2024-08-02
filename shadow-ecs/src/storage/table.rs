use crate::core::internal::blob::{Blob, BlobCell};
use std::collections::hash_map::DefaultHasher;
use std::{
    any::TypeId,
    collections::HashMap,
    hash::{Hash, Hasher},
};

use super::dense::map::DenseMap;
use super::dense::set::DenseSet;

pub struct ColumnCell {
    data: BlobCell,
}

impl ColumnCell {
    pub fn from<T: 'static>(value: T) -> Self {
        let data = BlobCell::new::<T>(value);

        Self { data }
    }

    pub fn value<T: 'static>(&self) -> &T {
        self.data.value()
    }

    pub fn value_mut<T: 'static>(&mut self) -> &mut T {
        self.data.value_mut()
    }

    pub fn take<T: 'static>(self) -> T {
        self.data.take()
    }
}

pub struct SelectedCell<'a> {
    column: &'a Column,
    index: usize,
}

impl<'a> SelectedCell<'a> {
    fn new(column: &'a Column, index: usize) -> Self {
        Self { column, index }
    }

    pub fn value<T: 'static>(&self) -> Option<&T> {
        self.column.get::<T>(self.index)
    }

    pub fn value_mut<T: 'static>(&self) -> Option<&mut T> {
        self.column.get_mut::<T>(self.index)
    }
}

pub struct Column {
    data: Blob,
}

impl Column {
    pub fn new<T: 'static>() -> Self {
        Self {
            data: Blob::new::<T>(0),
        }
    }

    pub fn copy(column: &Column) -> Self {
        Column {
            data: Blob::with_layout(column.data.layout().clone(), 0, column.data.drop().copied()),
        }
    }

    pub fn copy_cell(cell: &ColumnCell) -> Self {
        Column {
            data: Blob::with_layout(cell.data.layout().clone(), 0, cell.data.drop().copied()),
        }
    }

    pub fn get<T: 'static>(&self, index: usize) -> Option<&T> {
        self.data.get::<T>(index)
    }

    pub fn get_mut<T: 'static>(&self, index: usize) -> Option<&mut T> {
        self.data.get_mut::<T>(index)
    }

    pub fn push<T: 'static>(&mut self, value: T) {
        self.data.push(value)
    }

    pub fn insert<T: 'static>(&mut self, index: usize, value: T) {
        self.data.insert(index, value)
    }

    pub fn extend(&mut self, column: Column) {
        self.data.extend(column.data)
    }

    pub fn remove<T: 'static>(&mut self, index: usize) -> T {
        self.data.remove(index)
    }

    pub fn swap_remove<T: 'static>(&mut self, index: usize) -> T {
        self.data.swap_remove(index)
    }

    pub fn select(&self, index: usize) -> Option<SelectedCell> {
        if index >= self.len() {
            None
        } else {
            Some(SelectedCell::new(self, index))
        }
    }

    pub fn push_cell(&mut self, cell: ColumnCell) {
        self.data.extend(cell.data.into())
    }

    pub fn insert_cell(&mut self, index: usize, cell: ColumnCell) {
        self.data.insert_blob(index, cell.data.into())
    }

    pub fn remove_cell(&mut self, index: usize) -> ColumnCell {
        let data = self.data.remove_blob(index).into();
        ColumnCell { data }
    }

    pub fn swap_remoe_cell(&mut self, index: usize) -> ColumnCell {
        let data = self.data.swap_remove_blob(index).into();
        ColumnCell { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    pub fn clear(&mut self) {
        self.data.clear()
    }
}

impl From<ColumnCell> for Column {
    fn from(cell: ColumnCell) -> Self {
        Column {
            data: cell.data.into(),
        }
    }
}

impl From<&ColumnCell> for Column {
    fn from(cell: &ColumnCell) -> Self {
        Column {
            data: Blob::with_layout(cell.data.layout().clone(), 1, cell.data.drop().copied()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnKey(u64);

impl ColumnKey {
    pub fn from<K: 'static>() -> Self {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<K>().hash(&mut hasher);

        ColumnKey(hasher.finish())
    }

    pub fn raw(id: u64) -> Self {
        ColumnKey(id)
    }
}

impl std::ops::Deref for ColumnKey {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Row {
    columns: DenseMap<ColumnKey, ColumnCell>,
}

impl Row {
    pub fn new() -> Self {
        Self {
            columns: DenseMap::new(),
        }
    }

    pub fn columns(&self) -> &[ColumnKey] {
        self.columns.keys()
    }

    pub fn add_field<C: 'static>(&mut self, value: C) -> Option<ColumnCell> {
        let key = ColumnKey::from::<C>();
        self.columns.insert(key, ColumnCell::from(value))
    }

    pub fn add_cell(&mut self, key: impl Into<ColumnKey>, cell: ColumnCell) -> Option<ColumnCell> {
        self.columns.insert(key.into(), cell.into())
    }

    pub fn remove_field<C: 'static>(&mut self) -> Option<C> {
        let key = ColumnKey::from::<C>();
        self.columns.remove(&key)?.take()
    }

    pub fn remove_cell(&mut self, key: impl Into<ColumnKey>) -> Option<ColumnCell> {
        self.columns.remove(&key.into())
    }

    pub fn field<C: 'static>(&self) -> Option<&C> {
        let key = ColumnKey::from::<C>();
        Some(self.columns.get(&key)?.value::<C>())
    }

    pub fn field_mut<C: 'static>(&mut self) -> Option<&mut C> {
        let key = ColumnKey::from::<C>();
        Some(self.columns.get_mut(&key)?.value_mut::<C>())
    }

    pub fn cell(&self, key: &ColumnKey) -> Option<&ColumnCell> {
        self.columns.get(key)
    }

    pub fn cell_mut(&mut self, key: &ColumnKey) -> Option<&mut ColumnCell> {
        self.columns.get_mut(key)
    }

    pub fn fields(&self) -> &[ColumnKey] {
        self.columns.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ColumnKey, &ColumnCell)> {
        self.columns.iter()
    }

    pub fn contains(&self, key: &ColumnKey) -> bool {
        self.columns.contains(key)
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (ColumnKey, ColumnCell)> + '_ {
        self.columns.drain()
    }

    pub fn sort(&mut self) {
        self.columns.sort(|a, b| a.0.cmp(&b.0));
    }

    pub fn table_layout<R: RowIndex>(&self) -> TableLayout<R> {
        let mut layout = TableLayout::new();
        for (key, cell) in self.columns.iter() {
            layout.add_column(*key, Column::from(cell));
        }

        layout
    }
}

impl IntoIterator for Row {
    type Item = (ColumnKey, ColumnCell);
    type IntoIter = std::iter::Zip<std::vec::IntoIter<ColumnKey>, std::vec::IntoIter<ColumnCell>>;

    fn into_iter(self) -> Self::IntoIter {
        self.columns.into_iter()
    }
}

impl<'a> IntoIterator for &'a Row {
    type Item = (&'a ColumnKey, &'a ColumnCell);
    type IntoIter =
        std::iter::Zip<std::slice::Iter<'a, ColumnKey>, std::slice::Iter<'a, ColumnCell>>;

    fn into_iter(self) -> Self::IntoIter {
        self.columns.iter()
    }
}

pub struct SelectedRow<'a> {
    columns: HashMap<ColumnKey, &'a Column>,
    index: usize,
}

impl<'a> SelectedRow<'a> {
    pub fn new(columns: HashMap<ColumnKey, &'a Column>, index: usize) -> Self {
        Self { columns, index }
    }

    pub fn field<C: 'static>(&self) -> Option<&C> {
        let key = ColumnKey::from::<C>();
        self.columns.get(&key)?.get::<C>(self.index)
    }

    pub fn field_mut<C: 'static>(&self) -> Option<&mut C> {
        let key = ColumnKey::from::<C>();
        self.columns.get(&key)?.get_mut::<C>(self.index)
    }

    pub fn cell(&self, key: &ColumnKey) -> Option<SelectedCell> {
        self.columns.get(key)?.select(self.index)
    }

    pub fn cell_mut(&self, key: &ColumnKey) -> Option<SelectedCell> {
        self.columns.get(key)?.select(self.index)
    }

    pub fn fields(&self) -> std::collections::hash_map::Keys<ColumnKey, &'a Column> {
        self.columns.keys()
    }
}

pub trait RowIndex: Hash + Eq + PartialEq + Copy {
    fn index(&self) -> usize;
    fn gen(&self) -> usize;
}

pub struct TableLayout<R: RowIndex> {
    columns: HashMap<ColumnKey, Column>,
    _row: std::marker::PhantomData<R>,
}

impl<R: RowIndex> TableLayout<R> {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
            _row: std::marker::PhantomData,
        }
    }

    pub fn add_field<C: 'static>(&mut self) -> &mut Self {
        let key = ColumnKey::from::<C>();
        self.columns.insert(key, Column::new::<C>());
        self
    }

    pub fn with_field<C: 'static>(mut self) -> Self {
        let key = ColumnKey::from::<C>();
        self.columns.insert(key, Column::new::<C>());
        self
    }

    pub fn add_column(&mut self, key: ColumnKey, column: Column) -> &mut Self {
        self.columns.insert(key, column);
        self
    }

    pub fn with_column(mut self, key: ColumnKey, column: Column) -> Self {
        self.columns.insert(key, column);
        self
    }

    pub fn build(self) -> Table<R> {
        Table {
            columns: self.columns,
            rows: DenseSet::new(),
        }
    }
}

pub struct Table<R: RowIndex> {
    columns: HashMap<ColumnKey, Column>,
    rows: DenseSet<R>,
}

impl<R: RowIndex> Table<R> {
    pub fn builder() -> TableLayout<R> {
        TableLayout::new()
    }

    pub fn rows(&self) -> &[R] {
        self.rows.keys()
    }

    pub fn columns(&self) -> std::collections::hash_map::Keys<ColumnKey, Column> {
        self.columns.keys()
    }

    pub fn field<C: 'static>(&self, index: impl Into<R>) -> Option<&C> {
        let key = ColumnKey::from::<C>();
        let index = index.into();
        let index = self.rows.index_of(&index)?;
        self.columns.get(&key)?.get::<C>(index)
    }

    pub fn field_mut<C: 'static>(&self, index: impl Into<R>) -> Option<&mut C> {
        let key = ColumnKey::from::<C>();
        let index = index.into();
        let index = self.rows.index_of(&index)?;
        self.columns.get(&key)?.get_mut::<C>(index)
    }

    pub fn cell(&self, key: &ColumnKey, index: impl Into<R>) -> Option<SelectedCell> {
        let index = index.into();
        let index = self.rows.index_of(&index)?;
        self.columns.get(key)?.select(index)
    }

    pub fn cell_mut(&self, key: &ColumnKey, index: impl Into<R>) -> Option<SelectedCell> {
        let index = index.into();
        let index = self.rows.index_of(&index)?;
        self.columns.get(key)?.select(index)
    }

    pub fn select(&self, index: impl Into<R>) -> Option<SelectedRow> {
        let index = index.into();
        let index = self.rows.index_of(&index)?;
        let mut columns = HashMap::new();
        for (field, column) in &self.columns {
            columns.insert(field.clone(), column);
        }

        Some(SelectedRow::new(columns, index))
    }

    pub fn insert(&mut self, index: impl Into<R>, mut row: Row) {
        self.rows.insert(index.into());
        for (field, column) in &mut self.columns {
            let cell = row.remove_cell(*field).unwrap();
            column.push_cell(cell);
        }
    }

    pub fn remove(&mut self, index: impl Into<R>) -> Option<Row> {
        let index = index.into();
        let idx = self.rows.index_of(&index)?;
        Some(self.rows.swap_remove_at(idx));
        let mut row = Row::new();
        for (field, column) in &mut self.columns {
            let cell = column.swap_remoe_cell(idx);
            row.add_cell(field.clone(), cell);
        }

        Some(row)
    }

    pub fn contains(&self, index: impl Into<R>) -> bool {
        self.rows.contains(&index.into())
    }

    pub fn clear(&mut self) {
        self.rows.clear();
        for column in self.columns.values_mut() {
            column.clear();
        }
    }
}
